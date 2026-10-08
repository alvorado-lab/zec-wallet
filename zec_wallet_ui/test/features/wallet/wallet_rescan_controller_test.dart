import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_sync_controller.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// A controllable [syncStatusProvider] stub — skips the real stream/lifecycle
/// machinery so a test can drive the rescan controller's reached-tip clear by
/// emitting a status directly. Subclasses [SyncStatusNotifier] so it matches the
/// provider's notifier type for `overrideWith`.
class _StubSyncNotifier extends SyncStatusNotifier {
  @override
  AsyncValue<SyncStatus> build() => const AsyncValue<SyncStatus>.loading();

  void emit(SyncStatus status) => state = AsyncValue<SyncStatus>.data(status);

  /// The value-less loading the REAL syncStatusProvider returns on every build()
  /// re-run — including the session swap a rescan performs. The rebuilding cue
  /// MUST survive this (it must clear only on a real reached-tip).
  void emitLoading() => state = const AsyncValue<SyncStatus>.loading();
}

/// The rescan-recovery presentation controller (FR-1b). Drives the action and the
/// "rebuilding" cue against host-VM fakes — no native lib, no device. The session
/// swap + outcome live in [OnboardingController] (tested separately); here we pin
/// the PRESENTATION: which [WalletRescanState] each outcome yields, and that the
/// rebuilding cue is cleared exactly on reached-tip (not on any other status).
void main() {
  // #405 keyed the commit-point fence on `walletSyncPassesRunProvider`, which
  // watches the sync DRIVE — and building that controller reaches
  // `appLifecycleProvider`'s `AppLifecycleListener`, which needs a binding.
  // These are pure-VM container tests, so initialise one explicitly.
  TestWidgetsFlutterBinding.ensureInitialized();

  late _StubSyncNotifier sync;

  ProviderContainer harness(
    FakeWalletProvisioner provisioner, {
    List<Override> extra = const [],
  }) {
    sync = _StubSyncNotifier();
    final container = ProviderContainer(
      overrides: [
        walletProvisionerProvider.overrideWithValue(provisioner),
        onboardingStoreProvider.overrideWithValue(
          FakeOnboardingStore(confirmed: true),
        ),
        // One shared instance so the test can `emit` onto the SAME notifier the
        // rescan controller listens to.
        syncStatusProvider.overrideWith(() => sync),
        ...extra,
      ],
    );
    addTearDown(container.dispose);
    container.listen(onboardingControllerProvider, (_, _) {});
    container.listen(walletSessionProvider, (_, _) {});
    container.listen(walletRescanControllerProvider, (_, _) {});
    return container;
  }

  /// Reach Active (a confirmed wallet booted), the only state a rescan runs from.
  Future<ProviderContainer> atActive(
    FakeWalletProvisioner p, {
    List<Override> extra = const [],
  }) async {
    final c = harness(p, extra: extra);
    await pumpEventQueue();
    expect(
      c.read(onboardingControllerProvider),
      isA<OnboardingActive>(),
      reason: 'precondition',
    );
    return c;
  }

  WalletRescanState rescanOf(ProviderContainer c) =>
      c.read(walletRescanControllerProvider);
  WalletRescanController notifierOf(ProviderContainer c) =>
      c.read(walletRescanControllerProvider.notifier);

  test('the default state is idle', () async {
    final c = await atActive(FakeWalletProvisioner(exists: true));
    expect(rescanOf(c), isA<WalletRescanIdle>());
  });

  test(
    'a SUCCESSFUL rescan enters Rebuilding carrying the chosen date',
    () async {
      final c = await atActive(FakeWalletProvisioner(exists: true));
      final date = DateTime(2022, 6);

      await notifierOf(c).rescan(RescanFromTime(date));

      final state = rescanOf(c);
      expect(state, isA<WalletRescanRebuilding>());
      final target = (state as WalletRescanRebuilding).target;
      expect((target as RescanFromTime).earliestTime, date);
    },
  );

  test('a SCAN-ALL rescan enters Rebuilding carrying the all-history '
      'intent', () async {
    final c = await atActive(FakeWalletProvisioner(exists: true));

    await notifierOf(c).rescan(const RescanAllHistory());

    final state = rescanOf(c);
    expect(state, isA<WalletRescanRebuilding>());
    expect((state as WalletRescanRebuilding).target, isA<RescanAllHistory>());
  });

  test('the wallet-birthday DEFAULT enters Rebuilding carrying its floor '
      '(#317 -- the banner names what the user chose)', () async {
    final p = FakeWalletProvisioner(exists: true);
    final c = await atActive(p);

    await notifierOf(c).rescan(const RescanFromWalletBirthday(2400000));

    final state = rescanOf(c);
    expect(state, isA<WalletRescanRebuilding>());
    final target = (state as WalletRescanRebuilding).target;
    expect((target as RescanFromWalletBirthday).floorHeight, 2400000);
    // The adapter seam saw the SAME sealed arm (not a date re-conversion).
    expect(p.lastRescanTarget, isA<RescanFromWalletBirthday>());
  });

  test('reaching the tip (UpToDate) clears the rebuilding cue to idle', () async {
    final c = await atActive(FakeWalletProvisioner(exists: true));
    await notifierOf(c).rescan(RescanFromTime(DateTime(2022, 6)));
    expect(rescanOf(c), isA<WalletRescanRebuilding>());

    // The repopulating scan reaches the chain tip → the history is whole again.
    sync.emit(const SyncStatus.upToDate(tip: 2_500_000));
    await pumpEventQueue();

    expect(
      rescanOf(c),
      isA<WalletRescanIdle>(),
      reason: 'the cue is cleared exactly on reached-tip',
    );
  });

  test('a NON-tip status (Scanning) does NOT clear the rebuilding cue', () async {
    final c = await atActive(FakeWalletProvisioner(exists: true));
    await notifierOf(c).rescan(RescanFromTime(DateTime(2022, 6)));

    // Mid-scan progress must keep the cue up — the history is still repopulating.
    sync.emit(
      const SyncStatus.scanning(
        from: 2_000_000,
        to: 2_500_000,
        percent: 40,
        spendableReady: true,
        rewound: false,
      ),
    );
    await pumpEventQueue();

    expect(rescanOf(c), isA<WalletRescanRebuilding>());
  });

  test('the value-less loading on the session swap does NOT clear the rebuilding '
      'cue (it survives the swap, clears only on reached-tip)', () async {
    final c = await atActive(FakeWalletProvisioner(exists: true));
    await notifierOf(c).rescan(RescanFromTime(DateTime(2022, 6)));
    expect(rescanOf(c), isA<WalletRescanRebuilding>());

    // The session swap rebuilds syncStatusProvider → it re-emits loading. The cue
    // must NOT clear here (a future refactor matching the null SyncStatus inside a
    // loading AsyncValue would silently break the survive-the-swap guarantee).
    sync.emitLoading();
    await pumpEventQueue();
    expect(rescanOf(c), isA<WalletRescanRebuilding>());

    // ...then a genuine reached-tip clears it.
    sync.emit(const SyncStatus.upToDate(tip: 2_500_000));
    await pumpEventQueue();
    expect(rescanOf(c), isA<WalletRescanIdle>());
  });

  test('a rescan that fails-but-recovers shows the honest Failed cue; dismiss '
      'returns to idle', () async {
    final p = FakeWalletProvisioner(exists: true);
    final c = await atActive(p);
    // Make the rebuild fail (the re-open recovers — failOpen unset).
    p.failRescan = const WalletApiError(
      code: 'RW-STORE-004',
      message: 'rebuild failed',
      kind: WalletErrorKind.storeCorrupt(),
    );

    await notifierOf(c).rescan(const RescanAllHistory());
    expect(rescanOf(c), isA<WalletRescanFailed>());

    notifierOf(c).dismissFailure();
    expect(rescanOf(c), isA<WalletRescanIdle>());
  });

  test('the in-flight-send fence shows the hours-scale Blocked cue (its own '
      'state, never the generic Failed "moment" copy); dismiss returns to '
      'idle', () async {
    final p = FakeWalletProvisioner(exists: true);
    final c = await atActive(p);
    // The typed §4.4 witness-inversion fence: refused, then recovered by
    // re-open — the outcome (not the recovery) is what differs.
    p.failRescan = const WalletApiError(
      code: 'RW-LIFE-006',
      message: 'rescan refused while a send is in flight',
      kind: WalletErrorKind.rescanWithInFlightSend(),
    );

    await notifierOf(c).rescan(const RescanAllHistory());
    expect(rescanOf(c), isA<WalletRescanBlockedBySettlingSend>());

    notifierOf(c).dismissFailure();
    expect(rescanOf(c), isA<WalletRescanIdle>());
  });

  test('a disk-full rescan shows the actionable NeedsSpace cue (its own '
      'state, never the generic Failed "moment" copy — #375); dismiss returns '
      'to idle', () async {
    final p = FakeWalletProvisioner(exists: true);
    final c = await atActive(p);
    // The typed honest DiskFull from the rebuild + WAL fold: refused by the
    // disk, recovered by re-open — retrying without freeing space re-fails.
    p.failRescan = const WalletApiError(
      code: 'RW-STORE-005',
      message: 'not enough space to rebuild',
      kind: WalletErrorKind.diskFull(),
    );

    await notifierOf(c).rescan(const RescanAllHistory());
    expect(rescanOf(c), isA<WalletRescanFailedNeedsSpace>());

    notifierOf(c).dismissFailure();
    expect(rescanOf(c), isA<WalletRescanIdle>());
  });

  test(
    'a rescan that fails AND cannot re-open leaves the rescan UI idle',
    () async {
      final p = FakeWalletProvisioner(exists: true);
      final c = await atActive(p);
      // Both the rebuild and the recovery re-open fail → the whole surface becomes
      // OnboardingFailed; the rescan controller shows nothing (idle).
      p
        ..failRescan = const WalletApiError(
          code: 'RW-STORE-004',
          message: 'rebuild failed',
          kind: WalletErrorKind.storeCorrupt(),
        )
        ..failOpen = const WalletApiError(
          code: 'RW-KEYSTORE',
          message: 'locked',
          kind: WalletErrorKind.keystoreUnavailable(),
        );

      await notifierOf(c).rescan(const RescanAllHistory());

      expect(rescanOf(c), isA<WalletRescanIdle>());
      expect(c.read(onboardingControllerProvider), isA<OnboardingFailed>());
    },
  );

  test(
    'a double-tap while the rebuild is running is a no-op (one rebuild)',
    () async {
      final p = FakeWalletProvisioner(exists: true)
        ..holdRescan = Completer<void>();
      final c = await atActive(p);

      final first = notifierOf(c).rescan(RescanFromTime(DateTime(2022, 6)));
      // The controller is now Running; a second tap must no-op.
      expect(rescanOf(c), isA<WalletRescanRunning>());
      final second = notifierOf(c).rescan(RescanFromTime(DateTime(2020)));
      await pumpEventQueue();

      expect(p.rescanCount, 1, reason: 'only one rebuild ever ran');

      p.holdRescan!.complete();
      await first;
      await second;
      // The FIRST tap's date wins (the second never reached the controller).
      final target = (rescanOf(c) as WalletRescanRebuilding).target;
      expect((target as RescanFromTime).earliestTime, DateTime(2022, 6));
    },
  );

  // ── The COMMIT-point fence (security MED; #405 widened it) ──────────
  // The overflow-menu disable is ENTRY-only and the truth behind it is
  // reactive: a sheet opened while sync was healthy can straddle a host flip
  // OR a failed start and confirm here. A rescan wipes the DB and can only
  // rebuild via a sync pass — the fence must refuse VISIBLY at the COMMIT,
  // before any destructive call, for BOTH reasons a pass may not come.

  test('S205-c: a policy-off rescan() refuses VISIBLY WITHOUT ever '
      'reaching the onboarding rebuild — never a wipe stranded behind a dead '
      'sync loop, never a silent dead confirm', () async {
    final p = FakeWalletProvisioner(exists: true);
    final c = await atActive(
      p,
      extra: [walletSyncPolicyProvider.overrideWithValue(false)],
    );

    await notifierOf(c).rescan(const RescanAllHistory());

    expect(
      rescanOf(c),
      isA<WalletRescanBlockedBySyncNotRunning>(),
      reason:
          'refused VISIBLY on its OWN state — a silent dead confirm is a '
          'broken promise, and nothing destructive ran so this is not the '
          'shared Failed cue (#405)',
    );
    expect(
      p.rescanCount,
      0,
      reason: 'fenced at the COMMIT — the destructive call never happened',
    );
    expect(
      c.read(onboardingControllerProvider),
      isA<OnboardingActive>(),
      reason: 'the wallet is untouched (no session swap even began)',
    );

    // The refusal is an ordinary failed cue: dismissable back to idle.
    notifierOf(c).dismissFailure();
    expect(rescanOf(c), isA<WalletRescanIdle>());
  });

  test('S205-c counterfactual: with a pass coming (the harness default) the '
      'same rescan reaches the rebuild — the fence gates on the SSOT and on '
      'nothing else', () async {
    final p = FakeWalletProvisioner(exists: true);
    final c = await atActive(p);

    await notifierOf(c).rescan(const RescanAllHistory());

    expect(p.rescanCount, 1, reason: 'the rebuild ran');
    expect(rescanOf(c), isA<WalletRescanRebuilding>());
  });

  // #405 — THE HOLE THE POLICY-SHAPED FENCE LEFT OPEN. A start command that
  // FAILED leaves `walletSyncPolicyProvider` reading TRUE while the loop never
  // ran and never will without a retry, so the old fence PASSED exactly the
  // confirm it exists to stop and the destructive wipe went ahead behind a
  // "Rebuilding" promise nothing could keep. Fully wired: the REAL sync
  // controller reaches `failed` off a throwing `startSync`, the REAL
  // `walletSyncPassesRunProvider` derives `false` from it, and the REAL fence
  // reads that.
  test('#405: a FAILED-start rescan() is refused at the COMMIT too — the '
      'policy still reads ON, so only the SSOT catches this one', () async {
    final p = FakeWalletProvisioner(
      exists: true,
      session: FakeWalletSession()..failStart = true,
    );
    final c = await atActive(p);
    // Build + settle the sync drive: the start throws, the controller lands on
    // `failed`, and the policy is untouched (still the default `true`).
    c.listen(walletSyncControllerProvider, (_, _) {});
    await pumpEventQueue();
    expect(
      c.read(walletSyncControllerProvider),
      WalletSyncDrive.failed,
      reason: 'precondition — the start command threw',
    );
    expect(
      c.read(walletSyncPolicyProvider),
      isTrue,
      reason:
          'precondition — the POLICY is ON, which is exactly why the '
          'policy-shaped fence let this through',
    );
    expect(
      c.read(walletSyncPassesRunProvider),
      isFalse,
      reason: 'the SSOT sees what the policy cannot',
    );

    await notifierOf(c).rescan(const RescanAllHistory());

    expect(rescanOf(c), isA<WalletRescanBlockedBySyncNotRunning>());
    expect(
      p.rescanCount,
      0,
      reason:
          'THE POINT: the destructive wipe never happened. Before #405 this '
          'was 1 — the DB was rebuilt against a loop that was not running',
    );
    expect(
      c.read(onboardingControllerProvider),
      isA<OnboardingActive>(),
      reason: 'the wallet is untouched (no session swap even began)',
    );

    // Dismissable back to idle like every other refusal cue.
    notifierOf(c).dismissFailure();
    expect(rescanOf(c), isA<WalletRescanIdle>());
  });

  // ── Wallet-identity reset (#380 (c)) ───────────────────────────────────────
  // The controller deliberately outlives the session swap — so without an
  // explicit reset, its state also outlives the WALLET: a deleted wallet's
  // Rebuilding banner would render over the NEXT wallet's surface until ITS
  // first reached-tip, and a failure notice would describe a wallet that no
  // longer exists.

  test('deleting the wallet clears a live Rebuilding cue — the next wallet '
      'must not render the dead wallet\'s banner (#380 (c))', () async {
    final c = await atActive(FakeWalletProvisioner(exists: true));
    await notifierOf(c).rescan(const RescanAllHistory());
    expect(rescanOf(c), isA<WalletRescanRebuilding>(), reason: 'precondition');

    final outcome = await c
        .read(onboardingControllerProvider.notifier)
        .deleteWallet();
    expect(outcome, WalletDeletionOutcome.shredded, reason: 'precondition');
    expect(c.read(onboardingControllerProvider), isA<OnboardingWelcome>());

    expect(
      rescanOf(c),
      isA<WalletRescanIdle>(),
      reason: 'the cue described the shredded wallet',
    );
  });

  test('deleting the wallet clears a Failed notice — it must not outlive the '
      'wallet it described (#380 (c))', () async {
    final p = FakeWalletProvisioner(exists: true)
      ..failRescan = const WalletApiError(
        code: 'RW-STORE-004',
        message: 'rebuild failed',
        kind: WalletErrorKind.storeCorrupt(),
      );
    final c = await atActive(p);
    await notifierOf(c).rescan(const RescanAllHistory());
    expect(rescanOf(c), isA<WalletRescanFailed>(), reason: 'precondition');

    p.failRescan = null; // the delete itself must succeed
    await c.read(onboardingControllerProvider.notifier).deleteWallet();
    expect(c.read(onboardingControllerProvider), isA<OnboardingWelcome>());

    expect(rescanOf(c), isA<WalletRescanIdle>());
  });

  // ── The durable catch-up cue (#380 (a)/(b)) ────────────────────────────────
  // walletCatchUpCueProvider re-derives the explanation from what the wallet
  // itself knows (lastSynced == null + below tip), so it survives the exact
  // states where the in-memory controller has forgotten: a process death
  // mid-catch-up and a failure-notice dismiss over a rebuilt wallet.

  group('walletCatchUpCueProvider', () {
    const scanning = SyncStatus.scanning(
      from: 2_000_000,
      to: 2_500_000,
      percent: 40,
      spendableReady: false,
      rewound: false,
    );

    ProviderContainer cueHarness(
      FakeWalletSession session, {
      List<Override> extra = const [],
    }) {
      sync = _StubSyncNotifier();
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(session),
          syncStatusProvider.overrideWith(() => sync),
          ...extra,
        ],
      );
      addTearDown(container.dispose);
      container.listen(walletCatchUpCueProvider, (_, _) {});
      return container;
    }

    test('a never-synced wallet below tip derives the Syncing cue on a FRESH '
        'container — the relaunch-survival property (#380 (a))', () async {
      // A fresh container models the post-death relaunch: the rescan
      // controller is back at its built-in Idle, and ONLY the durable pair
      // (a null lastSynced stamp + a below-tip status) carries the truth.
      final c = cueHarness(
        FakeWalletSession(
          current: scanning,
          snapshotValue: walletStateFixture(syncStatus: scanning),
        ),
      );
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpSyncing>());
    });

    test('a previously-synced wallet below tip derives NO cue — routine '
        'catch-up scans stay uncluttered', () async {
      final c = cueHarness(
        FakeWalletSession(
          current: scanning,
          snapshotValue: walletStateFixture(
            syncStatus: scanning,
            lastSynced: const SyncStamp(height: 2_400_000, at: 0),
          ),
        ),
      );
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());
    });

    test('reached-tip derives NO cue even over a still-null stamp — the live '
        'status clears the cue on the same edge that clears the '
        'controller', () async {
      final c = cueHarness(
        FakeWalletSession(
          current: scanning,
          snapshotValue: walletStateFixture(syncStatus: scanning),
        ),
      );
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpSyncing>());

      sync.emit(const SyncStatus.upToDate(tip: 2_500_000));
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());
    });

    test(
      '#357: a reached-tip (everSynced) wallet OFFLINE with a null stamp '
      'derives NO cue — everSynced is CLEARED on rescan, so a set flag '
      'unambiguously means a settled balance; suppress the over-explanation',
      () async {
        const offline = SyncStatus.offline();
        final c = cueHarness(
          FakeWalletSession(
            current: offline,
            // everSynced true + null stamp = reached tip SINCE the last rescan (a
            // swallowed stamp-write fault / pre-#317 / offline relaunch) ⇒ settled.
            snapshotValue: walletStateFixture(
              syncStatus: offline,
              everSynced: true,
            ),
          ),
        );
        await pumpEventQueue();
        expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());
      },
    );

    test('#357 HIGH fix: a post-rescan-rebuild wallet OFFLINE (everSynced '
        'CLEARED) STILL shows the Syncing cue — the "did I lose funds?" panic '
        'window a `!scanning` proxy wrongly suppressed', () async {
      const offline = SyncStatus.offline();
      final c = cueHarness(
        FakeWalletSession(
          current: offline,
          // A rescan clears everSynced (core resets the row); process death +
          // offline relaunch mid-rebuild ⇒ everSynced FALSE + null stamp + offline
          // (NOT scanning). The rebuilding balance MUST keep its reassurance.
          snapshotValue: walletStateFixture(
            syncStatus: offline,
            everSynced: false,
          ),
        ),
      );
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpSyncing>());
    });

    test('#357: a reached-tip (everSynced) wallet on a routine SCAN suppresses '
        'the cue — a few new blocks over a settled balance need no first-run '
        'framing', () async {
      final c = cueHarness(
        FakeWalletSession(
          current: scanning,
          // everSynced true (reached tip since last rescan) + a routine top-up
          // scan ⇒ settled balance, no over-explanation.
          snapshotValue: walletStateFixture(
            syncStatus: scanning,
            everSynced: true,
          ),
        ),
      );
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());
    });

    test('#377 s357b-2: the durable rescan breadcrumb upgrades the relaunch '
        'cue to Rebuilding(target: null) — the banner can name the rescan the '
        'user chose, even OFFLINE, instead of the first-run framing', () async {
      const offline = SyncStatus.offline();
      final c = cueHarness(
        FakeWalletSession(
          current: offline,
          // Process death mid-rebuild + offline relaunch: the controller is
          // back at Idle (choice lost), everSynced cleared by the rescan, and
          // ONLY the durable breadcrumb carries "this emptiness is a rescan".
          snapshotValue: walletStateFixture(
            syncStatus: offline,
            everSynced: false,
            rescanRebuilding: true,
          ),
        ),
      );
      await pumpEventQueue();
      final cue = c.read(walletCatchUpCueProvider);
      expect(cue, isA<WalletCatchUpRebuilding>());
      expect(
        (cue as WalletCatchUpRebuilding).target,
        isNull,
        reason: 'the range choice did not survive the relaunch — only the fact',
      );
    });

    test('#377 s357b-2: a STALE breadcrumb (swallowed clear fault at tip) is '
        'masked by everSynced — the banner can never pin over a settled '
        'balance', () async {
      const offline = SyncStatus.offline();
      final c = cueHarness(
        FakeWalletSession(
          current: offline,
          // The fault window: reached-tip set everSynced but the independent
          // breadcrumb clear failed. everSynced is checked FIRST, so the
          // settled balance stays uncluttered until a later pass re-clears.
          snapshotValue: walletStateFixture(
            syncStatus: offline,
            everSynced: true,
            rescanRebuilding: true,
          ),
        ),
      );
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());
    });

    test('#377 s357b-2: reached-tip THIS session suppresses the breadcrumb arm '
        'on the same edge that clears the controller', () async {
      final c = cueHarness(
        FakeWalletSession(
          current: scanning,
          snapshotValue: walletStateFixture(
            syncStatus: scanning,
            rescanRebuilding: true,
          ),
        ),
      );
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpRebuilding>());

      sync.emit(const SyncStatus.upToDate(tip: 2_500_000));
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());
    });

    test('the intra-session Rebuilding fast path wins and carries the target '
        '(the banner names what the user chose)', () async {
      final c = cueHarness(
        FakeWalletSession(
          current: scanning,
          snapshotValue: walletStateFixture(syncStatus: scanning),
        ),
        extra: [
          walletRescanControllerProvider.overrideWith(
            () => _FixedRescan(
              const WalletRescanRebuilding(target: RescanAllHistory()),
            ),
          ),
        ],
      );
      await pumpEventQueue();
      final cue = c.read(walletCatchUpCueProvider);
      expect(cue, isA<WalletCatchUpRebuilding>());
      expect((cue as WalletCatchUpRebuilding).target, isA<RescanAllHistory>());
    });

    test('dismissing a failure below tip hands over to the durable cue — the '
        'explanation never leaves the screen (#380 (b))', () async {
      final stub = _FixedRescan(const WalletRescanFailed());
      final c = cueHarness(
        FakeWalletSession(
          current: scanning,
          snapshotValue: walletStateFixture(syncStatus: scanning),
        ),
        extra: [walletRescanControllerProvider.overrideWith(() => stub)],
      );
      await pumpEventQueue();
      // Pre-dismiss the durable arm already explains (the activity cue keys
      // off it even while the notice renders above).
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpSyncing>());

      c.read(walletRescanControllerProvider.notifier).dismissFailure();
      await pumpEventQueue();
      expect(c.read(walletRescanControllerProvider), isA<WalletRescanIdle>());
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpSyncing>());
    });

    test('a previously-synced wallet at dismissal derives NO cue — the intact '
        'pre-rename/fence arms never claim a rebuild (#380 (b))', () async {
      final stub = _FixedRescan(const WalletRescanBlockedBySettlingSend());
      final c = cueHarness(
        FakeWalletSession(
          current: scanning,
          snapshotValue: walletStateFixture(
            syncStatus: scanning,
            lastSynced: const SyncStamp(height: 2_400_000, at: 0),
          ),
        ),
        extra: [walletRescanControllerProvider.overrideWith(() => stub)],
      );
      await pumpEventQueue();
      c.read(walletRescanControllerProvider.notifier).dismissFailure();
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());
    });

    test('a tip this SESSION proved is never un-proved by a routine scan — '
        'the cue cannot resurrect over a final balance while the stamp '
        're-read lags or faults (S190 review fold)', () async {
      // The stamp's visibility to the cue rides an async snapshot re-read
      // owned by widget/resume listeners; this harness deliberately never
      // re-reads (the fake's stamp stays null past the tip) — modelling the
      // busy-DB re-read fault AND the swap-surface-only host, the two real
      // paths where the durable pair alone would flap the cue back on.
      final c = cueHarness(
        FakeWalletSession(
          current: scanning,
          snapshotValue: walletStateFixture(syncStatus: scanning),
        ),
      );
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpSyncing>());

      // Reached-tip: the session PROVES the balance final (the latch).
      sync.emit(const SyncStatus.upToDate(tip: 2_500_000));
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());

      // A routine post-tip scan (new block) with the stamp STILL null must
      // NOT resurrect "catching up" — the balance it would explain is final.
      sync.emit(scanning);
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());

      // An offline drop post-tip likewise.
      sync.emit(const SyncStatus.offline());
      await pumpEventQueue();
      expect(c.read(walletCatchUpCueProvider), isA<WalletCatchUpNone>());
    });
  });
}

/// A fixed-state [WalletRescanController] stub (the wallet_screen_test idiom):
/// renders any presentation phase without driving a real rescan; the REAL
/// [WalletRescanController.dismissFailure] transition still applies.
class _FixedRescan extends WalletRescanController {
  _FixedRescan(this._initial);
  final WalletRescanState _initial;

  @override
  WalletRescanState build() => _initial;
}
