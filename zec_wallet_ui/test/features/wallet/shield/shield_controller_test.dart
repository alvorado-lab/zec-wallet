import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_controller.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_state.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Shield state-machine tests (Recv-3). The whole flow runs on the host VM
/// against `FakeWalletSession` behind the `WalletSession` port — no native
/// library, no device. The money-correctness + privacy properties are pinned in
/// the controller (the SDK is the source of truth; the sheet only renders).
void main() {
  ({ProviderContainer container, FakeWalletSession fake}) harness({
    WalletSession? session,
    bool nullSession = false,
  }) {
    final fake = session is FakeWalletSession ? session : FakeWalletSession();
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(
          nullSession ? null : (session ?? fake),
        ),
      ],
    );
    addTearDown(container.dispose);
    return (container: container, fake: fake);
  }

  ShieldController ctl(ProviderContainer c) =>
      c.read(shieldControllerProvider.notifier);
  ShieldState st(ProviderContainer c) => c.read(shieldControllerProvider);

  WalletApiError err(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

  group('prepare', () {
    test('a shieldable balance → ShieldReady carrying the proposal', () async {
      final h = harness();
      h.fake.proposeShieldResult = shieldProposalFixture(
        proposalId: 42,
        totalZat: 500000,
        feeZat: 15000,
      );
      await ctl(h.container).prepare();
      final s = st(h.container);
      expect(s, isA<ShieldReady>());
      final ready = s as ShieldReady;
      // The DTO carries the engine's own gross/fee/net (net = gross − fee), is
      // tagged a shield, and is privacy-positive (no transparent recipient).
      expect(ready.proposal.proposalId, 42);
      expect(ready.proposal.totalZat, 500000);
      expect(ready.proposal.feeZat, 15000);
      expect(ready.proposal.changeZat, 485000);
      expect(ready.proposal.isShield, isTrue);
      expect(ready.proposal.hasTransparentRecipient, isFalse);
      expect(h.fake.proposeShieldCount, 1);
    });

    test(
      'below threshold (null) → ShieldNothingToShield, never an error',
      () async {
        final h = harness();
        h.fake.proposeShieldResult = null;
        await ctl(h.container).prepare();
        expect(st(h.container), isA<ShieldNothingToShield>());
        expect(h.fake.proposeShieldCount, 1);
      },
    );

    test('a stale wallet → ShieldUnavailable(notSyncedYet)', () async {
      final h = harness();
      h.fake.proposeShieldThrows = err(const WalletErrorKind.proposalStale());
      await ctl(h.container).prepare();
      final s = st(h.container);
      expect(s, isA<ShieldUnavailable>());
      expect((s as ShieldUnavailable).reason, ShieldFaultReason.notSyncedYet);
    });

    test('a busy wallet → ShieldUnavailable(walletBusy)', () async {
      final h = harness();
      h.fake.proposeShieldThrows = err(
        const WalletErrorKind.walletBusy(phase: LifecyclePhase.closing),
      );
      await ctl(h.container).prepare();
      expect(
        (st(h.container) as ShieldUnavailable).reason,
        ShieldFaultReason.walletBusy,
      );
    });

    test('a storeBusy write-race → ShieldUnavailable(walletBusy), parity with '
        'the send flow (#373) — never a generic couldn\'t-prepare', () async {
      final h = harness();
      h.fake.proposeShieldThrows = err(const WalletErrorKind.storeBusy());
      await ctl(h.container).prepare();
      expect(
        (st(h.container) as ShieldUnavailable).reason,
        ShieldFaultReason.walletBusy,
      );
    });

    test(
      'a transient propose refusal → ShieldUnavailable(couldNotPrepareTransient) '
      '— INC-018 (b) reaches the shield flow too (the Batch C arch pass): '
      'the same Rust classifier that types a witness-held or locked-input '
      'refusal for Send types it for Shield, so the sheet says "try again '
      'in a moment", never the title-only dead-end',
      () async {
        final h = harness();
        h.fake.proposeShieldThrows = err(
          const WalletErrorKind.proposeTransient(),
        );
        await ctl(h.container).prepare();
        // Mutant: the proposeTransient arm deleted from classifyShieldPrepareFailure
        // → the wildcard maps it to couldNotPrepare and this row reds.
        expect(
          (st(h.container) as ShieldUnavailable).reason,
          ShieldFaultReason.couldNotPrepareTransient,
        );
      },
    );

    test('an out-of-disk prepare → ShieldUnavailable(storageFull) "free up '
        'space", never the generic couldNotPrepare (#373 follow-up)', () async {
      final h = harness();
      h.fake.proposeShieldThrows = err(const WalletErrorKind.diskFull());
      await ctl(h.container).prepare();
      expect(
        (st(h.container) as ShieldUnavailable).reason,
        ShieldFaultReason.storageFull,
      );
    });

    test(
      'no live session → ShieldUnavailable(walletUnavailable), no bridge call',
      () async {
        final h = harness(nullSession: true);
        await ctl(h.container).prepare();
        expect(
          (st(h.container) as ShieldUnavailable).reason,
          ShieldFaultReason.walletUnavailable,
        );
        expect(h.fake.proposeShieldCount, 0);
      },
    );

    test(
      'a non-FRB error → ShieldUnavailable(couldNotPrepare) — the generic arm',
      () async {
        final h = harness();
        h.fake.proposeShieldThrows = StateError('not a WalletApiError');
        await ctl(h.container).prepare();
        expect(
          (st(h.container) as ShieldUnavailable).reason,
          ShieldFaultReason.couldNotPrepare,
        );
      },
    );

    test(
      'a second prepare while one is in flight is a no-op (double-tap)',
      () async {
        final h = harness();
        h.fake.proposeShieldResult = shieldProposalFixture();
        final first = ctl(h.container).prepare();
        // A synchronous re-entry sees ShieldPreparing and no-ops.
        await ctl(h.container).prepare();
        await first;
        expect(
          h.fake.proposeShieldCount,
          1,
          reason: 'only the first prepare runs',
        );
      },
    );
  });

  group('confirm', () {
    Future<ProviderContainer> readied({
      List<TxSubmitResult>? sendResults,
    }) async {
      final h = harness();
      h.fake.proposeShieldResult = shieldProposalFixture(proposalId: 42);
      if (sendResults != null) h.fake.sendResults = sendResults;
      await ctl(h.container).prepare();
      expect(st(h.container), isA<ShieldReady>());
      return h.container;
    }

    test(
      'a successful broadcast → ShieldDone(SendSucceeded), consuming the token',
      () async {
        final c = await readied(
          sendResults: const [TxSubmitResult.success(txidHex: 'aa')],
        );
        await ctl(c).confirm();
        final s = st(c);
        expect(s, isA<ShieldDone>());
        expect((s as ShieldDone).outcome, isA<SendSucceeded>());
      },
    );

    test(
      'confirm only fires from ShieldReady — a stray confirm is a no-op',
      () async {
        final h = harness();
        // state is ShieldIdle (never prepared)
        await ctl(h.container).confirm();
        expect(st(h.container), isA<ShieldIdle>());
        expect(h.fake.sendCount, 0, reason: 'no token to consume from idle');
      },
    );

    test(
      'confirm is single-shot under a double-tap — the token is consumed once',
      () async {
        final h = harness();
        h.fake.proposeShieldResult = shieldProposalFixture();
        await ctl(h.container).prepare();
        expect(st(h.container), isA<ShieldReady>());
        // Two confirms back-to-back: the first synchronously transitions to
        // ShieldSubmitting BEFORE its await, so the second sees non-Ready and no-ops
        // (the SDK one-shot guard is the backstop). The laggy-phone double-tap.
        final c1 = ctl(h.container).confirm();
        final c2 = ctl(h.container).confirm();
        await Future.wait([c1, c2]);
        expect(
          h.fake.sendCount,
          1,
          reason:
              'the shield token is consumed at most once — never broadcast twice',
        );
      },
    );

    test(
      'an unbroadcast tx → ShieldDone(SendSavedForRetry) — money-safe',
      () async {
        final c = await readied(
          sendResults: const [TxSubmitResult.grpcFailure(txidHex: 'aa')],
        );
        await ctl(c).confirm();
        final s = st(c) as ShieldDone;
        expect(s.outcome, isA<SendSavedForRetry>());
      },
    );

    test(
      'a consumed token → ShieldDone(SendAlreadySubmitted), never re-broadcast',
      () async {
        final h = harness();
        h.fake.proposeShieldResult = shieldProposalFixture();
        h.fake.sendThrows = err(const WalletErrorKind.proposalAlreadyUsed());
        await ctl(h.container).prepare();
        await ctl(h.container).confirm();
        expect(
          (st(h.container) as ShieldDone).outcome,
          isA<SendAlreadySubmitted>(),
        );
      },
    );

    test(
      'a stale anchor at send → ShieldUnavailable(notSyncedYet) to re-prepare',
      () async {
        final h = harness();
        h.fake.proposeShieldResult = shieldProposalFixture();
        h.fake.sendThrows = err(const WalletErrorKind.proposalStale());
        await ctl(h.container).prepare();
        await ctl(h.container).confirm();
        expect(
          (st(h.container) as ShieldUnavailable).reason,
          ShieldFaultReason.notSyncedYet,
        );
      },
    );

    test(
      'a signFailed at send → ShieldDone(SendSignFailed) — no money moved',
      () async {
        final h = harness();
        h.fake.proposeShieldResult = shieldProposalFixture();
        h.fake.sendThrows = err(const WalletErrorKind.signFailed());
        await ctl(h.container).prepare();
        await ctl(h.container).confirm();
        expect((st(h.container) as ShieldDone).outcome, isA<SendSignFailed>());
      },
    );

    test(
      'a non-FRB error at send → ShieldOutcomeUnknown (R13: an untyped throw '
      'from a started spend is not provably before persistence)',
      () async {
        final h = harness();
        h.fake.proposeShieldResult = shieldProposalFixture();
        h.fake.sendThrows = StateError('not a WalletApiError');
        await ctl(h.container).prepare();
        await ctl(h.container).confirm();
        expect(st(h.container), isA<ShieldOutcomeUnknown>());
      },
    );
  });

  group('the HOST send authorizer (#327 seam)', () {
    ({
      ProviderContainer container,
      FakeWalletSession fake,
      FakeSendAuthorizer auth,
    })
    authorized({bool denyAll = false}) {
      final fake = FakeWalletSession();
      final auth = FakeSendAuthorizer(denyAll: denyAll);
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(container.dispose);
      return (container: container, fake: fake, auth: auth);
    }

    test('confirm routes through the authorizer EXACTLY once as a SHIELD '
        'intent carrying the proposal total', () async {
      final h = authorized();
      h.fake.proposeShieldResult = shieldProposalFixture(
        proposalId: 4,
        totalZat: 700000,
      );
      await ctl(h.container).prepare();
      await ctl(h.container).confirm();
      expect(st(h.container), isA<ShieldDone>());
      expect(h.auth.intents, hasLength(1));
      expect(h.auth.intents.single.kind, WalletSpendKind.shield);
      expect(h.auth.intents.single.amountZat, 700000);
      // #383 R3: a shield is a wallet-internal self-transfer carrying the
      // proposal's own fee — the prompt facts a host renders.
      expect(h.auth.intents.single.recipientIsSelf, isTrue);
      expect(h.auth.intents.single.feeZat, h.fake.proposeShieldResult!.feeZat);
      expect(h.fake.sendCount, 1);
      // The propose half is local + deterministic — never authorized.
      expect(h.fake.proposeShieldCount, 1);
    });

    test('a DENIED confirm lands back on the confirm sheet with ZERO bridge '
        'calls — the token stays consumable', () async {
      final h = authorized(denyAll: true);
      h.fake.proposeShieldResult = shieldProposalFixture(proposalId: 4);
      await ctl(h.container).prepare();
      final ready = st(h.container) as ShieldReady;
      await ctl(h.container).confirm();
      expect(st(h.container), same(ready));
      expect(h.fake.sendCount, 0);
    });
  });

  group('session-flip liveness (#330)', () {
    // The same two-halves pin as the send controller's group: `_disposed`
    // resets each build (no wedge after a host identity switch), and the
    // post-await identity guards keep the dead cycle from writing into the
    // new one. See send_controller_test.dart for the full race matrix; here
    // the shield-specific arms.
    ({ProviderContainer container, StateProvider<WalletSession?> sessionSwitch})
    flippable(FakeWalletSession first, {FakeSendAuthorizer? auth}) {
      final sessionSwitch = StateProvider<WalletSession?>((ref) => first);
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
          if (auth != null)
            walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(container.dispose);
      container.listen(shieldControllerProvider, (_, _) {});
      return (container: container, sessionSwitch: sessionSwitch);
    }

    test('ONE session flip does not wedge the controller — a shield on the '
        'NEW session still renders', () async {
      final fakeA = FakeWalletSession();
      final fakeB = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture(proposalId: 42);
      final h = flippable(fakeA);

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      await ctl(h.container).prepare();
      expect(
        st(h.container),
        isA<ShieldReady>(),
        reason: 'one identity switch must not leave the shield sheet dead',
      );
      expect(fakeB.proposeShieldCount, 1);
      expect(fakeA.proposeShieldCount, 0);
    });

    test('a proposeShield resolving AFTER a mid-flight flip writes NOTHING '
        'into the new cycle', () async {
      final fakeA = FakeWalletSession()
        ..proposeShieldGate = Completer<void>()
        ..proposeShieldResult = shieldProposalFixture(proposalId: 1);
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA);

      final pending = ctl(h.container).prepare();
      expect(st(h.container), isA<ShieldPreparing>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);
      expect(st(h.container), isA<ShieldIdle>());

      fakeA.proposeShieldGate!.complete();
      await pending;
      expect(
        st(h.container),
        isA<ShieldIdle>(),
        reason:
            'the dead cycle\'s shieldable-balance review must never '
            'surface over the fresh session',
      );
    });

    test('a denial landing AFTER a mid-prompt flip never restores the dead '
        'session\'s confirm sheet', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(
        denyAll: true,
        prompt: (_) => gate.future,
      );
      final fakeA = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture(proposalId: 4);
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA, auth: auth);

      await ctl(h.container).prepare();
      final confirm = ctl(h.container).confirm(); // parks at the host prompt
      expect(st(h.container), isA<ShieldSubmitting>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      gate.complete();
      await confirm;
      expect(
        st(h.container),
        isA<ShieldIdle>(),
        reason: 'no dead-session ShieldReady over the fresh cycle',
      );
      expect(fakeA.sendCount, 0);
      expect(fakeB.sendCount, 0);
    });

    test(
      'a denial landing while the NEW cycle\'s OWN confirm is Submitting '
      'restores nothing — instance identity, not type (S153 review F2)',
      () async {
        final gateA = Completer<void>();
        var calls = 0;
        final auth = FakeSendAuthorizer(
          prompt: (intent) async {
            calls++;
            if (calls == 1) {
              await gateA.future;
              throw const WalletSpendAuthorizationDenied();
            }
            await Completer<void>().future; // the new cycle parks forever
          },
        );
        final fakeA = FakeWalletSession()
          ..proposeShieldResult = shieldProposalFixture(proposalId: 4);
        final fakeB = FakeWalletSession()
          ..proposeShieldResult = shieldProposalFixture(proposalId: 5);
        final h = flippable(fakeA, auth: auth);

        await ctl(h.container).prepare();
        final confirmA = ctl(h.container).confirm(); // parks (call 1)

        h.container.read(h.sessionSwitch.notifier).state = fakeB;
        await Future<void>.delayed(Duration.zero);

        await ctl(h.container).prepare();
        unawaited(ctl(h.container).confirm()); // parks forever (call 2)
        final submittingB = st(h.container);
        expect(submittingB, isA<ShieldSubmitting>());

        gateA
            .complete(); // a type-only guard would restore the DEAD review here
        await confirmA;
        expect(st(h.container), same(submittingB));
        expect(fakeA.sendCount, 0);
        expect(fakeB.sendCount, 0);
      },
    );

    test('a dead cycle\'s proposeShield resolving while the NEW cycle is '
        'ITSELF Preparing writes nothing — distinct transient INSTANCES, not '
        'a canonical const (S153 review F2)', () async {
      final fakeA = FakeWalletSession()
        ..proposeShieldGate = Completer<void>()
        ..proposeShieldResult = shieldProposalFixture(proposalId: 1);
      final fakeB = FakeWalletSession()
        ..proposeShieldGate = Completer<void>()
        ..proposeShieldResult = shieldProposalFixture(proposalId: 2);
      final h = flippable(fakeA);

      final pendingA = ctl(h.container).prepare();
      expect(st(h.container), isA<ShieldPreparing>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      final pendingB = ctl(h.container).prepare();
      final preparingB = st(h.container);
      expect(preparingB, isA<ShieldPreparing>());

      fakeA.proposeShieldGate!.complete();
      await pendingA;
      expect(
        st(h.container),
        same(preparingB),
        reason:
            'the dead session\'s shieldable-balance review must not '
            'render while the new cycle is still preparing',
      );

      fakeB.proposeShieldGate!.complete();
      await pendingB;
      expect((st(h.container) as ShieldReady).proposal.proposalId, 2);
    });

    test('an APPROVAL landing after a mid-prompt flip is FENCED — the shield '
        'spends on NEITHER session (S153 wrap review)', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
      final fakeA = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture(proposalId: 4);
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA, auth: auth);

      await ctl(h.container).prepare();
      final confirm = ctl(h.container).confirm(); // parks at the host prompt
      expect(st(h.container), isA<ShieldSubmitting>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      gate.complete(); // the prompt APPROVES after the identity switch
      await confirm;
      expect(st(h.container), isA<ShieldIdle>());
      expect(
        fakeA.sendCount,
        0,
        reason: 'the fence refused the dead identity\'s spend',
      );
      expect(fakeB.sendCount, 0);
    });

    test('sheet re-entry mid-confirm RE-ATTACHES (S153 review F1): reset() is '
        'a no-op in flight, so the landing outcome still renders', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
      final fake = FakeWalletSession()
        ..proposeShieldResult = shieldProposalFixture(proposalId: 3);
      final h = flippable(fake, auth: auth);

      await ctl(h.container).prepare();
      final confirm = ctl(h.container).confirm(); // parks at the host prompt
      final submitting = st(h.container);
      expect(submitting, isA<ShieldSubmitting>());

      ctl(h.container).reset(); // the sheet's entry hook, re-opened mid-flow
      expect(st(h.container), same(submitting));

      gate.complete();
      await confirm;
      expect(st(h.container), isA<ShieldDone>());
      expect(fake.sendCount, 1);
    });
  });
}
