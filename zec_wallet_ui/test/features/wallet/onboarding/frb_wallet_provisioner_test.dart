import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/frb_wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_state.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/wallet_provisioner.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_config.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// The production [FrbWalletProvisioner] (spec §3.2g iii-B-2-b). A real
/// [WalletHandle] is opaque and cannot be constructed in `flutter test`, so the
/// SDK entry points are injected: this exercises the part that carries the
/// VALUE — the BOUNDED-WAIT + the typed timeout mapping, the typed-error
/// passthrough, and the reveal-before-open guard — without a device. The
/// handle-wrapping happy path is e2e-owed.
void main() {
  final config = buildWalletConfig(dbDir: '/tmp/zec-test');

  // A future that never resolves — forces the bounded-wait timeout to fire. Not
  // a Timer, so it leaves nothing pending after the test.
  Future<T> never<T>() => Completer<T>().future;
  const tinyBound = Duration(milliseconds: 30);

  test('the default local-I/O bound is the documented 30s (gate 7)', () {
    expect(
      FrbWalletProvisioner.defaultLocalIoBound,
      const Duration(seconds: 30),
    );
  });

  group('bounded-wait → typed keystoreUnavailable', () {
    test(
      'walletExists that never returns times out to keystoreUnavailable',
      () async {
        final p = FrbWalletProvisioner(
          config: config,
          localIoBound: tinyBound,
          walletExistsFn: (_) => never<bool>(),
        );
        final error = await p.walletExists().then<Object?>(
          (_) => null,
          onError: (Object e) => e,
        );
        expect(error, isA<WalletApiError>());
        final api = error as WalletApiError;
        expect(api.kind, const WalletErrorKind.keystoreUnavailable());
        // Host-namespaced code so a log reader never mistakes it for an SDK code.
        expect(api.code, 'RW-HOST-IO-TIMEOUT');
        // And the host classifier routes that to a RETRYABLE device-locked.
        expect(
          classifyOnboardingFailure(api),
          OnboardingFailureKind.deviceLocked,
        );
        expect(OnboardingFailureKind.deviceLocked.isRetryable, isTrue);
      },
    );

    test('a wedged createGenerated times out to keystoreUnavailable', () async {
      final p = FrbWalletProvisioner(
        config: config,
        localIoBound: tinyBound,
        createGeneratedFn: (_) => never<WalletHandle>(),
      );
      await expectLater(
        p.createGenerated(),
        throwsA(
          isA<WalletApiError>().having(
            (e) => e.kind,
            'kind',
            const WalletErrorKind.keystoreUnavailable(),
          ),
        ),
      );
    });

    test('a wedged open times out to keystoreUnavailable', () async {
      final p = FrbWalletProvisioner(
        config: config,
        localIoBound: tinyBound,
        openFn: (_) => never<WalletHandle>(),
      );
      await expectLater(
        p.open(),
        throwsA(
          isA<WalletApiError>().having(
            (e) => e.kind,
            'kind',
            const WalletErrorKind.keystoreUnavailable(),
          ),
        ),
      );
    });
  });

  group('passthrough', () {
    test('a fast walletExists returns its bool unchanged', () async {
      final pTrue = FrbWalletProvisioner(
        config: config,
        walletExistsFn: (_) async => true,
      );
      final pFalse = FrbWalletProvisioner(
        config: config,
        walletExistsFn: (_) async => false,
      );
      expect(await pTrue.walletExists(), isTrue);
      expect(await pFalse.walletExists(), isFalse);
    });

    test('forwards the constructor WalletConfig to the SDK entry point', () {
      // A config-threading bug (passing null / a different config to the SDK)
      // would be invisible to the return-value tests — pin it.
      WalletConfig? received;
      final p = FrbWalletProvisioner(
        config: config,
        walletExistsFn: (c) async {
          received = c;
          return false;
        },
      );
      return p.walletExists().then((_) => expect(received, same(config)));
    });

    test(
      'a genuine typed SDK error propagates UNCHANGED (not remapped)',
      () async {
        // The bounded-wait must only intercept TIMEOUTS — a real typed kind must
        // reach the classifier intact (here: alreadyOpen, distinct from the
        // timeout's keystoreUnavailable).
        final p = FrbWalletProvisioner(
          config: config,
          walletExistsFn: (_) async => throw const WalletApiError(
            code: 'RW-LOCK-001',
            message: 'another instance holds the wallet',
            kind: WalletErrorKind.walletAlreadyOpen(),
          ),
        );
        final error = await p.walletExists().then<Object?>(
          (_) => null,
          onError: (Object e) => e,
        );
        expect(error, isA<WalletApiError>());
        expect(
          (error as WalletApiError).kind,
          const WalletErrorKind.walletAlreadyOpen(),
        );
        expect(
          classifyOnboardingFailure(error),
          OnboardingFailureKind.alreadyOpen,
        );
      },
    );
  });

  // NOTE: revealMnemonic's bounded-wait reuses the SAME `_bounded` helper proven
  // by the three timeout tests above; its handle-bearing happy/timeout path needs
  // a real WalletHandle (not constructible in flutter test) and is e2e-owed.
  test('revealMnemonic before create/open fails loud (defensive)', () async {
    final p = FrbWalletProvisioner(
      config: config,
      walletExistsFn: (_) async => false,
    );
    // No handle yet (no create/open on this instance) — must not silently
    // return empty words.
    expect(() => p.revealMnemonic(), throwsStateError);
  });

  // A throwing restoreFn lets us assert what the adapter PASSED without
  // constructing an opaque WalletHandle (the happy handle-wrap is e2e-owed).
  group('restore — birthday threading + word pass-through', () {
    test(
      'a creation date stamps config.birthdayHeight from the injected estimator '
      '(network from config); words pass through UNCHANGED (no re-normalize)',
      () async {
        WalletConfig? seenConfig;
        List<String>? seenWords;
        Network? estNetwork;
        DateTime? estDate;
        final p = FrbWalletProvisioner(
          config: config, // birthdayHeight is null on the create-default config
          birthdayEstimateFn: (network, date) {
            estNetwork = network;
            estDate = date;
            return 1234567;
          },
          restoreFn: (cfg, words) async {
            seenConfig = cfg;
            seenWords = words;
            throw StateError('capture-and-stop');
          },
        );
        final created = DateTime.utc(2021, 6);

        await p
            .restore(['abandon', 'ability'], approximateCreationTime: created)
            .then<void>((_) {}, onError: (_) {});

        expect(seenConfig?.birthdayHeight, 1234567);
        expect(
          seenConfig?.network,
          config.network,
          reason: 'the screen never needs to know mainnet/testnet',
        );
        expect(
          seenWords,
          ['abandon', 'ability'],
          reason:
              'the adapter trusts the controller chokepoint, does not '
              're-case key material',
        );
        expect(estNetwork, config.network);
        expect(estDate, created);
      },
    );

    test(
      'no creation date → no birthday stamped, estimator NOT called '
      '(the full, money-safe scan); the config passes through untouched',
      () async {
        var estimatorCalls = 0;
        WalletConfig? seenConfig;
        final p = FrbWalletProvisioner(
          config: config,
          birthdayEstimateFn: (_, _) {
            estimatorCalls++;
            return 1;
          },
          restoreFn: (cfg, _) async {
            seenConfig = cfg;
            throw StateError('capture-and-stop');
          },
        );

        await p.restore(['abandon']).then<void>((_) {}, onError: (_) {});

        expect(estimatorCalls, 0);
        expect(seenConfig?.birthdayHeight, isNull);
        expect(seenConfig, config, reason: 'every other field unchanged');
      },
    );

    // The host's config carries its own birthday AND a gated endpoint: the
    // shape the two tests above cannot see, since the reference config has
    // neither (the 2026-10-05 review, F06 and F07).
    final hostConfig = WalletConfig(
      dbDir: '/tmp/zec-test',
      network: config.network,
      endpointUrl: 'https://lwd.example:443',
      endpointAuthHeader: 'x-api-key',
      endpointAuthValue: 'endpoint-key',
      tor: const TorPolicy.off(),
      seedPersistence: config.seedPersistence,
      birthdayHeight: 2_900_000,
      broadcastJitter: config.broadcastJitter,
      machineMemoPrefixes: config.machineMemoPrefixes,
    );

    Future<WalletConfig?> restoreSeen(DateTime? created) async {
      WalletConfig? seen;
      final p = FrbWalletProvisioner(
        config: hostConfig,
        birthdayEstimateFn: (_, _) => 2_500_000,
        restoreFn: (cfg, _) async {
          seen = cfg;
          throw StateError('capture-and-stop');
        },
      );
      await p
          .restore(['abandon'], approximateCreationTime: created)
          .then<void>((_) {}, onError: (_) {});
      return seen;
    }

    test('"scan all history" clears a birthday the HOST set: the scan '
        'starts at activation, not at the host\'s height', () async {
      final seen = await restoreSeen(null);
      expect(seen?.birthdayHeight, isNull);
      expect(seen?.endpointAuthHeader, 'x-api-key');
      expect(seen?.endpointAuthValue, 'endpoint-key');
    });

    test('a restore with a date keeps the gated endpoint\'s key', () async {
      final seen = await restoreSeen(DateTime.utc(2025, 1, 1));
      expect(seen?.birthdayHeight, 2500000);
      expect(seen?.endpointAuthHeader, 'x-api-key');
      expect(seen?.endpointAuthValue, 'endpoint-key');
    });

    test('a watch-only import keeps the gated endpoint\'s key', () async {
      WalletConfig? seen;
      final p = FrbWalletProvisioner(
        config: hostConfig,
        createWatchOnlyFn: (cfg, _) async {
          seen = cfg;
          throw StateError('capture-and-stop');
        },
      );
      await p
          .createWatchOnly('uview1', birthdayHeight: 2_600_000)
          .then<void>((_) {}, onError: (_) {});
      expect(seen?.birthdayHeight, 2600000);
      expect(seen?.endpointAuthHeader, 'x-api-key');
      expect(seen?.endpointAuthValue, 'endpoint-key');
    });

    test(
      'a wedged restore times out to the typed keystoreUnavailable',
      () async {
        final p = FrbWalletProvisioner(
          config: config,
          localIoBound: tinyBound,
          restoreFn: (_, _) => never<WalletHandle>(),
        );
        await expectLater(
          p.restore(['abandon']),
          throwsA(
            isA<WalletApiError>().having(
              (e) => e.kind,
              'kind',
              const WalletErrorKind.keystoreUnavailable(),
            ),
          ),
        );
      },
    );

    test(
      'a typed restore error propagates UNCHANGED (classifier sees it)',
      () async {
        final p = FrbWalletProvisioner(
          config: config,
          restoreFn: (_, _) async => throw const WalletApiError(
            code: 'RW-SEED-002',
            message: 'unknown word',
            kind: WalletErrorKind.invalidMnemonic(wordIndex: 2),
          ),
        );
        await expectLater(
          p.restore(['abandon']),
          throwsA(
            isA<WalletApiError>().having(
              (e) => e.kind,
              'kind',
              const WalletErrorKind.invalidMnemonic(wordIndex: 2),
            ),
          ),
        );
      },
    );
  });

  group('rescanFrom (FR-1b / ADR-0534)', () {
    test('rescanFrom before create/open fails LOUD (no silent no-op of a money '
        'action)', () async {
      // No create/open ran on this instance → `_handle` is null. A rescan is a
      // money-recovery action; a missing handle must fail loud, never silently
      // no-op (invariant 10). This is the one adapter path reachable host-VM (the
      // success path needs a real WalletHandle and is e2e-owed, like restore's).
      final p = FrbWalletProvisioner(config: config);
      await expectLater(
        p.rescanFrom(const RescanAllHistory()),
        throwsA(isA<StateError>()),
      );
      await expectLater(
        p.rescanFrom(RescanFromTime(DateTime(2022, 6))),
        throwsA(isA<StateError>()),
        reason: 'a dated rescan before open is the same loud precondition',
      );
      await expectLater(
        p.rescanFrom(const RescanFromWalletBirthday(2400000)),
        throwsA(isA<StateError>()),
        reason: 'the wallet-birthday default is the same loud precondition',
      );
    });

    test('estimateBirthdayHeight rides the injected estimator with the '
        "config's network (the size-cue seam, #317)", () {
      final estimated = <(Network, DateTime)>[];
      final p = FrbWalletProvisioner(
        config: config,
        birthdayEstimateFn: (network, time) {
          estimated.add((network, time));
          return 1234567;
        },
      );
      final date = DateTime(2024, 8, 1);
      expect(p.estimateBirthdayHeight(date), 1234567);
      expect(estimated, [(config.network, date)]);
    });
  });

  group('deleteWallet / custodyDisclosure (FR-14)', () {
    test('deleteWallet with no live handle wipes the config, skips close', () async {
      // The post-teardown / fresh-instance panic-wipe path (`_handle == null`):
      // there is nothing to close, so it goes straight to the SDK wipe with THIS
      // wallet's config. (The close-BEFORE-wipe ordering with a live handle needs
      // a real opaque WalletHandle and is e2e-owed, like the create/open wrap.)
      var closeCalls = 0;
      WalletConfig? wiped;
      final p = FrbWalletProvisioner(
        config: config,
        closeFn: (_) async => closeCalls++,
        wipeFn: (c) async => wiped = c,
      );
      await p.deleteWallet();
      expect(closeCalls, 0, reason: 'no live handle ⇒ nothing to close');
      expect(wiped?.dbDir, config.dbDir, reason: 'wipes THIS wallet by db_dir');
    });

    test('a wedged wipe times out to the typed keystoreUnavailable', () async {
      final p = FrbWalletProvisioner(
        config: config,
        localIoBound: tinyBound,
        wipeFn: (_) => never<void>(),
      );
      await expectLater(
        p.deleteWallet(),
        throwsA(
          isA<WalletApiError>().having(
            (e) => e.kind,
            'kind',
            const WalletErrorKind.keystoreUnavailable(),
          ),
        ),
      );
    });

    test(
      'custodyDisclosure forwards the config and returns the tier',
      () async {
        WalletConfig? probed;
        final p = FrbWalletProvisioner(
          config: config,
          custodyDisclosureFn: (c) async {
            probed = c;
            return const CustodyDisclosure(
              tier: 'apple_secure_enclave',
              eraseAssurance: EraseAssurance.hardwareKeyDeleted,
              degraded: false,
            );
          },
        );
        final disclosure = await p.custodyDisclosure();
        expect(probed?.dbDir, config.dbDir);
        expect(disclosure.tier, 'apple_secure_enclave');
        expect(disclosure.eraseAssurance, EraseAssurance.hardwareKeyDeleted);
      },
    );

    test(
      'a wedged custodyDisclosure times out to keystoreUnavailable',
      () async {
        final p = FrbWalletProvisioner(
          config: config,
          localIoBound: tinyBound,
          custodyDisclosureFn: (_) => never<CustodyDisclosure>(),
        );
        await expectLater(
          p.custodyDisclosure(),
          throwsA(
            isA<WalletApiError>().having(
              (e) => e.kind,
              'kind',
              const WalletErrorKind.keystoreUnavailable(),
            ),
          ),
        );
      },
    );

    // #252 (device-found): deleting WHILE the wallet syncs transiently sees
    // WalletOpen — a finishing scan batch legitimately holds the lock for a
    // moment after close(). The delete must WAIT it out, not fail the user.
    WalletApiError walletOpen() => const WalletApiError(
      code: 'RW-LIFE-002',
      message: 'wallet open',
      kind: WalletErrorKind.walletOpen(),
    );

    // The OPEN path's lock signal is a DIFFERENT kind (RW-LIFE-001, from
    // `WalletLock::acquire`) than the wipe refusal above — the fake must throw
    // what production throws (the review's dead-belt finding: open-retry
    // tests injecting `walletOpen` validated a kind open() never emits).
    WalletApiError walletAlreadyOpen() => const WalletApiError(
      code: 'RW-LIFE-001',
      message: 'wallet already open',
      kind: WalletErrorKind.walletAlreadyOpen(),
    );

    test(
      'a transient WalletOpen on wipe is WAITED OUT (the delete-during-sync race)',
      () async {
        var calls = 0;
        final p = FrbWalletProvisioner(
          config: config,
          wipeLockRetryBackoff: const Duration(milliseconds: 1),
          wipeFn: (_) async {
            calls++;
            if (calls < 3) throw walletOpen(); // held by a finishing scan batch
          },
        );
        await p.deleteWallet(); // succeeds once the lock frees
        expect(
          calls,
          3,
          reason: 'two transient WalletOpen, then the lock released',
        );
      },
    );

    test(
      'a WalletOpen that OUTLASTS the budget surfaces honestly (a foreign holder)',
      () async {
        var calls = 0;
        final p = FrbWalletProvisioner(
          config: config,
          wipeLockRetryBackoff: const Duration(milliseconds: 1),
          wipeLockRetries: 3,
          wipeFn: (_) async {
            calls++;
            throw walletOpen();
          },
        );
        await expectLater(
          p.deleteWallet(),
          throwsA(
            isA<WalletApiError>().having(
              (e) => e.kind,
              'kind',
              const WalletErrorKind.walletOpen(),
            ),
          ),
        );
        expect(
          calls,
          4,
          reason:
              '1 initial + 3 retries, then surfaced (never an infinite wait)',
        );
      },
    );

    test(
      'a non-WalletOpen wipe fault propagates IMMEDIATELY (only lock-contention waits)',
      () async {
        var calls = 0;
        final p = FrbWalletProvisioner(
          config: config,
          // A huge backoff would hang the test if a non-WalletOpen fault were retried.
          wipeLockRetryBackoff: const Duration(seconds: 99),
          wipeFn: (_) async {
            calls++;
            throw const WalletApiError(
              code: 'RW-KS-002',
              message: 'inconsistent',
              kind: WalletErrorKind.keystoreInconsistent(
                permanentlyInvalidated: false,
              ),
            );
          },
        );
        await expectLater(
          p.deleteWallet(),
          throwsA(
            isA<WalletApiError>().having(
              (e) => e.kind,
              'kind',
              const WalletErrorKind.keystoreInconsistent(
                permanentlyInvalidated: false,
              ),
            ),
          ),
        );
        expect(
          calls,
          1,
          reason: 'fail-closed faults are immediate; only WalletOpen waits',
        );
      },
    );

    // v-5c finding #2 (device-found): the recover-by-reopen after a rescan fault
    // races the SAME finishing scan batch the delete path does — the lock is held
    // for a moment after the handle closed, and the un-retried open() surfaced the
    // intermittent `WalletAlreadyOpen`. open() now rides the same bounded wait.
    // (The happy wrap needs a real opaque WalletHandle and is e2e-owed, so the
    // wait-out is proven by SEQUENCING: transient WalletOpen throws are consumed,
    // and the NEXT distinct fault is what surfaces.)
    test(
      'a transient WalletOpen on open is WAITED OUT (the rescan recover-by-reopen race)',
      () async {
        var calls = 0;
        final p = FrbWalletProvisioner(
          config: config,
          wipeLockRetryBackoff: const Duration(milliseconds: 1),
          openFn: (_) async {
            calls++;
            if (calls < 3) {
              throw walletAlreadyOpen(); // held by a finishing scan batch
            }
            // Lock released — the attempt PROCEEDS (proven by reaching a distinct
            // post-lock fault, since a real handle is not constructible here).
            throw const WalletApiError(
              code: 'RW-KS-001',
              message: 'unavailable',
              kind: WalletErrorKind.keystoreUnavailable(),
            );
          },
        );
        await expectLater(
          p.open(),
          throwsA(
            isA<WalletApiError>().having(
              (e) => e.kind,
              'kind',
              const WalletErrorKind.keystoreUnavailable(),
            ),
          ),
        );
        expect(
          calls,
          3,
          reason:
              'two transient WalletOpen waited out, then the attempt proceeded',
        );
      },
    );

    test(
      'a WalletOpen on open that OUTLASTS the budget surfaces honestly',
      () async {
        var calls = 0;
        final p = FrbWalletProvisioner(
          config: config,
          wipeLockRetryBackoff: const Duration(milliseconds: 1),
          wipeLockRetries: 3,
          openFn: (_) async {
            calls++;
            throw walletAlreadyOpen();
          },
        );
        await expectLater(
          p.open(),
          throwsA(
            isA<WalletApiError>().having(
              (e) => e.kind,
              'kind',
              const WalletErrorKind.walletAlreadyOpen(),
            ),
          ),
        );
        expect(
          calls,
          4,
          reason:
              '1 initial + 3 retries, then surfaced (a genuine second holder)',
        );
      },
    );

    test(
      'a non-WalletOpen open fault propagates IMMEDIATELY (only lock-contention waits)',
      () async {
        var calls = 0;
        final p = FrbWalletProvisioner(
          config: config,
          // A huge backoff would hang the test if a non-WalletOpen fault were retried.
          wipeLockRetryBackoff: const Duration(seconds: 99),
          openFn: (_) async {
            calls++;
            throw const WalletApiError(
              code: 'RW-KS-001',
              message: 'unavailable',
              kind: WalletErrorKind.keystoreUnavailable(),
            );
          },
        );
        await expectLater(
          p.open(),
          throwsA(
            isA<WalletApiError>().having(
              (e) => e.kind,
              'kind',
              const WalletErrorKind.keystoreUnavailable(),
            ),
          ),
        );
        expect(
          calls,
          1,
          reason:
              'only the lock-contention transient waits; all else is immediate',
        );
      },
    );
  });
}
