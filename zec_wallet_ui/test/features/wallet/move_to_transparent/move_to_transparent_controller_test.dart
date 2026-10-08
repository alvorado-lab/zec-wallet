import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_controller.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_state.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/send/zec_amount.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Move-to-transparent state-machine tests (§3.2i-1 — the Send expert layer's
/// first slice). The whole flow runs on the host VM against `FakeWalletSession`
/// behind the `WalletSession` port — no native library, no device. The
/// money-correctness + privacy properties are pinned here (the SDK is the source
/// of truth; the sheet only renders).
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

  MoveToTransparentController ctl(ProviderContainer c) =>
      c.read(moveToTransparentControllerProvider.notifier);
  MoveToTransparentState st(ProviderContainer c) =>
      c.read(moveToTransparentControllerProvider);

  WalletApiError err(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

  group('start / loadAddress', () {
    test(
      'loads the wallet\'s OWN transparent address → MoveAmountEntry',
      () async {
        final h = harness();
        h.fake.currentTransparentAddressResult = 't1myownTaddr';
        await ctl(h.container).start();
        final s = st(h.container);
        expect(s, isA<MoveAmountEntry>());
        expect((s as MoveAmountEntry).ownAddress, 't1myownTaddr');
        expect(s.fault, isNull);
        expect(h.fake.currentTransparentAddressCount, 1);
      },
    );

    test(
      'a failed address fetch → MoveUnavailable(couldNotLoadAddress), retryable',
      () async {
        final h = harness();
        h.fake.currentTransparentAddressThrows = StateError('wedged');
        await ctl(h.container).start();
        final s = st(h.container);
        expect(s, isA<MoveUnavailable>());
        expect(
          (s as MoveUnavailable).reason,
          MoveFaultReason.couldNotLoadAddress,
        );

        // retryLoad re-fetches and recovers once the FFI is healthy again.
        h.fake.currentTransparentAddressThrows = null;
        await ctl(h.container).retryLoad();
        expect(st(h.container), isA<MoveAmountEntry>());
      },
    );

    test(
      'no live wallet session → MoveUnavailable(walletUnavailable)',
      () async {
        final h = harness(nullSession: true);
        await ctl(h.container).start();
        final s = st(h.container);
        expect(s, isA<MoveUnavailable>());
        expect(
          (s as MoveUnavailable).reason,
          MoveFaultReason.walletUnavailable,
        );
      },
    );
  });

  group('prepare', () {
    Future<ProviderContainer> atAmountEntry(FakeWalletSession fake) async {
      final h = harness(session: fake);
      await ctl(h.container).start();
      expect(st(h.container), isA<MoveAmountEntry>());
      return h.container;
    }

    test(
      'composes a de-shield to the OWN address with NO memo, then proposes → MoveReady',
      () async {
        final fake = FakeWalletSession()
          ..currentTransparentAddressResult = 't1myownTaddr'
          ..proposeResult = sendProposalFixture(
            proposalId: 7,
            totalZat: 200000,
            feeZat: 1000,
            hasTransparentRecipient: true,
          );
        final c = await atAmountEntry(fake);

        await ctl(c).prepare('0.002'); // 0.002 ZEC = 200000 zat
        final s = st(c);
        expect(s, isA<MoveReady>());
        final ready = s as MoveReady;
        expect(ready.ownAddress, 't1myownTaddr');
        expect(ready.proposal.proposalId, 7);
        // The de-shield disclosure rides the proposal (a transparent output).
        expect(ready.proposal.hasTransparentRecipient, isTrue);

        // CORRECTNESS: the destination is the SDK-supplied OWN t-addr (never user
        // input), the amount is the integer-exact parse, and a transparent
        // recipient carries NO memo.
        expect(fake.lastComposeRecipient, 't1myownTaddr');
        expect(fake.lastComposeAmountZat, 200000);
        expect(fake.lastComposeMemo, isNull);
        expect(fake.composeCount, 1);
        expect(fake.proposeCount, 1);
      },
    );

    test(
      'a host-side amount parse fault → MoveAmountEntry, never a bridge call',
      () async {
        final fake = FakeWalletSession();
        final c = await atAmountEntry(fake);

        await ctl(c).prepare('not-a-number');
        final s = st(c);
        expect(s, isA<MoveAmountEntry>());
        final entry = s as MoveAmountEntry;
        expect(entry.fault, isA<SendAmountFault>());
        expect(
          (entry.fault as SendAmountFault).fault,
          ZecAmountFault.notANumber,
        );
        expect(entry.ownAddress, isNotEmpty); // form context preserved
        expect(fake.composeCount, 0); // never reached the SDK
        expect(fake.proposeCount, 0);
      },
    );

    test(
      'an EMPTY amount (Review tapped on a blank field) → empty parse fault',
      () async {
        final fake = FakeWalletSession();
        final c = await atAmountEntry(fake);

        await ctl(c).prepare('');
        final s = st(c);
        expect(s, isA<MoveAmountEntry>());
        expect((s as MoveAmountEntry).fault, isA<SendAmountFault>());
        expect(((s.fault as SendAmountFault).fault), ZecAmountFault.empty);
        expect(fake.composeCount, 0);
      },
    );

    test(
      'insufficient funds → MoveAmountEntry carrying the audited figures',
      () async {
        final fake = FakeWalletSession()
          ..proposeThrows = err(
            const WalletErrorKind.insufficientFunds(
              availableZat: 100,
              requiredZat: 500,
              pendingIncomingZat: 50,
            ),
          );
        final c = await atAmountEntry(fake);

        await ctl(c).prepare('1');
        final s = st(c);
        expect(s, isA<MoveAmountEntry>());
        final fault = (s as MoveAmountEntry).fault;
        expect(fault, isA<SendInsufficientFunds>());
        final insufficient = fault as SendInsufficientFunds;
        expect(insufficient.availableZat, 100);
        expect(insufficient.requiredZat, 500);
        expect(insufficient.pendingIncomingZat, 50);
      },
    );

    test(
      'not-synced-yet (proposalStale on propose) → MoveAmountEntry(notSyncedYet)',
      () async {
        final fake = FakeWalletSession()
          ..proposeThrows = err(const WalletErrorKind.proposalStale());
        final c = await atAmountEntry(fake);

        await ctl(c).prepare('1');
        final fault = (st(c) as MoveAmountEntry).fault;
        expect(fault, isA<SendCategoricalFault>());
        expect(
          (fault as SendCategoricalFault).reason,
          SendFaultReason.notSyncedYet,
        );
      },
    );

    test(
      'a compose-time throw → MoveAmountEntry with an honest inline fault',
      () async {
        final fake = FakeWalletSession()
          ..composeThrows = err(const WalletErrorKind.addressInvalid());
        final c = await atAmountEntry(fake);

        await ctl(c).prepare('1');
        expect(st(c), isA<MoveAmountEntry>());
        expect((st(c) as MoveAmountEntry).fault, isA<SendCategoricalFault>());
        expect(fake.proposeCount, 0); // compose failed before propose
      },
    );

    test(
      'double-tap is interlocked — a second prepare cannot kick off a 2nd propose',
      () async {
        final fake = FakeWalletSession()
          ..proposeResult = sendProposalFixture(proposalId: 9);
        final c = await atAmountEntry(fake);

        // Fire two prepares without awaiting the first: the first transitions to
        // MovePreparing synchronously (before its propose await), so the second
        // sees state != MoveAmountEntry and no-ops.
        final f1 = ctl(c).prepare('1');
        final f2 = ctl(c).prepare('1');
        await Future.wait([f1, f2]);

        expect(st(c), isA<MoveReady>());
        expect(fake.composeCount, 1);
        expect(fake.proposeCount, 1);
      },
    );
  });

  group('confirm', () {
    Future<ProviderContainer> atReady(FakeWalletSession fake) async {
      final h = harness(session: fake);
      await ctl(h.container).start();
      await ctl(h.container).prepare('1');
      expect(st(h.container), isA<MoveReady>());
      return h.container;
    }

    test(
      'every tx broadcast → MoveSent(SendSucceeded), consuming the token by id',
      () async {
        final fake = FakeWalletSession()
          ..proposeResult = sendProposalFixture(proposalId: 21)
          ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
        final c = await atReady(fake);

        await ctl(c).confirm();
        expect(st(c), isA<MoveSent>());
        expect((st(c) as MoveSent).outcome, isA<SendSucceeded>());
        expect(fake.sendCount, 1);
        expect(fake.lastSendProposalId, 21);
      },
    );

    test(
      'a partial/failed broadcast is DATA → MoveSent(SendSavedForRetry), no fund loss',
      () async {
        final fake = FakeWalletSession()
          ..sendResults = const [
            TxSubmitResult.success(txidHex: 'aa'),
            TxSubmitResult.grpcFailure(txidHex: 'bb'),
          ];
        final c = await atReady(fake);

        await ctl(c).confirm();
        expect((st(c) as MoveSent).outcome, isA<SendSavedForRetry>());
      },
    );

    test(
      'already-consumed token → MoveSent(SendAlreadySubmitted), never sent twice',
      () async {
        final fake = FakeWalletSession()
          ..sendThrows = err(const WalletErrorKind.proposalAlreadyUsed());
        final c = await atReady(fake);

        await ctl(c).confirm();
        expect((st(c) as MoveSent).outcome, isA<SendAlreadySubmitted>());
      },
    );

    test(
      'a stale anchor on send → back to MoveAmountEntry(amountsExpired) to re-propose',
      () async {
        final fake = FakeWalletSession()
          ..sendThrows = err(const WalletErrorKind.proposalStale());
        final c = await atReady(fake);

        await ctl(c).confirm();
        final s = st(c);
        expect(s, isA<MoveAmountEntry>());
        expect((s as MoveAmountEntry).fault, isA<SendCategoricalFault>());
        expect(
          (s.fault as SendCategoricalFault).reason,
          SendFaultReason.amountsExpired,
        );
      },
    );

    test('a sign failure → MoveSent(SendSignFailed); no money moved', () async {
      final fake = FakeWalletSession()
        ..sendThrows = err(const WalletErrorKind.signFailed());
      final c = await atReady(fake);

      await ctl(c).confirm();
      expect((st(c) as MoveSent).outcome, isA<SendSignFailed>());
    });

    test(
      'an out-of-disk on send → back to MoveAmountEntry(storageFull) "free '
      'up space", never the terminal Couldn\'t-move dead-end (#373 follow-up)',
      () async {
        final fake = FakeWalletSession()
          ..sendThrows = err(const WalletErrorKind.diskFull());
        final c = await atReady(fake);

        await ctl(c).confirm();
        final s = st(c);
        expect(s, isA<MoveAmountEntry>());
        expect(
          ((s as MoveAmountEntry).fault as SendCategoricalFault).reason,
          SendFaultReason.storageFull,
        );
      },
    );

    test(
      'wallet busy on send → back to MoveAmountEntry(walletBusy), retryable',
      () async {
        final fake = FakeWalletSession()
          ..sendThrows = err(
            const WalletErrorKind.walletBusy(phase: LifecyclePhase.closing),
          );
        final c = await atReady(fake);

        await ctl(c).confirm();
        final s = st(c);
        expect(s, isA<MoveAmountEntry>());
        expect((s as MoveAmountEntry).fault, isA<SendCategoricalFault>());
        expect(
          (s.fault as SendCategoricalFault).reason,
          SendFaultReason.walletBusy,
        );
      },
    );

    test(
      'a successful move invalidates the cold snapshot (shielded balance dropped)',
      () async {
        final fake = FakeWalletSession()
          ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
        final h = harness(session: fake);
        // Keep the snapshot provider alive so an invalidate triggers a re-fetch.
        h.container.listen(
          walletSnapshotProvider,
          (_, _) {},
          fireImmediately: true,
        );
        await Future<void>.delayed(Duration.zero); // the initial fetch
        final before = fake.snapshotCount;

        await ctl(h.container).start();
        await ctl(h.container).prepare('1');
        await ctl(h.container).confirm();
        await Future<void>.delayed(Duration.zero); // let the re-fetch run

        expect(
          fake.snapshotCount,
          greaterThan(before),
          reason: 'the balance card must refresh after funds leave the pool',
        );
      },
    );

    test(
      'an already-submitted outcome ALSO invalidates the snapshot (funds moved)',
      () async {
        final fake = FakeWalletSession()
          ..sendThrows = err(const WalletErrorKind.proposalAlreadyUsed());
        final h = harness(session: fake);
        h.container.listen(
          walletSnapshotProvider,
          (_, _) {},
          fireImmediately: true,
        );
        await Future<void>.delayed(Duration.zero);
        final before = fake.snapshotCount;

        await ctl(h.container).start();
        await ctl(h.container).prepare('1');
        await ctl(h.container).confirm();
        await Future<void>.delayed(Duration.zero);

        expect(
          (st(h.container) as MoveSent).outcome,
          isA<SendAlreadySubmitted>(),
        );
        expect(
          fake.snapshotCount,
          greaterThan(before),
          reason: 'a prior in-flight send already moved the funds',
        );
      },
    );
  });

  group('lifecycle safety', () {
    test(
      'retryLoad is a no-op except from MoveUnavailable (can\'t clobber a flow)',
      () async {
        final fake = FakeWalletSession();
        final h = harness(session: fake);
        await ctl(h.container).start();
        await ctl(h.container).prepare('1');
        expect(st(h.container), isA<MoveReady>());

        await ctl(h.container).retryLoad(); // from MoveReady — must no-op
        expect(
          st(h.container),
          isA<MoveReady>(),
          reason: 'a stray retry must not reset an in-flight/ready flow',
        );
      },
    );

    test(
      'dispose during the address-load await never mutates state nor throws',
      () async {
        final h = harness();
        // Kick off start() (awaits the address future, pending on a microtask) but
        // DON'T await it; dispose the container before it resolves.
        final pending = ctl(h.container).start();
        h.container.dispose();
        // The future resolves post-dispose; the _disposed guard must swallow the
        // state mutation so nothing throws.
        await expectLater(pending, completes);
      },
    );

    test(
      'dispose during the in-flight send await never mutates state nor throws',
      () async {
        final fake = FakeWalletSession()
          ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
        final h = harness(session: fake);
        await ctl(h.container).start();
        await ctl(h.container).prepare('1');
        expect(st(h.container), isA<MoveReady>());

        // The money-critical await: confirm() is in flight (send pending on a
        // microtask) when the container disposes.
        final pending = ctl(h.container).confirm();
        h.container.dispose();
        await expectLater(pending, completes);
      },
    );

    test(
      'a wallet-session change does not wedge the controller (_disposed reset)',
      () async {
        // A session flip RE-RUNS build() on the same notifier; without resetting
        // _disposed each build it would stick true and silently drop every later
        // _set. This pins the live-across-a-flip behavior.
        final fakeA = FakeWalletSession()
          ..currentTransparentAddressResult = 't1A';
        final container = ProviderContainer(
          overrides: [walletSessionProvider.overrideWithValue(fakeA)],
        );
        addTearDown(container.dispose);
        await container
            .read(moveToTransparentControllerProvider.notifier)
            .start();
        expect(
          container.read(moveToTransparentControllerProvider),
          isA<MoveAmountEntry>(),
        );

        // Flip the session (wallet re-opened) → build() re-runs.
        final fakeB = FakeWalletSession()
          ..currentTransparentAddressResult = 't1B';
        container.updateOverrides([
          walletSessionProvider.overrideWithValue(fakeB),
        ]);
        container.read(
          moveToTransparentControllerProvider,
        ); // force the rebuild

        // The controller is still LIVE: it loads the new session's address.
        await container
            .read(moveToTransparentControllerProvider.notifier)
            .start();
        final s = container.read(moveToTransparentControllerProvider);
        expect(s, isA<MoveAmountEntry>());
        expect((s as MoveAmountEntry).ownAddress, 't1B');
      },
    );
  });

  group('navigation', () {
    test(
      'backToForm from review returns to the amount entry, preserving the address',
      () async {
        final fake = FakeWalletSession()
          ..currentTransparentAddressResult = 't1myownTaddr';
        final h = harness(session: fake);
        await ctl(h.container).start();
        await ctl(h.container).prepare('1');
        expect(st(h.container), isA<MoveReady>());

        ctl(h.container).backToForm();
        final s = st(h.container);
        expect(s, isA<MoveAmountEntry>());
        expect((s as MoveAmountEntry).ownAddress, 't1myownTaddr');
        expect(s.fault, isNull); // a clean form, prior fault cleared
      },
    );

    test(
      'confirm is a no-op except from MoveReady (a stray tap can\'t double-send)',
      () async {
        final fake = FakeWalletSession();
        final h = harness(session: fake);
        await ctl(h.container).start(); // MoveAmountEntry, not review
        await ctl(h.container).confirm();
        expect(fake.sendCount, 0);
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

    test('confirm routes through the authorizer EXACTLY once as an UNSHIELD '
        'intent carrying the proposal total', () async {
      final h = authorized();
      h.fake.proposeResult = sendProposalFixture(
        proposalId: 6,
        totalZat: 120000,
      );
      await ctl(h.container).start();
      await ctl(h.container).prepare('0.0012');
      await ctl(h.container).confirm();
      expect(st(h.container), isA<MoveSent>());
      expect(h.auth.intents, hasLength(1));
      expect(h.auth.intents.single.kind, WalletSpendKind.unshield);
      expect(h.auth.intents.single.amountZat, 120000);
      // #383 R3 / field pins: the unshield pays the wallet's OWN
      // t-addr (funds never leave the wallet — the prompt may say so), and
      // the fee is the proposal's own figure.
      expect(h.auth.intents.single.recipientIsSelf, isTrue);
      expect(h.auth.intents.single.feeZat, h.fake.proposeResult!.feeZat);
      expect(h.fake.sendCount, 1);
    });

    test(
      'a DENIED confirm lands back on the review with ZERO bridge calls',
      () async {
        final h = authorized(denyAll: true);
        h.fake.proposeResult = sendProposalFixture(proposalId: 6);
        await ctl(h.container).start();
        await ctl(h.container).prepare('1');
        final ready = st(h.container) as MoveReady;
        await ctl(h.container).confirm();
        expect(st(h.container), same(ready));
        expect(h.fake.sendCount, 0);
      },
    );

    test('a denial landing AFTER a mid-prompt session swap never restores the '
        'dead session\'s review (S152 review H2 / #330 — the identity guard '
        'is load-bearing because this controller resets _disposed on '
        'rebuild)', () async {
      final fakeA = FakeWalletSession();
      final fakeB = FakeWalletSession();
      final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(
        denyAll: true,
        prompt: (_) => gate.future,
      );
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
          walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(container.dispose);
      // Keep the controller listened so the session-swap rebuild flushes.
      container.listen(moveToTransparentControllerProvider, (_, _) {});

      await ctl(container).start();
      await ctl(container).prepare('1');
      expect(st(container), isA<MoveReady>());
      final confirm = ctl(container).confirm(); // parks at the host prompt
      await Future<void>.delayed(Duration.zero);

      // The wallet session swaps mid-prompt — build() re-runs and resets the
      // machine to its fresh loading state.
      container.read(sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);
      expect(st(container), isA<MoveLoadingAddress>());

      // The parked prompt resolves as a denial: the restore must be SKIPPED
      // (writing the old MoveReady here would resurrect a dead session's
      // proposal over the fresh state).
      gate.complete();
      await confirm;
      expect(st(container), isA<MoveLoadingAddress>());
      expect(fakeA.sendCount, 0);
      expect(fakeB.sendCount, 0);
    });

    test(
      'an address load resolving AFTER a mid-flight session swap never '
      'writes the DEAD identity\'s own-address into the fresh cycle (#330 '
      '— a cross-identity de-shield destination would be a money hazard)',
      () async {
        final fakeA = FakeWalletSession()
          ..currentTransparentAddressGate = Completer<void>()
          ..currentTransparentAddressResult = 't1identityA';
        final fakeB = FakeWalletSession()
          ..currentTransparentAddressResult = 't1identityB';
        final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
        final container = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWith(
              (ref) => ref.watch(sessionSwitch),
            ),
          ],
        );
        addTearDown(container.dispose);
        container.listen(moveToTransparentControllerProvider, (_, _) {});

        final pending = ctl(container).start(); // parks at the address fetch
        expect(st(container), isA<MoveLoadingAddress>());

        container.read(sessionSwitch.notifier).state = fakeB;
        await Future<void>.delayed(Duration.zero);

        // Identity A's address arrives late — it must be dropped, not rendered
        // as identity B's de-shield destination.
        fakeA.currentTransparentAddressGate!.complete();
        await pending;
        expect(st(container), isA<MoveLoadingAddress>());

        // The sheet re-drives start() for the new session (its listen seam) —
        // and gets identity B's OWN address.
        await ctl(container).start();
        final s = st(container);
        expect(s, isA<MoveAmountEntry>());
        expect((s as MoveAmountEntry).ownAddress, 't1identityB');
      },
    );

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
        final fakeA = FakeWalletSession();
        final fakeB = FakeWalletSession();
        final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
        final container = ProviderContainer(
          overrides: [
            walletSessionProvider.overrideWith(
              (ref) => ref.watch(sessionSwitch),
            ),
            walletSendAuthorizerProvider.overrideWithValue(auth),
          ],
        );
        addTearDown(container.dispose);
        container.listen(moveToTransparentControllerProvider, (_, _) {});

        await ctl(container).start();
        await ctl(container).prepare('1');
        final confirmA = ctl(container).confirm(); // parks (call 1)

        container.read(sessionSwitch.notifier).state = fakeB;
        await Future<void>.delayed(Duration.zero);

        // Drive the NEW cycle into its own confirm-in-flight.
        await ctl(container).start();
        await ctl(container).prepare('1');
        unawaited(ctl(container).confirm()); // parks forever (call 2)
        final submittingB = st(container);
        expect(submittingB, isA<MoveSubmitting>());

        gateA
            .complete(); // a type-only guard would restore the DEAD review here
        await confirmA;
        expect(st(container), same(submittingB));
        expect(fakeA.sendCount, 0);
        expect(fakeB.sendCount, 0);
      },
    );

    test('an APPROVAL landing after a mid-prompt flip is FENCED — the move '
        'spends on NEITHER session (S153 wrap review)', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
      final fakeA = FakeWalletSession();
      final fakeB = FakeWalletSession();
      final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch)),
          walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(container.dispose);
      container.listen(moveToTransparentControllerProvider, (_, _) {});

      await ctl(container).start();
      await ctl(container).prepare('1');
      final confirm = ctl(container).confirm(); // parks at the host prompt
      expect(st(container), isA<MoveSubmitting>());

      container.read(sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      gate.complete(); // the prompt APPROVES after the identity switch
      await confirm;
      expect(st(container), isA<MoveLoadingAddress>());
      expect(
        fakeA.sendCount,
        0,
        reason: 'the fence refused the dead identity\'s spend',
      );
      expect(fakeB.sendCount, 0);
    });

    test('sheet re-entry mid-confirm RE-ATTACHES (S153 review F1): start() is '
        'a no-op while a sign is in flight, so the landing outcome still '
        'renders', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
      final fake = FakeWalletSession();
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(container.dispose);

      await ctl(container).start();
      await ctl(container).prepare('1');
      final confirm = ctl(container).confirm(); // parks at the host prompt
      final submitting = st(container);
      expect(submitting, isA<MoveSubmitting>());

      await ctl(container).start(); // the sheet's entry hook, re-opened
      expect(
        st(container),
        same(submitting),
        reason: 're-entry must not clobber an in-flight sign',
      );

      gate.complete();
      await confirm;
      expect(st(container), isA<MoveSent>());
      expect(fake.sendCount, 1);
    });
  });
}
