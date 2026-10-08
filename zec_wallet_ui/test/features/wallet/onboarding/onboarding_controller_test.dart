import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_controller.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_store.dart'
    show DeepScanRestoreNoteState;
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The onboarding state machine (spec §3.2g iii-B) — the money-safety gate.
/// Driven against host-VM fakes (no native lib, no device): the controller
/// depends only on the `WalletProvisioner`/`OnboardingStore` ports.
///
/// THE invariant under test (the deposit gate): a deposit-capable
/// `WalletSession` is exposed to the wallet surface — `walletSessionProvider` —
/// ONLY in [OnboardingActive], i.e. ONLY after the recovery-phrase backup is
/// confirmed-and-PERSISTED. Every other phase (including a provisioned-but-
/// unconfirmed wallet resumed after a crash) yields `null`.
void main() {
  // Build a container wired with the two fakes; kept alive by a listener so the
  // boot probe's async result is observed. No timers in this machine, so
  // `pumpEventQueue` drains every continuation deterministically.
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

  group('boot / resume fork', () {
    test('no wallet on disk resumes to welcome (no session)', () async {
      final c = harness(
        FakeWalletProvisioner(exists: false),
        FakeOnboardingStore(),
      );

      // build() is synchronous → the transient loading first.
      expect(stateOf(c), isA<OnboardingLoading>());
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingWelcome>());
      expect(sessionOf(c), isNull);
    });

    test('a confirmed wallet resumes active and EXPOSES the session', () async {
      final p = FakeWalletProvisioner(exists: true);
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingActive>());
      // The gate is open exactly because backup was confirmed.
      expect(sessionOf(c), same(p.session));
      expect(p.openCount, 1);
    });

    test('a provisioned but UNCONFIRMED wallet resumes into FORCED backup, '
        'never active (the crash-resume money-safety crux)', () async {
      // A wallet was created on a prior launch but the app died before the
      // user confirmed the backup. On restart it MUST re-force the backup —
      // a deposit must never be invited into an un-backed-up wallet.
      final p = FakeWalletProvisioner(exists: true);
      final c = harness(p, FakeOnboardingStore(confirmed: false));
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingAwaitingBackup>());
      expect(sessionOf(c), isNull, reason: 'not deposit-ready before backup');
      expect(p.openCount, 1, reason: 'opened to drive the forced backup');
    });

    test(
      'an unset confirmation flag fails SAFE (forced backup, not active)',
      () async {
        // confirmed == null (prefs lost / reinstall over an existing DB): the
        // fail-safe default is "not confirmed" → re-force the backup.
        final p = FakeWalletProvisioner(exists: true);
        final c = harness(p, FakeOnboardingStore());
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingAwaitingBackup>());
        expect(sessionOf(c), isNull);
      },
    );

    test('a degraded backup-flag read fails WITHOUT opening the wallet '
        '(no orphaned single-writer lock)', () async {
      // RELIABILITY (reviewer fold): the controller reads the host backup flag
      // BEFORE opening. If that read fails (a degraded shared_preferences
      // channel on a flaky device), open() must NOT be attempted — an
      // opened-then-dropped handle would hold the single-writer lock until GC
      // and make a retry hit walletAlreadyOpen.
      final p = FakeWalletProvisioner(exists: true);
      final store = FakeOnboardingStore()
        ..failGet = const WalletApiError(
          code: 'RW-PREFS',
          message: 'degraded prefs',
          kind: WalletErrorKind.io(),
        );
      final c = harness(p, store);
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingFailed>());
      expect(p.openCount, 0, reason: 'no wallet opened → no orphaned lock');
      expect(store.getCount, 1, reason: 'the flag read was attempted first');
    });

    test('a TRANSIENT backup-flag read failure recovers on retry', () async {
      // The flag read is now the FIRST step of the resume fork (reviewer fold),
      // so a flaky shared_preferences channel on boot is a new first-failure
      // surface. It must be transient-recoverable: fail → Failed(retryable) →
      // clear → retry re-probes → the correct resumed state.
      final p = FakeWalletProvisioner(exists: true);
      final store = FakeOnboardingStore(confirmed: true)
        ..failGet = const WalletApiError(
          code: 'RW-PREFS',
          message: 'degraded prefs',
          kind: WalletErrorKind.io(),
        );
      final c = harness(p, store);
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingFailed>());
      expect(p.openCount, 0);

      // The channel recovers; retry re-runs the probe to the confirmed wallet.
      store.failGet = null;
      await c.read(onboardingControllerProvider.notifier).retry();
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingActive>());
      expect(sessionOf(c), same(p.session));
    });
  });

  group('create flow', () {
    test('create moves to awaiting-backup, NEVER straight to active', () async {
      final p = FakeWalletProvisioner(exists: false);
      final c = harness(p, FakeOnboardingStore());
      await pumpEventQueue(); // -> welcome

      await notifierOf(c).startCreate();
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingAwaitingBackup>());
      expect(
        sessionOf(c),
        isNull,
        reason: 'no deposit before backup confirmed',
      );
      expect(p.createCount, 1);
    });

    test('create resets the backup gate BEFORE writing the wallet '
        '(crash-safe ordering)', () async {
      // If the gate reset can\'t be persisted, the wallet must NOT be
      // created — else a crash could leave an un-gated wallet on disk. The
      // store throws on the reset; create must abort having written nothing.
      final p = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore()..failSet = StateError('prefs down');
      final c = harness(p, store);
      await pumpEventQueue(); // -> welcome

      await notifierOf(c).startCreate();
      await pumpEventQueue();

      expect(
        p.createCount,
        0,
        reason: 'no wallet written without a closed gate',
      );
      expect(
        store.lastSetValue,
        isFalse,
        reason: 'the reset was attempted first',
      );
      expect(stateOf(c), isA<OnboardingFailed>());
      expect(sessionOf(c), isNull);
    });

    test('a create failure surfaces Failed with the classified kind', () async {
      final p = FakeWalletProvisioner(exists: false)
        ..failCreate = const WalletApiError(
          code: 'RW-STORE-OOM',
          message: 'disk full',
          kind: WalletErrorKind.diskFull(),
        );
      final c = harness(p, FakeOnboardingStore());
      await pumpEventQueue(); // -> welcome

      await notifierOf(c).startCreate();
      await pumpEventQueue();

      final st = stateOf(c);
      expect(st, isA<OnboardingFailed>());
      expect((st as OnboardingFailed).kind, OnboardingFailureKind.storageFull);
      expect(st.kind.isRetryable, isTrue);
      expect(sessionOf(c), isNull);
    });

    test('startCreate is a no-op outside welcome', () async {
      final p = FakeWalletProvisioner(exists: true);
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue(); // -> active

      await notifierOf(c).startCreate();
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingActive>());
      expect(p.createCount, 0);
    });
  });

  group('confirm flow', () {
    test('confirm PERSISTS then activates and exposes the session', () async {
      final p = FakeWalletProvisioner(exists: true);
      final store = FakeOnboardingStore(confirmed: false);
      final c = harness(p, store);
      await pumpEventQueue(); // -> awaiting backup (unconfirmed on disk)

      expect(sessionOf(c), isNull);
      await notifierOf(c).confirmBackup();
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingActive>());
      expect(store.persisted, isTrue, reason: 'confirmation durably persisted');
      expect(store.lastSetValue, isTrue);
      expect(sessionOf(c), same(p.session), reason: 'gate opens only now');
    });

    test(
      'confirm keeps the gate CLOSED when the persist fails (no deposit on a '
      'confirmation that did not survive)',
      () async {
        final p = FakeWalletProvisioner(exists: true);
        final store = FakeOnboardingStore(confirmed: false)
          ..failSet = StateError('prefs write failed');
        final c = harness(p, store);
        await pumpEventQueue(); // -> awaiting backup

        // The failure rethrows so the screen can surface "try again"…
        await expectLater(
          notifierOf(c).confirmBackup(),
          throwsA(isA<StateError>()),
        );
        await pumpEventQueue();

        // …and the gate stayed shut: back to awaiting, still no session.
        expect(stateOf(c), isA<OnboardingAwaitingBackup>());
        expect(
          store.persisted,
          isNot(true),
          reason: 'never recorded confirmed',
        );
        expect(sessionOf(c), isNull);
      },
    );

    test('confirmBackup is a no-op outside awaiting-backup', () async {
      final p = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      final c = harness(p, store);
      await pumpEventQueue(); // -> welcome

      await notifierOf(c).confirmBackup();
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingWelcome>());
      expect(store.setCount, 0);
    });
  });

  group('error + retry', () {
    test('an open failure on resume surfaces Failed; retry re-probes', () async {
      // Device locked at launch → KeystoreUnavailable → deviceLocked (retryable).
      final p = FakeWalletProvisioner(exists: true)
        ..failOpen = const WalletApiError(
          code: 'RW-KEY-LOCKED',
          message: 'keystore locked',
          kind: WalletErrorKind.keystoreUnavailable(),
        );
      final store = FakeOnboardingStore(confirmed: true);
      final c = harness(p, store);
      await pumpEventQueue();

      final failed = stateOf(c);
      expect(failed, isA<OnboardingFailed>());
      expect(
        (failed as OnboardingFailed).kind,
        OnboardingFailureKind.deviceLocked,
      );
      expect(failed.kind.isRetryable, isTrue);
      expect(sessionOf(c), isNull);

      // User unlocks the device, then retries: the probe now succeeds → active.
      p.failOpen = null;
      await notifierOf(c).retry();
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingActive>());
      expect(sessionOf(c), same(p.session));
      expect(p.openCount, 2, reason: 're-probed on retry');
    });

    test('a walletExists failure surfaces Failed(unknown)', () async {
      final p = FakeWalletProvisioner()..failWalletExists = StateError('io');
      final c = harness(p, FakeOnboardingStore());
      await pumpEventQueue();

      final st = stateOf(c);
      expect(st, isA<OnboardingFailed>());
      expect((st as OnboardingFailed).kind, OnboardingFailureKind.unknown);
    });

    test('retry outside a failed state is a no-op (no re-probe flash)', () async {
      // retry() is the Failed-screen Retry action, not a general re-probe: from
      // Active it must NOT re-open the wallet or flash Loading.
      final p = FakeWalletProvisioner(exists: true);
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue(); // -> active
      expect(stateOf(c), isA<OnboardingActive>());
      expect(p.openCount, 1);

      await notifierOf(c).retry();
      await pumpEventQueue();

      expect(
        stateOf(c),
        isA<OnboardingActive>(),
        reason: 'no transient Loading',
      );
      expect(p.openCount, 1, reason: 'no re-probe from a non-failed state');
      expect(sessionOf(c), same(p.session));
    });
  });

  group('no backend wired (production seam)', () {
    test(
      'null provisioner/store is Unavailable and exposes no session',
      () async {
        final container = ProviderContainer(
          overrides: [
            walletProvisionerProvider.overrideWithValue(null),
            onboardingStoreProvider.overrideWithValue(null),
          ],
        );
        addTearDown(container.dispose);
        container.listen(onboardingControllerProvider, (_, _) {});

        expect(
          container.read(onboardingControllerProvider),
          isA<OnboardingUnavailable>(),
        );
        expect(container.read(walletSessionProvider), isNull);
      },
    );
  });

  group('disposal safety', () {
    test('disposing mid-probe never sets state after dispose (no throw)', () async {
      final hold = Completer<void>();
      final p = FakeWalletProvisioner(exists: true)..holdOpen = hold;
      final container = ProviderContainer(
        overrides: [
          walletProvisionerProvider.overrideWithValue(p),
          onboardingStoreProvider.overrideWithValue(
            FakeOnboardingStore(confirmed: true),
          ),
        ],
      );
      container.listen(onboardingControllerProvider, (_, _) {});

      expect(
        container.read(onboardingControllerProvider),
        isA<OnboardingLoading>(),
      );
      await pumpEventQueue(); // walletExists resolved; open() now awaiting the gate

      container.dispose(); // tear down WHILE the open is in flight
      hold.complete(); // open resolves AFTER dispose — the guard must swallow it
      await pumpEventQueue(); // a post-dispose `state =` would throw here
      // Reaching here without an exception is the assertion.
    });
  });

  group('classifier (boundary, pure)', () {
    test('maps the onboarding-relevant kinds; unmapped/non-FRB → unknown', () {
      WalletApiError err(WalletErrorKind kind) =>
          WalletApiError(code: 'X', message: 'x', kind: kind);

      expect(
        classifyOnboardingFailure(
          err(const WalletErrorKind.keystoreUnavailable()),
        ),
        OnboardingFailureKind.deviceLocked,
      );
      expect(
        classifyOnboardingFailure(
          err(const WalletErrorKind.walletAlreadyOpen()),
        ),
        OnboardingFailureKind.alreadyOpen,
      );
      // T0-7 (§4aa OP-5): a held lock during the FIRST OPEN of a new or
      // seed-restored wallet. Before T0-7 the core folded it into
      // `storeCorrupt` and this screen showed the RED "restore from your
      // recovery phrase" to a user whose wallet was never written; the core now
      // classifies it, and it must land on a retryable kind that names the next
      // step. `alreadyOpen` is that kind — "another instance holds this wallet,
      // close it and retry" — and it must NOT fall through to `unknown`, whose
      // copy names no step at all.
      expect(
        classifyOnboardingFailure(err(const WalletErrorKind.storeBusy())),
        OnboardingFailureKind.alreadyOpen,
      );
      expect(
        OnboardingFailureKind.alreadyOpen.isRetryable,
        isTrue,
        reason: 'a transient lock must offer Retry',
      );
      expect(
        classifyOnboardingFailure(err(const WalletErrorKind.vaultAbsent())),
        OnboardingFailureKind.noVault,
      );
      expect(
        classifyOnboardingFailure(err(const WalletErrorKind.diskFull())),
        OnboardingFailureKind.storageFull,
      );
      expect(
        classifyOnboardingFailure(
          err(
            const WalletErrorKind.sync_(stall: StallReason.endpointUnreachable),
          ),
        ),
        OnboardingFailureKind.network,
      );
      // All of the damaged-data kinds collapse to needsRecovery (restore, not
      // retry) — cover every arm of the union so a future re-route is caught.
      expect(
        classifyOnboardingFailure(err(const WalletErrorKind.storeCorrupt())),
        OnboardingFailureKind.needsRecovery,
      );
      expect(
        classifyOnboardingFailure(err(const WalletErrorKind.sealInvalid())),
        OnboardingFailureKind.needsRecovery,
      );
      expect(
        classifyOnboardingFailure(
          err(const WalletErrorKind.sealVersionUnsupported(found: 9)),
        ),
        OnboardingFailureKind.needsRecovery,
      );
      expect(
        classifyOnboardingFailure(
          err(const WalletErrorKind.wrapArtifactInvalid()),
        ),
        OnboardingFailureKind.needsRecovery,
      );
      expect(
        classifyOnboardingFailure(
          err(const WalletErrorKind.wrapVersionUnsupported(found: 9)),
        ),
        OnboardingFailureKind.needsRecovery,
      );
      expect(
        classifyOnboardingFailure(
          err(
            const WalletErrorKind.keystoreInconsistent(
              permanentlyInvalidated: true,
            ),
          ),
        ),
        OnboardingFailureKind.needsRecovery,
      );
      expect(
        classifyOnboardingFailure(
          err(
            const WalletErrorKind.keystoreInconsistent(
              permanentlyInvalidated: false,
            ),
          ),
        ),
        OnboardingFailureKind.needsRecovery,
      );
      // An interrupted create routes to the retryable interruptedSetup (open
      // can't repair a remnant; a retry re-probes and create resumes it) — NOT
      // the unknown fall-through (the iii-B-2-b-ii defense-in-depth pin).
      expect(
        classifyOnboardingFailure(
          err(const WalletErrorKind.provisioningIncomplete()),
        ),
        OnboardingFailureKind.interruptedSetup,
      );
      // A raw IO failure (the degraded-filesystem probe case): the SDK THROWS
      // it (never Ok(false)→Welcome), so it reaches a retryable failure here —
      // explicitly pinned, never an accidental fall-through.
      expect(
        classifyOnboardingFailure(err(const WalletErrorKind.io())),
        OnboardingFailureKind.unknown,
      );
      // An unmapped FRB kind and a plain Dart error both fall to unknown.
      expect(
        classifyOnboardingFailure(err(const WalletErrorKind.notFound())),
        OnboardingFailureKind.unknown,
      );
      expect(
        classifyOnboardingFailure(StateError('boom')),
        OnboardingFailureKind.unknown,
      );
    });

    test('retryable vs recovery axis is honest', () {
      // Transient → retry; damaged/absent → a different recovery path.
      expect(OnboardingFailureKind.deviceLocked.isRetryable, isTrue);
      expect(OnboardingFailureKind.alreadyOpen.isRetryable, isTrue);
      expect(OnboardingFailureKind.storageFull.isRetryable, isTrue);
      expect(OnboardingFailureKind.network.isRetryable, isTrue);
      expect(OnboardingFailureKind.interruptedSetup.isRetryable, isTrue);
      expect(OnboardingFailureKind.unknown.isRetryable, isTrue);
      expect(OnboardingFailureKind.needsRecovery.isRetryable, isFalse);
      expect(OnboardingFailureKind.noVault.isRetryable, isFalse);
    });
  });

  group('model: the deposit gate holds across a seeded walk', () {
    test('session is exposed iff active iff backup persisted', () async {
      // A deterministic, legal action sequence (testing-patterns §4). Illegal
      // actions are no-ops by the controller\'s own guards, so a blind rotation
      // stays legal. After EVERY step the core safety invariant must hold.
      final p = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      final c = harness(p, store);
      await pumpEventQueue(); // -> welcome

      final n = notifierOf(c);
      final actions = <Future<void> Function()>[
        n.confirmBackup, // no-op: not awaiting
        n.startCreate, //   welcome -> awaiting backup
        n.startCreate, //   no-op: not welcome
        n.confirmBackup, // awaiting -> active (persists)
        n.confirmBackup, // no-op: not awaiting
        n.retry, //         no-op: guarded to the failed state (stays active)
        n.startCreate, //   no-op: not welcome
      ];

      for (var i = 0; i < actions.length; i++) {
        try {
          await actions[i]();
        } catch (_) {
          // confirm rethrows on a persist failure; the invariant must STILL hold.
        }
        await pumpEventQueue();

        final st = stateOf(c);
        final session = sessionOf(c);
        final active = st is OnboardingActive;
        expect(
          session != null,
          active,
          reason: 'step $i: a session is exposed iff onboarding is active',
        );
        if (active) {
          expect(
            store.persisted,
            isTrue,
            reason: 'step $i: active implies a durably-persisted backup',
          );
        }
      }

      // The walk actually reached the deposit-ready state (not vacuously safe).
      expect(stateOf(c), isA<OnboardingActive>());
      expect(sessionOf(c), isNotNull);
    });

    test(
      'second model walk: a transient open failure routes through retry to '
      'active — session exposed iff active iff persisted at every step',
      () async {
        // The resume fork sees a wallet on disk WITH a confirmed backup, but
        // the keystore is transiently locked → the probe fails first. A blind
        // rotation that includes retry() (guarded to the failed state) must
        // recover to active once the fault clears, and the deposit-gate
        // invariant must hold after EVERY step — including the failed ones,
        // where no session may leak despite a confirmed-on-disk backup.
        final p = FakeWalletProvisioner(exists: true)
          ..failOpen = const WalletApiError(
            code: 'RW-KEY-LOCKED',
            message: 'keystore locked',
            kind: WalletErrorKind.keystoreUnavailable(),
          );
        final store = FakeOnboardingStore(confirmed: true);
        final c = harness(p, store);
        await pumpEventQueue(); // -> Failed(deviceLocked): open() threw

        expect(
          stateOf(c),
          isA<OnboardingFailed>(),
          reason: 'transient open fault surfaces Failed, not active',
        );
        expect(
          sessionOf(c),
          isNull,
          reason:
              'a confirmed-on-disk wallet still exposes NO session while '
              'the open is failing — the gate is state-driven, not '
              'flag-driven',
        );

        final n = notifierOf(c);
        // Step indices line up with the clears below so the injected fault is
        // deterministic: steps 0–2 run while still locked (retry re-fails),
        // the fault clears, then step 3's retry reaches active.
        final steps = <(String, Future<void> Function())>[
          ('confirmBackup while failed (no-op)', n.confirmBackup),
          ('startCreate while failed (no-op)', n.startCreate),
          ('retry still locked -> re-Failed', n.retry),
          (
            'clear-then-retry -> active',
            () async {
              p.failOpen = null; // the device was unlocked
              await n.retry();
            },
          ),
          ('retry from active (no-op, no re-open)', n.retry),
          ('startCreate from active (no-op)', n.startCreate),
        ];

        for (var i = 0; i < steps.length; i++) {
          final (label, action) = steps[i];
          try {
            await action();
          } catch (_) {
            // No step here is expected to throw, but the invariant must hold
            // regardless of how an action terminates.
          }
          await pumpEventQueue();

          final st = stateOf(c);
          final session = sessionOf(c);
          final active = st is OnboardingActive;
          expect(
            session != null,
            active,
            reason: 'step $i ($label): session exposed iff active',
          );
          if (active) {
            expect(
              store.persisted,
              isTrue,
              reason: 'step $i ($label): active implies a persisted backup',
            );
          }
        }

        // Recovery actually happened (not vacuously safe): the cleared retry
        // re-probed and opened the wallet. Three opens total — the boot probe
        // (failed), the still-locked retry (failed), and the cleared retry
        // (succeeded). Each failed open re-probes a fresh open; only the last
        // resolves to active.
        expect(stateOf(c), isA<OnboardingActive>());
        expect(sessionOf(c), same(p.session));
        expect(
          p.openCount,
          3,
          reason:
              'boot open (failed) + still-locked retry (failed) + '
              'cleared retry (succeeded)',
        );
      },
    );
  });

  // The operational pass the build tests skipped: two taps racing the same
  // async step, and a container torn down WHILE a persist is mid-flight. These
  // are the money-relevant re-entrancy edges — a double-create would mint a
  // SECOND wallet (orphaning the first's funds), and a post-dispose `state =`
  // would crash the app on shutdown. Both must be impossible by construction.
  group('deeper pass: re-entrancy + disposal', () {
    test('double-tap startCreate creates exactly ONE wallet (the synchronous '
        'Generating set makes the second tap a no-op)', () async {
      // Two rapid taps on "Create wallet" before the first await yields. The
      // first call synchronously sets OnboardingGenerating BEFORE its first
      // await; the second call, seeing state != Welcome, must early-return.
      // A second createGenerated() would mint a second seed and strand the
      // first wallet's funds — it must NEVER happen.
      final p = FakeWalletProvisioner(exists: false);
      final store = FakeOnboardingStore();
      final c = harness(p, store);
      await pumpEventQueue(); // -> welcome

      final n = notifierOf(c);
      final first = n.startCreate();
      final second = n.startCreate(); // NO await between the taps
      await Future.wait([first, second]);
      await pumpEventQueue();

      expect(
        p.createCount,
        1,
        reason: 'the second tap must not create a wallet',
      );
      expect(stateOf(c), isA<OnboardingAwaitingBackup>());
      expect(
        sessionOf(c),
        isNull,
        reason: 'no deposit before backup confirmed',
      );
      // The crash-safe gate reset ran exactly once too (one create, one reset).
      expect(store.setCount, 1, reason: 'the gate was reset once, not twice');
      expect(store.lastSetValue, isFalse);
    });

    test(
      'double-tap confirmBackup persists exactly ONCE and exposes the session '
      '(the synchronous Confirming set makes the second tap a no-op)',
      () async {
        // Two rapid taps on "I\'ve backed it up". The first synchronously sets
        // OnboardingConfirming before awaiting the persist; the second, seeing
        // state != AwaitingBackup, early-returns. The true-write must land
        // exactly once and the gate open exactly once.
        final p = FakeWalletProvisioner(exists: true);
        final store = FakeOnboardingStore(confirmed: false);
        final c = harness(p, store);
        await pumpEventQueue(); // -> awaiting backup (unconfirmed on disk)
        expect(stateOf(c), isA<OnboardingAwaitingBackup>());
        expect(sessionOf(c), isNull);

        final n = notifierOf(c);
        final first = n.confirmBackup();
        final second = n.confirmBackup(); // NO await between the taps
        await Future.wait([first, second]);
        await pumpEventQueue();

        expect(
          store.setCount,
          1,
          reason: 'the confirmation true-write must persist exactly once',
        );
        expect(store.lastSetValue, isTrue);
        expect(store.persisted, isTrue, reason: 'durably confirmed');
        expect(stateOf(c), isA<OnboardingActive>());
        expect(
          sessionOf(c),
          same(p.session),
          reason: 'the gate opens exactly once, exposing the one session',
        );
      },
    );

    test('disposing mid-confirm never sets state after dispose (the _disposed '
        'guard after the persist await swallows the continuation)', () async {
      // Built inline WITHOUT addTearDown — this test disposes the container
      // by hand, mid-flight, exactly like the 'disposing mid-probe' case.
      final holdSet = Completer<void>();
      final p = FakeWalletProvisioner(exists: true);
      final store = FakeOnboardingStore(confirmed: false)..holdSet = holdSet;
      final container = ProviderContainer(
        overrides: [
          walletProvisionerProvider.overrideWithValue(p),
          onboardingStoreProvider.overrideWithValue(store),
        ],
      );
      container.listen(onboardingControllerProvider, (_, _) {});
      container.listen(walletSessionProvider, (_, _) {});

      await pumpEventQueue(); // -> awaiting backup
      expect(
        container.read(onboardingControllerProvider),
        isA<OnboardingAwaitingBackup>(),
      );

      // Start the confirm; it synchronously enters Confirming, then blocks on
      // the held persist (state stays Confirming, gate still closed).
      final confirming = container
          .read(onboardingControllerProvider.notifier)
          .confirmBackup();
      await pumpEventQueue();
      expect(
        container.read(onboardingControllerProvider),
        isA<OnboardingConfirming>(),
        reason: 'blocked on the held persist, before the gate opens',
      );

      container.dispose(); // tear down WHILE the persist is in flight
      holdSet.complete(); // the persist resolves AFTER dispose
      await confirming; // the continuation runs; the guard must swallow it
      await pumpEventQueue(); // a post-dispose `state =` would throw here
      // Reaching here without an exception is the assertion: the _disposed
      // guard after the persist await prevented the OnboardingActive set.
    });

    test(
      'disposing mid-create (the gate-reset persist in flight) never sets state '
      'after dispose',
      () async {
        // startCreate\'s FIRST await is the crash-safe false-write. Hold it, so
        // dispose lands while that persist is in flight; the _disposed guard
        // after the await must keep the create continuation from touching the
        // torn-down ref (and createGenerated must never even run).
        final holdSet = Completer<void>();
        final p = FakeWalletProvisioner(exists: false);
        final store = FakeOnboardingStore()..holdSet = holdSet;
        final container = ProviderContainer(
          overrides: [
            walletProvisionerProvider.overrideWithValue(p),
            onboardingStoreProvider.overrideWithValue(store),
          ],
        );
        container.listen(onboardingControllerProvider, (_, _) {});
        container.listen(walletSessionProvider, (_, _) {});

        await pumpEventQueue(); // -> welcome
        expect(
          container.read(onboardingControllerProvider),
          isA<OnboardingWelcome>(),
        );

        // Start the create; it synchronously enters Generating, then blocks on
        // the held gate-reset write (before the wallet is ever written).
        final creating = container
            .read(onboardingControllerProvider.notifier)
            .startCreate();
        await pumpEventQueue();
        expect(
          container.read(onboardingControllerProvider),
          isA<OnboardingGenerating>(),
          reason: 'blocked on the held gate-reset, before createGenerated',
        );
        expect(p.createCount, 0, reason: 'no wallet written yet');

        container
            .dispose(); // tear down WHILE the gate-reset persist is in flight
        holdSet.complete(); // the reset resolves AFTER dispose
        await creating; // the continuation runs; the guard must swallow it
        await pumpEventQueue(); // a post-dispose `state =` would throw here

        // The guard fired right after the gate-reset await: createGenerated was
        // never reached, so no wallet was minted into a torn-down session.
        expect(
          p.createCount,
          0,
          reason: 'the create aborted at the _disposed guard, post-reset',
        );
      },
    );
  });

  // The unstable-mobile-network pass. The build/re-entrancy tests above prove
  // the gate and the at-most-once interlocks; these prove the flow stays HONEST
  // while the network is slow or flapping — the lived reality of onboarding a
  // money wallet over a 3G/edge connection. Two failure shapes the user
  // actually hits: (a) a step is SLOW (the await hasn't resolved yet) — the UI
  // must show a transient spinner state, never a stuck blank that looks frozen;
  // (b) a step FAILS THEN A RETRY SUCCEEDS (the flaky-link normal) — the flow
  // must reach the correct end state and expose a session EXACTLY once, never
  // mid-way. Driven with held `Completer`s (no real delays), so the in-flight
  // window is observed deterministically.
  group('unstable network: slow steps show progress, not a stuck blank', () {
    test('a SLOW boot open() shows OnboardingLoading until it resolves, then '
        'active (a slow network renders a spinner, not nothing)', () async {
      // A confirmed wallet is on disk but open() is in flight over a slow
      // link. The boot probe has passed walletExists() and is blocked on the
      // held open(); the controller MUST sit in the transient Loading (a UI
      // renders its neutral spinner) with NO session exposed — not a blank,
      // not a premature active. Releasing open() resolves to active.
      final hold = Completer<void>();
      final p = FakeWalletProvisioner(exists: true)..holdOpen = hold;
      final c = harness(p, FakeOnboardingStore(confirmed: true));

      // build() is synchronous → Loading immediately, before any await.
      expect(stateOf(c), isA<OnboardingLoading>());
      await pumpEventQueue(); // walletExists() resolves; open() now awaiting.

      // Still Loading — open() has NOT returned. This is the "slow network"
      // window the user stares at: it must be a spinner state, never blank.
      expect(
        stateOf(c),
        isA<OnboardingLoading>(),
        reason: 'open() in flight → still the transient spinner state',
      );
      expect(sessionOf(c), isNull, reason: 'no session while open is pending');

      hold.complete(); // the slow open() finally returns
      await pumpEventQueue();

      expect(
        stateOf(c),
        isA<OnboardingActive>(),
        reason: 'resolves correctly once the slow open completes',
      );
      expect(sessionOf(c), same(p.session));
      expect(p.openCount, 1);
    });

    test(
      'a SLOW createGenerated() shows OnboardingGenerating until it resolves, '
      'then awaiting-backup (no premature deposit, no stuck blank)',
      () async {
        // From welcome the user taps Create; the seed-seal write is slow. The
        // gate-reset persist completes, then createGenerated() blocks. The
        // controller MUST sit in the transient Generating (the "creating your
        // wallet" spinner) with NO session — never blank, never active.
        final hold = Completer<void>();
        final p = FakeWalletProvisioner(exists: false)..holdCreate = hold;
        final c = harness(p, FakeOnboardingStore());
        await pumpEventQueue(); // -> welcome
        expect(stateOf(c), isA<OnboardingWelcome>());

        final creating = notifierOf(c).startCreate();
        await pumpEventQueue(); // gate-reset persisted; createGenerated awaiting.

        // The slow-create window: a transient progress state, not a blank.
        expect(
          stateOf(c),
          isA<OnboardingGenerating>(),
          reason: 'createGenerated() in flight → the Generating spinner',
        );
        expect(
          sessionOf(c),
          isNull,
          reason: 'no session while create is pending',
        );

        hold.complete(); // the slow seed-seal finally returns
        await creating;
        await pumpEventQueue();

        expect(
          stateOf(c),
          isA<OnboardingAwaitingBackup>(),
          reason: 'resolves to forced backup once the slow create completes',
        );
        expect(
          sessionOf(c),
          isNull,
          reason: 'still no deposit before the backup is confirmed',
        );
        expect(p.createCount, 1);
      },
    );

    test('a SLOW confirm persist shows OnboardingConfirming until it resolves, '
        'then active exposing the session exactly once', () async {
      // The user taps "I've backed it up" but the prefs write is slow. The
      // controller MUST sit in the transient Confirming with the gate still
      // CLOSED (no session) until the persist durably lands — only then
      // active. Pins that a slow confirm renders progress, and the deposit
      // gate opens strictly on the completed persist, never on the intent.
      final hold = Completer<void>();
      final store = FakeOnboardingStore(confirmed: false)..holdSet = hold;
      final p = FakeWalletProvisioner(exists: true);
      final c = harness(p, store);
      await pumpEventQueue(); // -> awaiting backup (unconfirmed on disk)
      expect(stateOf(c), isA<OnboardingAwaitingBackup>());

      final confirming = notifierOf(c).confirmBackup();
      await pumpEventQueue(); // synchronously Confirming; blocked on the persist.

      expect(
        stateOf(c),
        isA<OnboardingConfirming>(),
        reason: 'persist in flight → the Confirming spinner, gate still shut',
      );
      expect(
        sessionOf(c),
        isNull,
        reason: 'NO session while the confirmation has not durably landed',
      );

      hold.complete(); // the slow prefs write finally lands
      await confirming;
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingActive>());
      expect(
        store.persisted,
        isTrue,
        reason: 'gate opened on the durable write',
      );
      expect(
        sessionOf(c),
        same(p.session),
        reason: 'the gate opened exactly once, on the completed persist',
      );
    });
  });

  group('unstable network: a transient failure THEN a retry recovers', () {
    test(
      'CREATE fails transiently, then retry re-probes to welcome and a second '
      'create succeeds — the flaky-link create-recovery path (no session '
      'exposed anywhere mid-way)',
      () async {
        // The first Create tap fails before the wallet is written (e.g. the
        // keychain seal hiccupped on a busy device). retry() re-runs the boot
        // PROBE, not startCreate — and because no wallet was written, the probe
        // sees walletExists() == false and lands the user back on Welcome, from
        // which a second Create succeeds. This is the REAL recovery a user does
        // after a flaky failure: tap Retry, then Create again. The session must
        // be null at every observable step until the second create's backup is
        // confirmed.
        final p = FakeWalletProvisioner(exists: false)
          ..failCreate = const WalletApiError(
            code: 'RW-KEY-LOCKED',
            message: 'keystore busy',
            kind: WalletErrorKind.keystoreUnavailable(),
          );
        final store = FakeOnboardingStore();
        final c = harness(p, store);
        await pumpEventQueue(); // -> welcome

        // First create: transient failure. The gate-reset ran; the create-write
        // threw; nothing reached awaiting-backup → Failed(deviceLocked).
        await notifierOf(c).startCreate();
        await pumpEventQueue();
        final failed = stateOf(c);
        expect(failed, isA<OnboardingFailed>());
        expect(
          (failed as OnboardingFailed).kind,
          OnboardingFailureKind.deviceLocked,
        );
        expect(failed.kind.isRetryable, isTrue);
        expect(sessionOf(c), isNull, reason: 'no session on a failed create');
        expect(p.createCount, 1, reason: 'one create was attempted and failed');

        // The user taps Retry. retry() re-probes; the failed create wrote NO
        // wallet, so walletExists() is still false → back to Welcome (NOT a
        // re-create). This is the documented retry semantics for a create fault.
        p.failCreate = null; // the device freed up / keychain settled
        await notifierOf(c).retry();
        await pumpEventQueue();
        expect(
          stateOf(c),
          isA<OnboardingWelcome>(),
          reason:
              'retry re-probes; no wallet on disk → welcome, not a re-create',
        );
        expect(
          p.createCount,
          1,
          reason: 'retry re-PROBES — it does not create',
        );
        expect(sessionOf(c), isNull);

        // The user taps Create again — this time it succeeds → forced backup.
        await notifierOf(c).startCreate();
        await pumpEventQueue();
        expect(
          stateOf(c),
          isA<OnboardingAwaitingBackup>(),
          reason: 'the second create reached the forced-backup holding state',
        );
        expect(p.createCount, 2, reason: 'exactly one successful create now');
        expect(
          sessionOf(c),
          isNull,
          reason: 'STILL no deposit until this backup is confirmed',
        );

        // And the recovery completes through confirm → active, session exposed
        // exactly once, at the very end.
        await notifierOf(c).confirmBackup();
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingActive>());
        expect(
          sessionOf(c),
          same(p.session),
          reason: 'the gate opens once, only after the recovered flow confirms',
        );
      },
    );

    test(
      'CONFIRM persist fails transiently, then a retried confirm succeeds — the '
      'gate opens only on the successful persist and the session is exposed '
      'exactly once (the "prefs hiccup then works" path)',
      () async {
        // The user taps "I've backed it up"; the prefs write fails once (a busy
        // disk / a transient store fault). The gate stays CLOSED and the flow
        // reverts to awaiting-backup so the screen can prompt "couldn't save —
        // try again". The user taps confirm AGAIN; this time the persist lands
        // → active. Distinct from the existing single-failure test: this proves
        // the RETRY reaches deposit-ready and the session is exposed exactly
        // once, on the second (successful) persist — never on the first.
        final p = FakeWalletProvisioner(exists: true);
        final store = FakeOnboardingStore(confirmed: false)
          ..failSet = StateError('prefs write failed');
        final c = harness(p, store);
        await pumpEventQueue(); // -> awaiting backup (unconfirmed on disk)
        expect(stateOf(c), isA<OnboardingAwaitingBackup>());
        expect(sessionOf(c), isNull);

        // First confirm: the persist throws → the action rethrows for the
        // screen, the gate stays shut, and the flow is back at awaiting-backup.
        await expectLater(
          notifierOf(c).confirmBackup(),
          throwsA(isA<StateError>()),
        );
        await pumpEventQueue();
        expect(
          stateOf(c),
          isA<OnboardingAwaitingBackup>(),
          reason: 'a failed persist reverts to awaiting — gate stays closed',
        );
        expect(
          store.persisted,
          isNot(true),
          reason: 'the confirmation never durably landed on the first try',
        );
        expect(sessionOf(c), isNull, reason: 'NO session on a failed confirm');
        expect(
          store.setCount,
          1,
          reason: 'one (failed) persist attempt so far',
        );

        // The store recovers; the user retries confirm. THIS persist lands.
        store.failSet = null;
        await notifierOf(c).confirmBackup();
        await pumpEventQueue();

        expect(
          stateOf(c),
          isA<OnboardingActive>(),
          reason: 'the retried confirm reaches deposit-ready',
        );
        expect(
          store.persisted,
          isTrue,
          reason: 'the gate opened on the SUCCESSFUL second persist',
        );
        expect(
          store.setCount,
          2,
          reason: 'exactly two persist attempts: the failed one + the good one',
        );
        expect(
          sessionOf(c),
          same(p.session),
          reason:
              'the session is exposed exactly once — on the good persist, '
              'never on the failed first attempt',
        );
      },
    );
  });

  // Money-honesty sweep: a session is the deposit capability. It must be null
  // in EVERY non-active state reachable by these flows. The two model walks
  // above assert `session != null iff active` along a rotation; this is the
  // explicit, named roll-call — each transient and each failed/holding state
  // driven to deterministically, with sessionOf asserted null. A future state
  // that accidentally threads a pre-active session to the surface fails the row
  // that reaches it, by name.
  group('money-honesty: no session is exposed in ANY non-active state', () {
    // Each row drives the controller TO the named state and leaves it there
    // (held where the state is transient), so sessionOf can be read in it.
    test(
      'OnboardingLoading (boot probe in flight) exposes no session',
      () async {
        final hold = Completer<void>();
        final p = FakeWalletProvisioner(exists: true)..holdOpen = hold;
        final c = harness(p, FakeOnboardingStore(confirmed: true));
        await pumpEventQueue(); // blocked in open() → still Loading
        expect(stateOf(c), isA<OnboardingLoading>());
        expect(sessionOf(c), isNull);
        hold.complete(); // release so teardown is clean
        await pumpEventQueue();
      },
    );

    test('OnboardingWelcome (no wallet) exposes no session', () async {
      final c = harness(
        FakeWalletProvisioner(exists: false),
        FakeOnboardingStore(),
      );
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingWelcome>());
      expect(sessionOf(c), isNull);
    });

    test(
      'OnboardingGenerating (create in flight) exposes no session',
      () async {
        final hold = Completer<void>();
        final p = FakeWalletProvisioner(exists: false)..holdCreate = hold;
        final c = harness(p, FakeOnboardingStore());
        await pumpEventQueue(); // -> welcome
        final creating = notifierOf(c).startCreate();
        await pumpEventQueue(); // blocked in createGenerated() → Generating
        expect(stateOf(c), isA<OnboardingGenerating>());
        expect(sessionOf(c), isNull);
        hold.complete();
        await creating;
        await pumpEventQueue();
      },
    );

    test(
      'OnboardingAwaitingBackup (provisioned, unconfirmed) exposes no session',
      () async {
        final c = harness(
          FakeWalletProvisioner(exists: true),
          FakeOnboardingStore(confirmed: false),
        );
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingAwaitingBackup>());
        expect(sessionOf(c), isNull);
      },
    );

    test(
      'OnboardingConfirming (persist in flight) exposes no session',
      () async {
        final hold = Completer<void>();
        final store = FakeOnboardingStore(confirmed: false)..holdSet = hold;
        final c = harness(FakeWalletProvisioner(exists: true), store);
        await pumpEventQueue(); // -> awaiting backup
        final confirming = notifierOf(c).confirmBackup();
        await pumpEventQueue(); // blocked in the persist → Confirming
        expect(stateOf(c), isA<OnboardingConfirming>());
        expect(sessionOf(c), isNull);
        hold.complete();
        await confirming;
        await pumpEventQueue();
      },
    );

    test('OnboardingFailed (open threw at boot) exposes no session', () async {
      final p = FakeWalletProvisioner(exists: true)
        ..failOpen = const WalletApiError(
          code: 'RW-KEY-LOCKED',
          message: 'keystore locked',
          kind: WalletErrorKind.keystoreUnavailable(),
        );
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingFailed>());
      expect(
        sessionOf(c),
        isNull,
        reason:
            'a confirmed-on-disk wallet exposes NO session while open '
            'fails — the gate is state-driven, not flag-driven',
      );
    });

    test('OnboardingUnavailable (no backend) exposes no session', () async {
      final container = ProviderContainer(
        overrides: [
          walletProvisionerProvider.overrideWithValue(null),
          onboardingStoreProvider.overrideWithValue(null),
        ],
      );
      addTearDown(container.dispose);
      container.listen(onboardingControllerProvider, (_, _) {});
      container.listen(walletSessionProvider, (_, _) {});
      expect(
        container.read(onboardingControllerProvider),
        isA<OnboardingUnavailable>(),
      );
      expect(container.read(walletSessionProvider), isNull);
    });
  });

  // -------------------------------------------------------------------------
  // RESTORE flow (the inbound recovery-phrase crossing — money-safety gate)
  // -------------------------------------------------------------------------
  group('restore flow', () {
    // A correct but messy phrase (mixed case + extra whitespace) — exactly what
    // a soft keyboard / paste produces; the chokepoint must normalize it. Only
    // three words because the FAKE provisioner does NOT BIP39-validate or
    // length-gate (the length gate lives in the screen, not the controller), so
    // this is enough to exercise the controller's normalize + gate path.
    const messyWords = <String>['  Abandon ', 'ABILITY', 'aBoUt'];

    Future<ProviderContainer> atRestoreInput(
      FakeWalletProvisioner p,
      FakeOnboardingStore store,
    ) async {
      final c = harness(p, store);
      await pumpEventQueue(); // → Welcome
      notifierOf(c).beginRestore();
      return c;
    }

    test(
      'beginRestore from Welcome → RestoreInput (no session, no fault)',
      () async {
        final c = harness(
          FakeWalletProvisioner(exists: false),
          FakeOnboardingStore(),
        );
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingWelcome>());

        notifierOf(c).beginRestore();
        final s = stateOf(c);
        expect(s, isA<OnboardingRestoreInput>());
        expect((s as OnboardingRestoreInput).fault, isNull);
        expect(sessionOf(c), isNull);
      },
    );

    test('beginRestore is guarded to Welcome (a no-op from Active)', () async {
      final p = FakeWalletProvisioner(exists: true);
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingActive>());

      notifierOf(c).beginRestore();
      expect(stateOf(c), isA<OnboardingActive>(), reason: 'no-op off Welcome');
    });

    test('cancelRestore → back to Welcome', () async {
      final c = await atRestoreInput(
        FakeWalletProvisioner(exists: false),
        FakeOnboardingStore(),
      );
      expect(stateOf(c), isA<OnboardingRestoreInput>());

      notifierOf(c).cancelRestore();
      expect(stateOf(c), isA<OnboardingWelcome>());
    });

    test(
      'startRestore happy path → Active, session EXPOSED, gate persisted true '
      'exactly once (false-then-true crash-safe ordering)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final store = FakeOnboardingStore();
        final c = await atRestoreInput(p, store);

        notifierOf(c).startRestore(messyWords);
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingActive>());
        expect(sessionOf(c), same(p.session), reason: 'deposit-ready');
        expect(p.restoreCount, 1);
        // Crash-safe ordering: a false close THEN a true open — two writes.
        expect(store.setCount, 2);
        expect(store.trueSetCount, 1, reason: 'gate opened exactly once');
        expect(store.persisted, isTrue);
      },
    );

    test(
      'startRestore LOWERCASES + trims every word before the SDK (the hard '
      'host-UI contract — a correct backup typed with autocaps must restore)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = await atRestoreInput(p, FakeOnboardingStore());

        notifierOf(c).startRestore(messyWords);
        await pumpEventQueue();

        expect(
          p.lastRestoreWords,
          ['abandon', 'ability', 'about'],
          reason: 'the SDK never sees mis-cased/space-padded words',
        );
      },
    );

    test(
      'startRestore threads the optional creation date to the adapter',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = await atRestoreInput(p, FakeOnboardingStore());
        final created = DateTime.utc(2021, 6);

        notifierOf(
          c,
        ).startRestore(messyWords, approximateCreationTime: created);
        await pumpEventQueue();

        expect(p.lastRestoreCreationTime, created);
      },
    );

    test(
      'a never-completing restore holds in Restoring with NO session',
      () async {
        final p = FakeWalletProvisioner(exists: false)
          ..holdRestore = Completer<void>();
        final c = await atRestoreInput(p, FakeOnboardingStore());

        notifierOf(c).startRestore(messyWords);
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingRestoring>());
        expect(sessionOf(c), isNull, reason: 'never deposit-ready mid-restore');
        p.holdRestore!.complete();
      },
    );

    test(
      'double-tap startRestore restores EXACTLY one wallet (re-entrancy)',
      () async {
        final p = FakeWalletProvisioner(exists: false)
          ..holdRestore = Completer<void>();
        final c = await atRestoreInput(p, FakeOnboardingStore());

        // First tap moves to Restoring (synchronously, before the first await);
        // the second sees state != RestoreInput and no-ops.
        notifierOf(c).startRestore(messyWords);
        notifierOf(c).startRestore(messyWords);
        await pumpEventQueue();
        p.holdRestore!.complete();
        await pumpEventQueue();

        expect(p.restoreCount, 1);
        expect(stateOf(c), isA<OnboardingActive>());
      },
    );

    test(
      'an invalid word returns to RestoreInput with the 1-based index (no '
      'full-screen failure that would lose the phrase); NO session',
      () async {
        final p = FakeWalletProvisioner(exists: false)
          // SDK reports a 0-based index (the 7th word → 6).
          ..failRestore = const WalletApiError(
            code: 'RW-SEED-002',
            message: 'unknown word',
            kind: WalletErrorKind.invalidMnemonic(wordIndex: 6),
          );
        final c = await atRestoreInput(p, FakeOnboardingStore());

        notifierOf(c).startRestore(messyWords);
        await pumpEventQueue();

        final s = stateOf(c);
        expect(s, isA<OnboardingRestoreInput>());
        expect(
          (s as OnboardingRestoreInput).fault,
          RestoreInputFault.invalidWord,
        );
        expect(s.invalidWordIndex, 7, reason: '0-based 6 → human word 7');
        expect(sessionOf(c), isNull);
      },
    );

    test(
      'a checksum failure (no word index) → invalidWord with null index',
      () async {
        final p = FakeWalletProvisioner(exists: false)
          ..failRestore = const WalletApiError(
            code: 'RW-SEED-002',
            message: 'bad checksum',
            kind: WalletErrorKind.invalidMnemonic(),
          );
        final c = await atRestoreInput(p, FakeOnboardingStore());

        notifierOf(c).startRestore(messyWords);
        await pumpEventQueue();

        final s = stateOf(c) as OnboardingRestoreInput;
        expect(s.fault, RestoreInputFault.invalidWord);
        expect(s.invalidWordIndex, isNull);
      },
    );

    test('a too-recent birthday → RestoreInput(birthdayTooRecent)', () async {
      final p = FakeWalletProvisioner(exists: false)
        ..failRestore = const WalletApiError(
          code: 'RW-CFG-002',
          message: 'birthday in future',
          kind: WalletErrorKind.birthdayInFuture(),
        );
      final c = await atRestoreInput(p, FakeOnboardingStore());

      notifierOf(
        c,
      ).startRestore(messyWords, approximateCreationTime: DateTime.utc(2099));
      await pumpEventQueue();

      expect(
        (stateOf(c) as OnboardingRestoreInput).fault,
        RestoreInputFault.birthdayTooRecent,
      );
    });

    test('seedMismatch over a remnant → RestoreInput(seedMismatch)', () async {
      final p = FakeWalletProvisioner(exists: false)
        ..failRestore = const WalletApiError(
          code: 'RW-SEED-005',
          message: 'seed mismatch',
          kind: WalletErrorKind.seedMismatch(),
        );
      final c = await atRestoreInput(p, FakeOnboardingStore());

      notifierOf(c).startRestore(messyWords);
      await pumpEventQueue();

      expect(
        (stateOf(c) as OnboardingRestoreInput).fault,
        RestoreInputFault.seedMismatch,
      );
    });

    test(
      'walletAlreadyExists → RestoreInput(alreadyExists) (defensive arm)',
      () async {
        // Near-unreachable from Welcome (the boot fork routes an existing wallet
        // to open), but classifyRestoreInputFault is a pure total function — pin
        // every arm so a structural error never falls through to a generic screen.
        final p = FakeWalletProvisioner(exists: false)
          ..failRestore = const WalletApiError(
            code: 'RW-STORE-006',
            message: 'wallet already exists',
            kind: WalletErrorKind.walletAlreadyExists(),
          );
        final c = await atRestoreInput(p, FakeOnboardingStore());

        notifierOf(c).startRestore(messyWords);
        await pumpEventQueue();

        expect(
          (stateOf(c) as OnboardingRestoreInput).fault,
          RestoreInputFault.alreadyExists,
        );
        expect(sessionOf(c), isNull);
      },
    );

    test('a PROVISIONING failure (device locked) goes to the generic failed '
        'screen, NOT the restore form', () async {
      final p = FakeWalletProvisioner(exists: false)
        ..failRestore = const WalletApiError(
          code: 'RW-KEY-001',
          message: 'keystore unavailable',
          kind: WalletErrorKind.keystoreUnavailable(),
        );
      final c = await atRestoreInput(p, FakeOnboardingStore());

      notifierOf(c).startRestore(messyWords);
      await pumpEventQueue();

      final s = stateOf(c);
      expect(s, isA<OnboardingFailed>());
      expect((s as OnboardingFailed).kind, OnboardingFailureKind.deviceLocked);
      expect(sessionOf(c), isNull);
    });

    test(
      'restore OK but the confirm-open persist fails → FORCED backup (gate '
      'CLOSED), never deposit-ready on a confirmation that did not survive',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final store = FakeOnboardingStore()
          // The crash-safe `false` close succeeds; only the `true` open fails.
          ..failSetTrue = const WalletApiError(
            code: 'RW-STORE-IO',
            message: 'prefs write failed',
            kind: WalletErrorKind.diskFull(),
          );
        final c = await atRestoreInput(p, store);

        notifierOf(c).startRestore(messyWords);
        await pumpEventQueue();

        expect(p.restoreCount, 1, reason: 'the wallet WAS restored');
        expect(stateOf(c), isA<OnboardingAwaitingBackup>());
        expect(sessionOf(c), isNull, reason: 'gate stays closed');
        expect(store.persisted, isFalse, reason: 'only the false close stuck');
      },
    );

    test(
      'startRestore is guarded to RestoreInput (a no-op from Welcome)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = harness(p, FakeOnboardingStore());
        await pumpEventQueue(); // Welcome, NOT RestoreInput

        notifierOf(c).startRestore(messyWords);
        await pumpEventQueue();

        expect(p.restoreCount, 0);
        expect(stateOf(c), isA<OnboardingWelcome>());
      },
    );
  });

  // #397 §3.7 D5 — the watch-only IMPORT flow. Mirrors the restore group: the
  // deposit-gate discipline (a session reaches the UI iff Active) MUST hold on
  // this new path too, and a fixable input fault (a malformed / wrong-network
  // key) must return to the input form with the pasted key intact.
  group('watch-only import flow', () {
    const ufvk = 'uview1testexportedviewingkeyxxxxxxxxxxxxxxxxxxxxxxxx';
    final created = DateTime.utc(2022, 6);

    Future<ProviderContainer> atWatchOnlyInput(
      FakeWalletProvisioner p,
      FakeOnboardingStore store,
    ) async {
      final c = harness(p, store);
      await pumpEventQueue(); // → Welcome
      notifierOf(c).beginWatchOnly();
      return c;
    }

    test('beginWatchOnly from Welcome → WatchOnlyInput (no session)', () async {
      final c = harness(
        FakeWalletProvisioner(exists: false),
        FakeOnboardingStore(),
      );
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingWelcome>());

      notifierOf(c).beginWatchOnly();
      final s = stateOf(c);
      expect(s, isA<OnboardingWatchOnlyInput>());
      expect((s as OnboardingWatchOnlyInput).fault, isNull);
      expect(sessionOf(c), isNull);
    });

    test('beginWatchOnly is guarded to Welcome (no-op from Active)', () async {
      final c = harness(
        FakeWalletProvisioner(exists: true),
        FakeOnboardingStore(confirmed: true),
      );
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingActive>());
      notifierOf(c).beginWatchOnly();
      expect(stateOf(c), isA<OnboardingActive>());
    });

    test('cancelWatchOnly → back to Welcome', () async {
      final c = await atWatchOnlyInput(
        FakeWalletProvisioner(exists: false),
        FakeOnboardingStore(),
      );
      expect(stateOf(c), isA<OnboardingWatchOnlyInput>());
      notifierOf(c).cancelWatchOnly();
      expect(stateOf(c), isA<OnboardingWelcome>());
    });

    test(
      'startWatchOnly happy path → Active, session EXPOSED + watch-only, gate '
      'persisted true (no backup step), key + birthday threaded',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final store = FakeOnboardingStore();
        final c = await atWatchOnlyInput(p, store);

        notifierOf(c).startWatchOnly(ufvk, creationDate: created);
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingActive>());
        expect(sessionOf(c), same(p.session));
        // The pasted key + the picker's estimated birthday reached the adapter.
        expect(p.lastWatchOnlyUfvk, ufvk);
        expect(
          p.lastWatchOnlyBirthdayHeight,
          FakeWalletProvisioner.estimateHeightFor(created),
        );
        // A watch-only wallet has NOTHING to back up — the gate is marked
        // confirmed so a reboot opens straight to Active.
        expect(store.persisted, isTrue);
        // The session reports watch-only (the chrome key).
        expect(await sessionOf(c)!.isWatchOnly(), isTrue);
        // #397 P1a — the KIND is captured SYNCHRONOUSLY on OnboardingActive (no
        // boot read needed here, it was just created), so isWatchOnlyProvider
        // resolves the chrome on the first active frame.
        expect((stateOf(c) as OnboardingActive).isWatchOnly, isTrue);
      },
    );

    test(
      '#397 P1a — isWatchOnlyProvider reads the kind SYNCHRONOUSLY off '
      'OnboardingActive (no async settle → no spend-affordance flash)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());
        notifierOf(c).startWatchOnly(ufvk, creationDate: created);
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingActive>());
        // A plain synchronous read — the provider resolves true from the captured
        // kind on OnboardingActive, never waiting on a FutureProvider to settle
        // (the flash the chrome exhibited before this fix).
        expect(c.read(isWatchOnlyProvider), isTrue);
      },
    );

    test('#397 P1a — a boot-open of a confirmed WATCH-ONLY wallet captures the '
        'kind (Active.isWatchOnly true, read once at boot)', () async {
      final p = FakeWalletProvisioner(
        exists: true,
        session: FakeWalletSession()..isWatchOnlyResult = true,
      );
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue();

      final s = stateOf(c);
      expect(s, isA<OnboardingActive>());
      expect((s as OnboardingActive).isWatchOnly, isTrue);
    });

    test('#397 P1a — a boot-open of a confirmed SEED wallet is NOT watch-only '
        '(Active.isWatchOnly false)', () async {
      // Default fake session: isWatchOnlyResult false, birthday Some.
      final c = harness(
        FakeWalletProvisioner(exists: true),
        FakeOnboardingStore(confirmed: true),
      );
      await pumpEventQueue();

      final s = stateOf(c);
      expect(s, isA<OnboardingActive>());
      expect((s as OnboardingActive).isWatchOnly, isFalse);
    });

    test('#397 P1b — a boot-open of the account-less watch-only CRASH REMNANT '
        '(watch-only kind, NULL birthday) WIPES the incomplete store and routes '
        'to a clean re-import (never a stranded broken Active)', () async {
      // The remnant: createWatchOnly provisioned the store Complete but crashed
      // before importing the account, so is_watch_only() reads true (manifest
      // kind) while birthdayHeight() returns null (no account row). No seed, no
      // money — the boot fork must wipe + re-import.
      final p = FakeWalletProvisioner(
        exists: true,
        session: FakeWalletSession()
          ..isWatchOnlyResult = true
          ..birthdayHeightResult = null,
      );
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue();

      // Wiped exactly once, and routed to the import form (not a broken Active).
      expect(p.deleteCount, 1);
      expect(p.exists, isFalse); // walletExists reset → clean re-import precond
      final s = stateOf(c);
      expect(s, isA<OnboardingWatchOnlyInput>());
      expect(sessionOf(c), isNull); // the broken session is never exposed
    });

    test(
      '#397 P1b — a healthy watch-only wallet (kind true, birthday SOME) is '
      'NEVER wiped as a remnant (the null-birthday signal is precise)',
      () async {
        final p = FakeWalletProvisioner(
          exists: true,
          session: FakeWalletSession()
            ..isWatchOnlyResult = true
            ..birthdayHeightResult = 2400000,
        );
        final c = harness(p, FakeOnboardingStore(confirmed: true));
        await pumpEventQueue();

        expect(p.deleteCount, 0); // a healthy wallet is never wiped
        expect(stateOf(c), isA<OnboardingActive>());
        expect((stateOf(c) as OnboardingActive).isWatchOnly, isTrue);
      },
    );

    test(
      '#397 P1b — a transient birthday-read FAULT on a watch-only wallet is '
      'NOT treated as a missing account (never wipe on a read blip)',
      () async {
        final p = FakeWalletProvisioner(
          exists: true,
          session: FakeWalletSession()
            ..isWatchOnlyResult = true
            ..birthdayHeightThrows = const WalletApiError(
              code: 'RW-STORE-006',
              message: 'store busy',
              kind: WalletErrorKind.storeBusy(),
            ),
        );
        final c = harness(p, FakeOnboardingStore(confirmed: true));
        await pumpEventQueue();

        // A throw is not a null — the wallet is treated as provisioned (Active),
        // never wiped. If it truly was the remnant, its faults resurface later.
        expect(p.deleteCount, 0);
        expect(stateOf(c), isA<OnboardingActive>());
        expect((stateOf(c) as OnboardingActive).isWatchOnly, isTrue);
      },
    );

    test('#397 P1b — a WIPE fault on the remnant surfaces the honest failure '
        '(retry re-probes), never a stranded broken Active', () async {
      final p =
          FakeWalletProvisioner(
              exists: true,
              session: FakeWalletSession()
                ..isWatchOnlyResult = true
                ..birthdayHeightResult = null,
            )
            ..failDelete = const WalletApiError(
              code: 'RW-KEY-001',
              message: 'keystore unavailable',
              kind: WalletErrorKind.keystoreUnavailable(),
            );
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue();

      final s = stateOf(c);
      expect(s, isA<OnboardingFailed>());
      expect((s as OnboardingFailed).kind, OnboardingFailureKind.deviceLocked);
    });

    test(
      'a blank key never reaches the SDK — stays on the input form',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());

        notifierOf(c).startWatchOnly('   ', creationDate: created);
        await pumpEventQueue();

        expect(p.createWatchOnlyCount, 0);
        final s = stateOf(c);
        expect(s, isA<OnboardingWatchOnlyInput>());
        expect(
          (s as OnboardingWatchOnlyInput).fault,
          WatchOnlyInputFault.invalidViewingKey,
        );
      },
    );

    test(
      'a malformed key → WatchOnlyInput(invalidViewingKey), NO session',
      () async {
        final p = FakeWalletProvisioner(exists: false)
          ..failCreateWatchOnly = const WalletApiError(
            code: 'RW-VIEW-002',
            message: 'invalid viewing key',
            kind: WalletErrorKind.invalidViewingKey(),
          );
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());

        notifierOf(c).startWatchOnly(ufvk, creationDate: created);
        await pumpEventQueue();

        final s = stateOf(c);
        expect(s, isA<OnboardingWatchOnlyInput>());
        expect(
          (s as OnboardingWatchOnlyInput).fault,
          WatchOnlyInputFault.invalidViewingKey,
        );
        expect(sessionOf(c), isNull);
      },
    );

    test('a wrong-network key → WatchOnlyInput(networkMismatch)', () async {
      final p = FakeWalletProvisioner(exists: false)
        ..failCreateWatchOnly = const WalletApiError(
          code: 'RW-CFG-004',
          message: 'network mismatch',
          kind: WalletErrorKind.networkMismatch(),
        );
      final c = await atWatchOnlyInput(p, FakeOnboardingStore());

      notifierOf(c).startWatchOnly(ufvk, creationDate: created);
      await pumpEventQueue();

      expect(
        (stateOf(c) as OnboardingWatchOnlyInput).fault,
        WatchOnlyInputFault.networkMismatch,
      );
    });

    test(
      'a PROVISIONING failure (device locked) → the generic failed screen',
      () async {
        final p = FakeWalletProvisioner(exists: false)
          ..failCreateWatchOnly = const WalletApiError(
            code: 'RW-KEY-001',
            message: 'keystore unavailable',
            kind: WalletErrorKind.keystoreUnavailable(),
          );
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());

        notifierOf(c).startWatchOnly(ufvk, creationDate: created);
        await pumpEventQueue();

        final s = stateOf(c);
        expect(s, isA<OnboardingFailed>());
        expect(
          (s as OnboardingFailed).kind,
          OnboardingFailureKind.deviceLocked,
        );
        expect(sessionOf(c), isNull);
      },
    );

    test(
      'never deposit-ready mid-create (session null in CreatingWatchOnly)',
      () async {
        final p = FakeWalletProvisioner(exists: false)
          ..holdCreateWatchOnly = Completer<void>();
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());

        notifierOf(c).startWatchOnly(ufvk, creationDate: created);
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingCreatingWatchOnly>());
        expect(sessionOf(c), isNull);
        p.holdCreateWatchOnly!.complete();
      },
    );

    test('double-tap creates EXACTLY one wallet (re-entrancy)', () async {
      final p = FakeWalletProvisioner(exists: false)
        ..holdCreateWatchOnly = Completer<void>();
      final c = await atWatchOnlyInput(p, FakeOnboardingStore());

      notifierOf(c).startWatchOnly(ufvk, creationDate: created);
      notifierOf(c).startWatchOnly(ufvk, creationDate: created);
      await pumpEventQueue();
      p.holdCreateWatchOnly!.complete();
      await pumpEventQueue();

      expect(p.createWatchOnlyCount, 1);
      expect(stateOf(c), isA<OnboardingActive>());
    });

    test(
      'startWatchOnly is guarded to WatchOnlyInput (no-op from Welcome)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = harness(p, FakeOnboardingStore());
        await pumpEventQueue(); // Welcome, NOT WatchOnlyInput

        notifierOf(c).startWatchOnly(ufvk, creationDate: created);
        await pumpEventQueue();

        expect(p.createWatchOnlyCount, 0);
        expect(stateOf(c), isA<OnboardingWelcome>());
      },
    );

    test(
      '#397 P3 UX-L3 — internal whitespace in a pasted key is STRIPPED before '
      'the SDK decode (a line-wrapped PDF/email paste is rescued)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());

        // The same key, mangled the way a wrapped source pastes it: leading/
        // trailing space + embedded newline + embedded spaces + a zero-width
        // space (the PDF/rich-text contaminant a bare `\s` would miss).
        final mangled =
            '  ${ufvk.substring(0, 12)}\n${ufvk.substring(12, 30)} \u200B'
            '${ufvk.substring(30)}  ';
        notifierOf(c).startWatchOnly(mangled, creationDate: created);
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingActive>());
        expect(p.lastWatchOnlyUfvk, ufvk); // whitespace-free at the adapter
      },
    );

    test(
      '#397 P3 UX-L3 — a WHITESPACE-ONLY paste (spaces + newlines) still never '
      'reaches the SDK',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());

        notifierOf(c).startWatchOnly(' \n \t ', creationDate: created);
        await pumpEventQueue();

        expect(p.createWatchOnlyCount, 0);
        expect(
          (stateOf(c) as OnboardingWatchOnlyInput).fault,
          WatchOnlyInputFault.invalidViewingKey,
        );
      },
    );

    test(
      '#397 S231 F1 — an ALL-UPPERCASE key is CASE-FOLDED at the chokepoint so '
      'a PASTE gets the identical treatment the scan handler used to give only '
      'itself (bech32 is single-case; folding is identity on a valid key)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());

        // A QR-alphanumeric-mode / uppercase-copied paste of the same key.
        notifierOf(c).startWatchOnly(ufvk.toUpperCase(), creationDate: created);
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingActive>());
        expect(p.lastWatchOnlyUfvk, ufvk); // lower-cased at the adapter
      },
    );

    test(
      '#397 S231 F1 — a MIXED-case key is left UNTOUCHED (only an all-caps key '
      'is unambiguously foldable; mixed case is the SDK\'s honest reject)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());

        // Flip one char to upper — no longer all-caps, so no fold.
        final mixed = 'U${ufvk.substring(1)}';
        notifierOf(c).startWatchOnly(mixed, creationDate: created);
        await pumpEventQueue();

        expect(p.lastWatchOnlyUfvk, mixed); // reaches the SDK verbatim
      },
    );

    test(
      '#397 S232 — a paste laced with invisible Unicode format chars (soft '
      'hyphen U+00AD, word joiner U+2060) is STRIPPED before the SDK decode '
      '(a hyphenated PDF / rich-text paste is rescued, not falsely rejected)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = await atWatchOnlyInput(p, FakeOnboardingStore());

        // The classic line-wrapped-PDF contaminant: a soft hyphen at the wrap
        // point + a word joiner mid-string — both invisible, neither valid
        // bech32, both missed by a bare `\s`.
        final laced =
            '${ufvk.substring(0, 20)}­${ufvk.substring(20, 35)}'
            '⁠${ufvk.substring(35)}';
        notifierOf(c).startWatchOnly(laced, creationDate: created);
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingActive>());
        expect(p.lastWatchOnlyUfvk, ufvk); // format-char-free at the adapter
      },
    );

    test('#397 S232 — a watch-only import CLOBBERS the post-restore note to '
        'notApplicable (a `pending` a prior restored wallet left in the GLOBAL '
        'note key must never surface over a watch-only wallet)', () async {
      final p = FakeWalletProvisioner(exists: false);
      // Model a prior restored wallet's residue in the global note key.
      final store = FakeOnboardingStore()
        ..deepScanNote = DeepScanRestoreNoteState.pending;
      final c = await atWatchOnlyInput(p, store);

      notifierOf(c).startWatchOnly(ufvk, creationDate: created);
      await pumpEventQueue();

      expect(stateOf(c), isA<OnboardingActive>());
      expect(store.deepScanNote, DeepScanRestoreNoteState.notApplicable);
    });

    test(
      '#397 P3 security-H3 — a FAILED create resets the pre-create true write '
      '(a persisted true only ever describes a wallet that exists)',
      () async {
        final p = FakeWalletProvisioner(exists: false)
          ..failCreateWatchOnly = const WalletApiError(
            code: 'RW-VIEW-002',
            message: 'invalid viewing key',
            kind: WalletErrorKind.invalidViewingKey(),
          );
        final store = FakeOnboardingStore();
        final c = await atWatchOnlyInput(p, store);

        notifierOf(c).startWatchOnly(ufvk, creationDate: created);
        await pumpEventQueue();

        // Back on the form with the fault — and the gate flag rolled back.
        expect(stateOf(c), isA<OnboardingWatchOnlyInput>());
        expect(store.persisted, isFalse);
        expect(store.lastSetValue, isFalse);
      },
    );

    test(
      '#397 security-H3 (S229) — the flag rollback is UNCONDITIONAL, '
      'including on alreadyExists: the GLOBAL flag may describe a '
      'cross-process UNCONFIRMED seed sibling, and a stale true would boot '
      'it past the forced-backup gate — reset is the money-safe direction '
      '(worst case re-forces a done backup, annoying never dangerous)',
      () async {
        final p = FakeWalletProvisioner(exists: false)
          ..failCreateWatchOnly = const WalletApiError(
            code: 'RW-STORE-006',
            message: 'wallet already exists',
            kind: WalletErrorKind.walletAlreadyExists(),
          );
        final store = FakeOnboardingStore();
        final c = await atWatchOnlyInput(p, store);

        notifierOf(c).startWatchOnly(ufvk, creationDate: created);
        await pumpEventQueue();

        expect(
          (stateOf(c) as OnboardingWatchOnlyInput).fault,
          WatchOnlyInputFault.alreadyExists,
        );
        expect(store.persisted, isFalse);
        expect(store.lastSetValue, isFalse);
      },
    );

    test('#397 P3 security-H3 — the backup gate is a SEED gate: a healthy '
        'watch-only wallet with a LOST confirmed flag (reset prefs over an '
        'intact DB) boots straight to Active, never the forced-backup screen '
        '(whose reveal would fault SeedRequired)', () async {
      final p = FakeWalletProvisioner(
        exists: true,
        session: FakeWalletSession()
          ..isWatchOnlyResult = true
          ..birthdayHeightResult = 2400000,
      );
      // confirmed ABSENT — the silent prefs-loss scenario.
      final c = harness(p, FakeOnboardingStore());
      await pumpEventQueue();

      final s = stateOf(c);
      expect(s, isA<OnboardingActive>());
      expect((s as OnboardingActive).isWatchOnly, isTrue);
    });

    test(
      '#397 P3 security-H3 — the kind bypass does NOT weaken the seed gate: an '
      'unconfirmed SEED wallet still resumes into forced backup',
      () async {
        final c = harness(
          FakeWalletProvisioner(exists: true), // default session: seed kind
          FakeOnboardingStore(), // confirmed absent
        );
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingAwaitingBackup>());
        expect(sessionOf(c), isNull); // never deposit-ready before confirm
      },
    );

    test(
      '#397 P3 — the account-less REMNANT is wiped-and-rerouted even with the '
      'confirmed flag lost (the P1b detector runs before the backup gate)',
      () async {
        final p = FakeWalletProvisioner(
          exists: true,
          session: FakeWalletSession()
            ..isWatchOnlyResult = true
            ..birthdayHeightResult = null,
        );
        final c = harness(p, FakeOnboardingStore()); // confirmed absent
        await pumpEventQueue();

        expect(p.deleteCount, 1);
        expect(stateOf(c), isA<OnboardingWatchOnlyInput>());
      },
    );
  });

  // The rescan-recovery session lifecycle (FR-1b / ADR-0534). A rescan on an
  // ACTIVE wallet rebuilds the data DB at an earlier birthday and swaps a fresh
  // session into the gate; funds are SAFE in every outcome (the seed seal + prior
  // data DB are intact). These pin the four outcomes + the single-flight + the
  // money-safety invariant that the gate never closes on a recoverable fault.
  group('rescanActiveWallet (FR-1b)', () {
    // A confirmed wallet, resumed to Active — the only state a rescan runs from.
    Future<ProviderContainer> atActive(FakeWalletProvisioner p) async {
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingActive>(), reason: 'precondition');
      return c;
    }

    test(
      'SUCCESS swaps a FRESH session into the gate (graph-rebuild trigger)',
      () async {
        final rebuilt = FakeWalletSession();
        final p = FakeWalletProvisioner(exists: true)..rescanSession = rebuilt;
        final original = p.session;
        final c = await atActive(p);

        final pickedDate = DateTime(2023, 3, 15);
        final outcome = await notifierOf(
          c,
        ).rescanActiveWallet(RescanFromTime(pickedDate));

        expect(outcome, RescanOutcome.success);
        expect(p.rescanCount, 1);
        expect(
          (p.lastRescanTarget as RescanFromTime?)?.earliestTime,
          pickedDate,
          reason: 'the picked date threads to the adapter as its sealed arm',
        );
        expect(
          stateOf(c),
          isA<OnboardingActive>(),
          reason: 'the gate STAYS open across a rescan',
        );
        // The session IDENTITY changed to the rebuilt one — this is what rebuilds
        // the whole live-sync graph (sync re-subscribes from the lower birthday;
        // balance/history re-read against the now-empty, repopulating DB).
        expect(sessionOf(c), same(rebuilt));
        expect(sessionOf(c), isNot(same(original)));
      },
    );

    test('SCAN-ALL threads the all-history arm to the adapter', () async {
      final p = FakeWalletProvisioner(exists: true);
      final c = await atActive(p);

      final outcome = await notifierOf(
        c,
      ).rescanActiveWallet(const RescanAllHistory());

      expect(outcome, RescanOutcome.success);
      expect(p.rescanCount, 1);
      expect(
        p.lastRescanTarget,
        isA<RescanAllHistory>(),
        reason: 'all-history reaches the adapter un-reinterpreted',
      );
    });

    test('a rescan FAULT recovers by RE-OPENING — gate stays open at the prior '
        'birthday (no funds lost)', () async {
      final p = FakeWalletProvisioner(exists: true)
        ..failRescan = const WalletApiError(
          code: 'RW-STORE-004',
          message: 'rescan rebuild failed',
          kind: WalletErrorKind.storeCorrupt(),
        );
      final c = await atActive(p);
      final openCountBefore = p.openCount;

      final outcome = await notifierOf(
        c,
      ).rescanActiveWallet(const RescanAllHistory());

      expect(outcome, RescanOutcome.failedRecovered);
      expect(p.rescanCount, 1);
      expect(p.openCount, openCountBefore + 1, reason: 're-opened to recover');
      expect(
        stateOf(c),
        isA<OnboardingActive>(),
        reason: 'the wallet stays usable — the rescan simply did not take',
      );
      expect(
        sessionOf(c),
        same(p.session),
        reason: 're-opened session is the active one',
      );
    });

    test(
      'the SDK in-flight-send fence maps to blockedBySettlingSend — same '
      'recover-by-reopen, distinct outcome (never the "moment" copy)',
      () async {
        final p = FakeWalletProvisioner(exists: true)
          ..failRescan = const WalletApiError(
            code: 'RW-LIFE-006',
            message: 'rescan refused while a send is in flight',
            kind: WalletErrorKind.rescanWithInFlightSend(),
          );
        final c = await atActive(p);
        final openCountBefore = p.openCount;

        final outcome = await notifierOf(
          c,
        ).rescanActiveWallet(const RescanAllHistory());

        expect(outcome, RescanOutcome.blockedBySettlingSend);
        expect(
          p.openCount,
          openCountBefore + 1,
          reason: 're-opened to recover',
        );
        expect(
          stateOf(c),
          isA<OnboardingActive>(),
          reason: 'the wallet stays usable while the send settles',
        );
      },
    );

    test('the SDK disk-full fault maps to failedNeedsSpace — same '
        'recover-by-reopen, distinct outcome (never the "moment" copy on a '
        'full disk, #375)', () async {
      final p = FakeWalletProvisioner(exists: true)
        ..failRescan = const WalletApiError(
          code: 'RW-STORE-005',
          message: 'not enough space to rebuild',
          kind: WalletErrorKind.diskFull(),
        );
      final c = await atActive(p);
      final openCountBefore = p.openCount;

      final outcome = await notifierOf(
        c,
      ).rescanActiveWallet(const RescanAllHistory());

      expect(outcome, RescanOutcome.failedNeedsSpace);
      expect(p.openCount, openCountBefore + 1, reason: 're-opened to recover');
      expect(
        stateOf(c),
        isA<OnboardingActive>(),
        reason: 'the wallet stays usable at its prior state',
      );
    });

    test('a typed StoreBusy deliberately FOLDS into failedRecovered — the one '
        'rescan fault whose honest copy IS "try again in a moment" (#375 '
        'taxonomy pin)', () async {
      final p = FakeWalletProvisioner(exists: true)
        ..failRescan = const WalletApiError(
          code: 'RW-STORE-007',
          message: 'store busy',
          kind: WalletErrorKind.storeBusy(),
        );
      final c = await atActive(p);

      final outcome = await notifierOf(
        c,
      ).rescanActiveWallet(const RescanAllHistory());

      expect(outcome, RescanOutcome.failedRecovered);
      expect(stateOf(c), isA<OnboardingActive>());
    });

    test('a rescan fault AND a failed re-open routes to OnboardingFailed '
        '(closed)', () async {
      final p = FakeWalletProvisioner(exists: true);
      final c = await atActive(p); // boot open must succeed FIRST → Active
      // Only NOW make both the rescan and the recovery re-open fail.
      p
        ..failRescan = const WalletApiError(
          code: 'RW-STORE-004',
          message: 'rescan rebuild failed',
          kind: WalletErrorKind.storeCorrupt(),
        )
        ..failOpen = const WalletApiError(
          code: 'RW-KEYSTORE',
          message: 'device locked',
          kind: WalletErrorKind.keystoreUnavailable(),
        );

      final outcome = await notifierOf(
        c,
      ).rescanActiveWallet(const RescanAllHistory());

      expect(outcome, RescanOutcome.failedClosed);
      expect(stateOf(c), isA<OnboardingFailed>());
      expect(sessionOf(c), isNull, reason: 'no usable wallet — failed surface');
    });

    test('a rescan from a NON-active state is a guarded no-op', () async {
      final p = FakeWalletProvisioner(exists: false);
      final c = harness(p, FakeOnboardingStore());
      await pumpEventQueue(); // Welcome, NOT active

      final outcome = await notifierOf(
        c,
      ).rescanActiveWallet(const RescanAllHistory());

      expect(outcome, RescanOutcome.notActive);
      expect(p.rescanCount, 0);
      expect(stateOf(c), isA<OnboardingWelcome>());
    });

    test(
      'a concurrent rescan is single-flighted (one rebuild, not two)',
      () async {
        final p = FakeWalletProvisioner(exists: true)
          ..holdRescan = Completer<void>();
        final c = await atActive(p);

        // Fire two rescans before the first settles (the held rebuild is in flight).
        final first = notifierOf(
          c,
        ).rescanActiveWallet(const RescanAllHistory());
        final second = notifierOf(
          c,
        ).rescanActiveWallet(const RescanAllHistory());
        await pumpEventQueue();

        // The second saw the in-flight latch and no-opped (notActive); only ONE
        // rebuild ever ran — a rescan must never run twice on one DB.
        expect(await second, RescanOutcome.notActive);
        expect(p.rescanCount, 1);

        p.holdRescan!.complete();
        expect(await first, RescanOutcome.success);
      },
    );
  });

  group('deleteWallet (crypto-shred FR-14)', () {
    test(
      'shred from active resets to welcome, clears the flag, drops the session',
      () async {
        final p = FakeWalletProvisioner(exists: true);
        final store = FakeOnboardingStore(confirmed: true);
        final c = harness(p, store);
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingActive>());

        final outcome = await notifierOf(c).deleteWallet();
        await pumpEventQueue();

        expect(outcome, WalletDeletionOutcome.shredded);
        expect(
          p.deleteCount,
          1,
          reason: 'the authoritative close-then-wipe ran once',
        );
        // The surface reset to onboarding and the deposit gate CLOSED (no session).
        expect(stateOf(c), isA<OnboardingWelcome>());
        expect(sessionOf(c), isNull);
        // The host backup-confirmed flag was cleared (best-effort hygiene).
        expect(store.persisted, isFalse);
      },
    );

    test(
      'shred from awaiting-backup (an unconfirmed wallet) also works',
      () async {
        final p = FakeWalletProvisioner(exists: true);
        final c = harness(p, FakeOnboardingStore(confirmed: false));
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingAwaitingBackup>());

        final outcome = await notifierOf(c).deleteWallet();
        await pumpEventQueue();

        expect(outcome, WalletDeletionOutcome.shredded);
        expect(stateOf(c), isA<OnboardingWelcome>());
      },
    );

    test('a wipe FAULT recovers the live wallet — never a false delete', () async {
      final p = FakeWalletProvisioner(exists: true)
        // The keychain-first wipe deletes nothing on a fault; the fake models
        // that by NOT flipping `exists`. The controller must recover by reopening.
        ..failDelete = const WalletApiError(
          code: 'RW-KS-001',
          message: 'device locked',
          kind: WalletErrorKind.keystoreUnavailable(),
        );
      final store = FakeOnboardingStore(confirmed: true);
      final c = harness(p, store);
      await pumpEventQueue();
      expect(stateOf(c), isA<OnboardingActive>());

      final outcome = await notifierOf(c).deleteWallet();
      await pumpEventQueue();

      expect(outcome, WalletDeletionOutcome.failedRecovered);
      // The wallet is STILL usable (re-opened), never shown as deleted.
      expect(stateOf(c), isA<OnboardingActive>());
      expect(sessionOf(c), isNotNull);
      expect(
        p.openCount,
        greaterThanOrEqualTo(1),
        reason: 'recovered by reopening',
      );
      // The wallet was NOT erased, so the backup-confirmed flag must stay TRUE —
      // a recovered (still-funded) wallet must not be forced back into backup.
      expect(
        store.persisted,
        isTrue,
        reason: 'a faulted wipe must not clear the backup flag',
      );
    });

    test('a wipe fault AND a re-open fault routes to OnboardingFailed', () async {
      final p = FakeWalletProvisioner(exists: true)
        ..failDelete = const WalletApiError(
          code: 'RW-KS-001',
          message: 'device locked',
          kind: WalletErrorKind.keystoreUnavailable(),
        );
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue();
      // The boot open SUCCEEDS (Active); only AFTER that do we wedge open() so the
      // delete's recovery re-open is the thing that fails (the device re-locked
      // between the boot and the delete).
      expect(stateOf(c), isA<OnboardingActive>());
      p.failOpen = const WalletApiError(
        code: 'RW-KS-001',
        message: 'still locked',
        kind: WalletErrorKind.keystoreUnavailable(),
      );

      final outcome = await notifierOf(c).deleteWallet();
      await pumpEventQueue();

      expect(outcome, WalletDeletionOutcome.failedClosed);
      expect(stateOf(c), isA<OnboardingFailed>());
    });

    test(
      'delete from welcome (no wallet) is a typed no-op, nothing wiped',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = harness(p, FakeOnboardingStore());
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingWelcome>());

        final outcome = await notifierOf(c).deleteWallet();
        await pumpEventQueue();

        expect(outcome, WalletDeletionOutcome.notDeletable);
        expect(p.deleteCount, 0, reason: 'no wallet ⇒ never call wipe');
        expect(stateOf(c), isA<OnboardingWelcome>());
      },
    );

    test(
      'a double-tap is single-flighted — the wallet is shredded at most once',
      () async {
        final p = FakeWalletProvisioner(exists: true);
        final c = harness(p, FakeOnboardingStore(confirmed: true));
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingActive>());

        // Fire two deletes before the first settles. The `_deletionInFlight` latch
        // is set synchronously before the first await, so the second no-ops.
        final first = notifierOf(c).deleteWallet();
        final second = notifierOf(c).deleteWallet();
        await pumpEventQueue();

        expect(await second, WalletDeletionOutcome.notDeletable);
        expect(await first, WalletDeletionOutcome.shredded);
        expect(
          p.deleteCount,
          1,
          reason: 'never two close-then-wipe sequences on one wallet',
        );
      },
    );

    test(
      'a rescan in flight REFUSES a concurrent delete (mutual exclusion)',
      () async {
        final p = FakeWalletProvisioner(exists: true)
          ..holdRescan = Completer<void>();
        final c = harness(p, FakeOnboardingStore(confirmed: true));
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingActive>());

        // A rescan is in flight (state stays Active during the rebuild).
        final rescan = notifierOf(
          c,
        ).rescanActiveWallet(const RescanAllHistory());
        await pumpEventQueue();
        // A delete now must NOT proceed — else it would close the handle under the
        // rescan and the rescan's late state write would clobber the result.
        final outcome = await notifierOf(c).deleteWallet();
        expect(outcome, WalletDeletionOutcome.notDeletable);
        expect(
          p.deleteCount,
          0,
          reason: 'no wipe while a rescan holds the handle',
        );

        p.holdRescan!.complete();
        expect(await rescan, RescanOutcome.success);
      },
    );

    test(
      'a delete in flight REFUSES a concurrent rescan (mutual exclusion)',
      () async {
        final p = FakeWalletProvisioner(exists: true)
          ..holdDelete = Completer<void>();
        final c = harness(p, FakeOnboardingStore(confirmed: true));
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingActive>());

        // A delete is in flight (close+wipe held mid-sequence).
        final delete = notifierOf(c).deleteWallet();
        await pumpEventQueue();
        // A rescan now must NOT start over a wallet being erased.
        final rescan = await notifierOf(
          c,
        ).rescanActiveWallet(const RescanAllHistory());
        expect(rescan, RescanOutcome.notActive);
        expect(
          p.rescanCount,
          0,
          reason: 'no rescan while a delete holds the handle',
        );

        p.holdDelete!.complete();
        expect(await delete, WalletDeletionOutcome.shredded);
      },
    );
  });

  group('recoverByRestore — the #251 escape from a non-retryable failure', () {
    // Reach OnboardingFailed(needsRecovery): a wallet on disk whose open() fails
    // with a damaged-data kind. From here the user MUST have a way out (their funds
    // are phrase-recoverable; a money app may never trap them).
    Future<ProviderContainer> reachNeedsRecovery(
      FakeWalletProvisioner p,
    ) async {
      final c = harness(p, FakeOnboardingStore(confirmed: true));
      await pumpEventQueue();
      final st = stateOf(c);
      expect(st, isA<OnboardingFailed>());
      expect(
        (st as OnboardingFailed).kind,
        OnboardingFailureKind.needsRecovery,
      );
      return c;
    }

    test(
      'plain wipe succeeds → routes to the restore form, never force',
      () async {
        final p = FakeWalletProvisioner(exists: true)
          ..failOpen = const WalletApiError(
            code: 'RW',
            message: 'damaged',
            kind: WalletErrorKind.keystoreInconsistent(
              permanentlyInvalidated: false,
            ),
          );
        final c = await reachNeedsRecovery(p);

        await notifierOf(c).recoverByRestore();
        await pumpEventQueue();

        expect(
          stateOf(c),
          isA<OnboardingRestoreInput>(),
          reason:
              'a non-retryable failure is NEVER a dead-end — it routes to restore',
        );
        expect(
          p.deleteCount,
          1,
          reason: 'plain wipe is tried FIRST (never auto-force)',
        );
        expect(
          p.forceDeleteCount,
          0,
          reason: 'no force when the plain wipe succeeds',
        );
      },
    );

    test(
      'plain wipe reports keystoreInconsistent → DELIBERATE force, then restore',
      () async {
        final p = FakeWalletProvisioner(exists: true)
          ..failOpen = const WalletApiError(
            code: 'RW',
            message: 'damaged',
            kind: WalletErrorKind.keystoreInconsistent(
              permanentlyInvalidated: false,
            ),
          )
          ..failDelete = const WalletApiError(
            code: 'RW',
            message: 'key gone',
            kind: WalletErrorKind.keystoreInconsistent(
              permanentlyInvalidated: false,
            ),
          );
        final c = await reachNeedsRecovery(p);

        await notifierOf(c).recoverByRestore();
        await pumpEventQueue();

        expect(p.deleteCount, 1, reason: 'plain wipe tried FIRST');
        expect(
          p.forceDeleteCount,
          1,
          reason:
              'force escalates ONLY on keystoreInconsistent (the key is genuinely gone)',
        );
        expect(stateOf(c), isA<OnboardingRestoreInput>());
      },
    );

    test(
      'a non-keystoreInconsistent wipe fault does NOT force — re-probes (still escapable)',
      () async {
        final p = FakeWalletProvisioner(exists: true)
          ..failOpen = const WalletApiError(
            code: 'RW',
            message: 'damaged',
            kind: WalletErrorKind.keystoreInconsistent(
              permanentlyInvalidated: false,
            ),
          )
          ..failDelete = const WalletApiError(
            code: 'RW',
            message: 'wedged',
            kind: WalletErrorKind.keystoreUnavailable(),
          );
        final c = await reachNeedsRecovery(p);

        await notifierOf(c).recoverByRestore();
        await pumpEventQueue();

        expect(
          p.forceDeleteCount,
          0,
          reason:
              'NEVER auto-force a non-keystoreInconsistent fault (the SDK contract)',
        );
        expect(
          stateOf(c),
          isA<OnboardingFailed>(),
          reason:
              're-probed to the honest failure — the escape is offered again, no dead-end',
        );
      },
    );

    test(
      'even a force-wipe fault re-probes to a still-escapable failure (no dead-end)',
      () async {
        final p = FakeWalletProvisioner(exists: true)
          ..failOpen = const WalletApiError(
            code: 'RW',
            message: 'damaged',
            kind: WalletErrorKind.keystoreInconsistent(
              permanentlyInvalidated: false,
            ),
          )
          ..failDelete = const WalletApiError(
            code: 'RW',
            message: 'key gone',
            kind: WalletErrorKind.keystoreInconsistent(
              permanentlyInvalidated: false,
            ),
          )
          ..failForceDelete = const WalletApiError(
            code: 'RW',
            message: 'fs fault',
            kind: WalletErrorKind.io(),
          );
        final c = await reachNeedsRecovery(p);

        await notifierOf(c).recoverByRestore();
        await pumpEventQueue();

        expect(p.forceDeleteCount, 1);
        expect(
          stateOf(c),
          isA<OnboardingFailed>(),
          reason:
              'a force fault re-renders the failure — still escapable, never stuck',
        );
      },
    );

    test(
      'no-op from a non-failed state (guarded to OnboardingFailed)',
      () async {
        final p = FakeWalletProvisioner(exists: false);
        final c = harness(p, FakeOnboardingStore());
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingWelcome>());

        await notifierOf(c).recoverByRestore();
        await pumpEventQueue();

        expect(stateOf(c), isA<OnboardingWelcome>());
        expect(p.deleteCount, 0);
        expect(p.forceDeleteCount, 0);
      },
    );

    test(
      'a frustrated double-tap is single-flighted — wipes at most once',
      () async {
        // The escape button sits on a non-retryable failure the user may hammer.
        final p = FakeWalletProvisioner(exists: true)
          ..failOpen = const WalletApiError(
            code: 'RW',
            message: 'damaged',
            kind: WalletErrorKind.keystoreInconsistent(
              permanentlyInvalidated: false,
            ),
          )
          ..holdDelete = Completer<void>(); // hold the FIRST wipe in flight
        final c = await reachNeedsRecovery(p);

        final first = notifierOf(
          c,
        ).recoverByRestore(); // enters, awaits deleteWallet
        await pumpEventQueue();
        final second = notifierOf(
          c,
        ).recoverByRestore(); // _deletionInFlight ⇒ no-op
        await pumpEventQueue();

        p.holdDelete!.complete(); // release the first
        await Future.wait([first, second]);
        await pumpEventQueue();

        expect(
          p.deleteCount,
          1,
          reason: 'single-flight — the wipe runs at most once',
        );
        expect(p.forceDeleteCount, 0);
        expect(stateOf(c), isA<OnboardingRestoreInput>());
      },
    );
  });

  group('prior-activity guard on the forced-backup resume (#356-F2 / S174 '
      'security HIGH)', () {
    test(
      'a probe-resumed unconfirmed wallet WITH history (lost confirmed flag '
      'over an intact DB) lands AwaitingBackup with hasPriorActivity=true',
      () async {
        // The wallet has synced before (lastSynced set) — the signal a
        // previously-ACTIVE wallet leaves behind after its prefs flag is lost.
        final session = FakeWalletSession(
          snapshotValue: walletStateFixture(
            lastSynced: const SyncStamp(height: 500, at: 1751700000),
          ),
        );
        final c = harness(
          FakeWalletProvisioner(exists: true, session: session),
          FakeOnboardingStore(confirmed: false),
        );
        await pumpEventQueue();

        final state = stateOf(c);
        expect(state, isA<OnboardingAwaitingBackup>());
        expect((state as OnboardingAwaitingBackup).hasPriorActivity, isTrue);
        expect(sessionOf(c), isNull, reason: 'the deposit gate stays closed');
      },
    );

    test('a balance alone (never-synced snapshot carrying funds) also reads as '
        'prior activity', () async {
      final session = FakeWalletSession(
        snapshotValue: walletStateFixture(
          balance: balanceFixture(totalZat: 5000000),
        ),
      );
      final c = harness(
        FakeWalletProvisioner(exists: true, session: session),
        FakeOnboardingStore(confirmed: false),
      );
      await pumpEventQueue();

      expect((stateOf(c) as OnboardingAwaitingBackup).hasPriorActivity, isTrue);
    });

    test(
      'a virgin resumed wallet (crash between create and confirm) reads '
      'hasPriorActivity=false — the Start-over escape stays offered',
      () async {
        // Default fixture: lastSynced null, zero balance — the kill-test shape.
        final c = harness(
          FakeWalletProvisioner(exists: true),
          FakeOnboardingStore(confirmed: false),
        );
        await pumpEventQueue();

        final state = stateOf(c);
        expect(state, isA<OnboardingAwaitingBackup>());
        expect((state as OnboardingAwaitingBackup).hasPriorActivity, isFalse);
      },
    );

    test(
      'an UNREADABLE snapshot fails SAFE to hasPriorActivity=true (the escape '
      'hides; the pre-F2 re-confirm posture — never a blind delete offer)',
      () async {
        final session = FakeWalletSession()..snapshotThrows = true;
        final c = harness(
          FakeWalletProvisioner(exists: true, session: session),
          FakeOnboardingStore(confirmed: false),
        );
        await pumpEventQueue();

        expect(
          (stateOf(c) as OnboardingAwaitingBackup).hasPriorActivity,
          isTrue,
        );
      },
    );

    test(
      'the happy boot path (confirmed=true) takes NO extra snapshot read',
      () async {
        final session = FakeWalletSession();
        final c = harness(
          FakeWalletProvisioner(exists: true, session: session),
          FakeOnboardingStore(confirmed: true),
        );
        await pumpEventQueue();
        expect(stateOf(c), isA<OnboardingActive>());
        expect(
          session.snapshotCount,
          0,
          reason: 'the prior-activity probe runs only on the unconfirmed fork',
        );
      },
    );

    test('a HUNG snapshot is BOUNDED (10s) and fails SAFE to suppressed — boot '
        'proceeds to AwaitingBackup, never an eternal Loading (S175)', () {
      fakeAsync((async) {
        final c = harness(
          FakeWalletProvisioner(
            exists: true,
            session: _HangingSnapshotSession(),
          ),
          FakeOnboardingStore(confirmed: false),
        );
        async.flushMicrotasks();
        expect(stateOf(c), isA<OnboardingLoading>());

        async.elapse(const Duration(seconds: 10));
        async.flushMicrotasks();
        final state = stateOf(c);
        expect(state, isA<OnboardingAwaitingBackup>());
        expect(
          (state as OnboardingAwaitingBackup).hasPriorActivity,
          isTrue,
          reason: 'unknowable prior life ⇒ the escape stays hidden',
        );
      });
    });

    test(
      'a confirm persist-failure revert PRESERVES hasPriorActivity (the '
      'escape visibility never flickers open across the round-trip)',
      () async {
        final session = FakeWalletSession(
          snapshotValue: walletStateFixture(
            lastSynced: const SyncStamp(height: 500, at: 1751700000),
          ),
        );
        final store = FakeOnboardingStore(confirmed: false);
        final c = harness(
          FakeWalletProvisioner(exists: true, session: session),
          store,
        );
        await pumpEventQueue();
        expect(
          (stateOf(c) as OnboardingAwaitingBackup).hasPriorActivity,
          isTrue,
        );

        store.failSet = Exception('prefs write failed');
        await expectLater(notifierOf(c).confirmBackup(), throwsException);
        await pumpEventQueue();

        final reverted = stateOf(c);
        expect(reverted, isA<OnboardingAwaitingBackup>());
        expect((reverted as OnboardingAwaitingBackup).hasPriorActivity, isTrue);
      },
    );
  });

  group(
    'confirmBackup × deleteWallet mutual exclusion (S174 security MED)',
    () {
      test(
        'a confirm issued while a delete is IN FLIGHT is a no-op — the flag '
        'never persists true and the gate never opens over the shred',
        () async {
          final p = FakeWalletProvisioner(exists: true)
            ..holdDelete = Completer<void>();
          final store = FakeOnboardingStore(confirmed: false);
          final c = harness(p, store);
          await pumpEventQueue();
          expect(stateOf(c), isA<OnboardingAwaitingBackup>());

          // The shred starts (no synchronous state transition — the window the
          // latch closes) …
          final deletion = notifierOf(c).deleteWallet();
          await pumpEventQueue();
          expect(stateOf(c), isA<OnboardingAwaitingBackup>());

          // … and a raced confirm must refuse to run.
          await notifierOf(c).confirmBackup();
          expect(await store.isBackupConfirmed(), isFalse);
          expect(stateOf(c), isA<OnboardingAwaitingBackup>());

          p.holdDelete!.complete();
          expect(await deletion, WalletDeletionOutcome.shredded);
          await pumpEventQueue();
          expect(stateOf(c), isA<OnboardingWelcome>());
          expect(
            await store.isBackupConfirmed(),
            isFalse,
            reason: 'no stale durable true over a wiped wallet',
          );
          expect(sessionOf(c), isNull);
        },
      );

      test(
        'a delete issued while a confirm PERSIST is in flight is refused '
        '(Confirming is not a deletable state) and the confirm completes',
        () async {
          final store = FakeOnboardingStore(confirmed: false)
            ..holdSet = Completer<void>();
          final c = harness(FakeWalletProvisioner(exists: true), store);
          await pumpEventQueue();
          expect(stateOf(c), isA<OnboardingAwaitingBackup>());

          final confirm = notifierOf(c).confirmBackup(); // parks on the persist
          await pumpEventQueue();
          expect(stateOf(c), isA<OnboardingConfirming>());

          expect(
            await notifierOf(c).deleteWallet(),
            WalletDeletionOutcome.notDeletable,
          );

          store.holdSet!.complete();
          await confirm;
          await pumpEventQueue();
          expect(stateOf(c), isA<OnboardingActive>());
          expect(sessionOf(c), isNotNull);
        },
      );
    },
  );
}

/// A session whose snapshot never completes — the wedged-DB shape the probe's
/// bound exists for.
class _HangingSnapshotSession extends FakeWalletSession {
  @override
  Future<WalletState> snapshot() {
    snapshotCount++;
    return Completer<WalletState>().future;
  }
}
