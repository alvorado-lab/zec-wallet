import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_controller.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/send/zec_amount.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Send state-machine tests (inc-2d-ui). The whole flow runs on the host VM
/// against `FakeWalletSession` behind the `WalletSession` port — no native
/// library, no device — which is exactly why the URI composition lives in the
/// adapter, not here (the controller never calls the native ZIP-321 encoder).
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

  SendController ctl(ProviderContainer c) =>
      c.read(sendControllerProvider.notifier);
  SendState st(ProviderContainer c) => c.read(sendControllerProvider);

  // A typed SDK error for a given kind (the shape the fakes throw).
  WalletApiError err(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

  group('prepare → review', () {
    test(
      'composes via the port then proposes, echoing the recipient',
      () async {
        final h = harness();
        h.fake.proposeResult = sendProposalFixture(
          proposalId: 7,
          totalZat: 100500,
        );
        await ctl(
          h.container,
        ).prepare(address: 'u1recipient', amountText: '0.001', memo: 'hi');
        final s = st(h.container);
        expect(s, isA<SendReview>());
        final review = s as SendReview;
        expect(review.recipient, 'u1recipient');
        expect(review.proposal.proposalId, 7);
        // Amount was parsed host-side to zatoshis BEFORE the port crossing.
        expect(h.fake.lastComposeAmountZat, 100000); // 0.001 ZEC
        expect(h.fake.lastComposeRecipient, 'u1recipient');
        expect(h.fake.lastComposeMemo, 'hi');
        // The proposal was lowered from the URI the adapter composed.
        expect(
          h.fake.lastProposeUri,
          h.fake.composePaymentUri(recipient: 'u1recipient', amountZat: 100000),
        );
        expect(h.fake.proposeCount, 1);
      },
    );

    test('a bad amount fails host-side — no bridge call at all', () async {
      final h = harness();
      await ctl(
        h.container,
      ).prepare(address: 'u1x', amountText: 'not-a-number');
      final s = st(h.container);
      expect(s, isA<SendForm>());
      expect((s as SendForm).fault, isA<SendAmountFault>());
      expect((s.fault as SendAmountFault).fault, ZecAmountFault.notANumber);
      // The amount never parsed, so nothing crossed the port.
      expect(h.fake.composeCount, 0);
      expect(h.fake.proposeCount, 0);
    });

    test('a zero amount is rejected before any bridge call', () async {
      final h = harness();
      await ctl(h.container).prepare(address: 'u1x', amountText: '0');
      final s = st(h.container) as SendForm;
      expect((s.fault as SendAmountFault).fault, ZecAmountFault.notPositive);
      expect(h.fake.composeCount, 0);
    });

    test(
      'a bad address fails at compose (typed), staying on the form',
      () async {
        final h = harness();
        h.fake.composeThrows = err(const WalletErrorKind.addressInvalid());
        await ctl(h.container).prepare(address: 'garbage', amountText: '1');
        final s = st(h.container) as SendForm;
        expect(
          (s.fault as SendCategoricalFault).reason,
          SendFaultReason.addressInvalid,
        );
        expect(h.fake.proposeCount, 0); // never reached propose
      },
    );

    test('insufficient funds carries the audited figures', () async {
      final h = harness();
      h.fake.proposeThrows = err(
        const WalletErrorKind.insufficientFunds(
          availableZat: 100000,
          requiredZat: 150000,
          pendingIncomingZat: 25000,
        ),
      );
      await ctl(h.container).prepare(address: 'u1x', amountText: '0.0015');
      final fault = (st(h.container) as SendForm).fault;
      expect(fault, isA<SendInsufficientFunds>());
      final f = fault as SendInsufficientFunds;
      expect(f.availableZat, 100000);
      expect(f.requiredZat, 150000);
      expect(f.pendingIncomingZat, 25000);
    });

    test('a not-anchorable wallet maps to the offline-fork fault', () async {
      final h = harness();
      h.fake.proposeThrows = err(const WalletErrorKind.proposalStale());
      await ctl(h.container).prepare(address: 'u1x', amountText: '1');
      final fault = (st(h.container) as SendForm).fault as SendCategoricalFault;
      expect(fault.reason, SendFaultReason.notSyncedYet);
    });

    test(
      'a pasted address with surrounding whitespace is trimmed before compose',
      () async {
        // Real-world: a paste from a chat/QR app carries leading/trailing spaces
        // (and a stray newline). The composed recipient — and the recipient echoed
        // back on the confirm screen for the user to verify — MUST be the trimmed
        // value, never the raw paste (an untrimmed address could fail compose or
        // pay a malformed string). The amount field is already trimmed by the
        // parser; the address trim lives in `prepare`, and only this asserts it.
        final h = harness();
        await ctl(
          h.container,
        ).prepare(address: '  u1recipient\n ', amountText: '1');
        expect(h.fake.lastComposeRecipient, 'u1recipient');
        // And the echoed-back recipient on the review screen is the trimmed value,
        // so the user verifies the address that will actually be paid.
        expect((st(h.container) as SendReview).recipient, 'u1recipient');
      },
    );
  });

  group('confirm → result', () {
    Future<ProviderContainer> reviewing(FakeWalletSession fake) async {
      final h = harness(session: fake);
      await ctl(h.container).prepare(address: 'u1x', amountText: '1');
      expect(st(h.container), isA<SendReview>());
      return h.container;
    }

    test('all-success → sent', () async {
      final fake = FakeWalletSession()
        ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
      final c = await reviewing(fake);
      await ctl(c).confirm();
      final s = st(c) as SendSent;
      expect(s.outcome, isA<SendSucceeded>());
      expect((s.outcome as SendSucceeded).txCount, 1);
      expect(fake.lastSendProposalId, 1); // the proposal id round-tripped
    });

    test('outcome-landing refreshes the in-flight cue (#309) — every result-'
        'screen exit is covered, including the system back gesture', () async {
      // The durable "don't send it again" cue must be fresh the moment the
      // outcome lands, NOT only when a specific button is pressed: the user can
      // leave the result screen via Done, "Send another", OR the system back /
      // AppBar arrow (which fire no invalidation), and the next sync edge can
      // be a block away. Pin: confirm() itself invalidates the provider.
      final fake = FakeWalletSession()
        ..sendResults = const [
          TxSubmitResult.success(txidHex: 'tx0'),
          TxSubmitResult.grpcFailure(txidHex: 'tx1'),
        ];
      final c = await reviewing(fake);
      // Keep the provider ALIVE (the wallet screen beneath the send route
      // watches it in production) so invalidate triggers a live re-read.
      c.listen(walletInFlightSendsProvider, (_, _) {});
      await pumpEventQueue();
      expect(fake.listInFlightSendsCount, 1, reason: 'the mount read');

      await ctl(c).confirm();
      await pumpEventQueue();
      expect(
        fake.listInFlightSendsCount,
        2,
        reason: 'confirm() re-pulled the cue at outcome-landing',
      );
    });

    test(
      'outcome-landing refreshes the BALANCE snapshot too (S153 wrap '
      'review) — the in-flight cue is two-step-only, so for a plain '
      'shielded send the balance is the only away-at-landing signal',
      () async {
        final fake = FakeWalletSession()
          ..sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
        final c = await reviewing(fake);
        c.listen(walletSnapshotProvider, (_, _) {});
        await Future<void>.delayed(Duration.zero); // the mount read
        final before = fake.snapshotCount;

        await ctl(c).confirm();
        await Future<void>.delayed(Duration.zero); // let the re-fetch run
        expect(
          fake.snapshotCount,
          greaterThan(before),
          reason: 'confirm() re-pulled the balance at outcome-landing',
        );
      },
    );

    test('a broadcast failure is saved-for-retry, never lost', () async {
      final fake = FakeWalletSession()
        ..sendResults = const [
          TxSubmitResult.success(txidHex: 'aa'),
          TxSubmitResult.grpcFailure(txidHex: 'bb'),
        ];
      final c = await reviewing(fake);
      await ctl(c).confirm();
      final outcome = (st(c) as SendSent).outcome;
      expect(outcome, isA<SendSavedForRetry>());
      final r = outcome as SendSavedForRetry;
      expect(r.broadcast, 1);
      expect(r.total, 2);
    });

    test(
      'a double-tapped token surfaces as already-submitted (no double-send)',
      () async {
        final fake = FakeWalletSession()
          ..sendThrows = err(const WalletErrorKind.proposalAlreadyUsed());
        final c = await reviewing(fake);
        await ctl(c).confirm();
        expect((st(c) as SendSent).outcome, isA<SendAlreadySubmitted>());
      },
    );

    test('a stale anchor on send routes back to the form with amounts-expired '
        '(the numbers aged out, NOT a sync deficiency)', () async {
      final fake = FakeWalletSession()
        ..sendThrows = err(const WalletErrorKind.proposalStale());
      final c = await reviewing(fake);
      await ctl(c).confirm();
      final s = st(c);
      expect(s, isA<SendForm>());
      expect(
        ((s as SendForm).fault as SendCategoricalFault).reason,
        SendFaultReason.amountsExpired,
      );
    });

    test(
      'FR-46: the reported recipient amount is the one the SENT proposal '
      'carries, after a stale-anchor re-propose changed the numbers',
      () async {
        final fake = FakeWalletSession()
          ..proposeResult = sendProposalFixture(
            proposalId: 1,
            singleRecipientZat: 111000,
          )
          ..sendThrows = err(const WalletErrorKind.proposalStale());
        final c = await reviewing(fake);
        await ctl(c).confirm();
        expect(st(c), isA<SendForm>(), reason: 'the stale anchor sent nothing');
        expect(c.read(sendFlowOutcomeProvider), isNot(isA<SendFlowSent>()));

        fake
          ..proposeResult = sendProposalFixture(
            proposalId: 2,
            singleRecipientZat: 222000,
          )
          ..sendThrows = null;
        await ctl(c).prepare(address: 'u1x', amountText: '1');
        await ctl(c).confirm();
        expect(fake.lastSendProposalId, 2);
        final flow = c.read(sendFlowOutcomeProvider) as SendFlowSent;
        expect(flow.singleRecipientZat, 222000);
      },
    );

    test('a sign failure → couldn\'t-complete (try again)', () async {
      final fake = FakeWalletSession()
        ..sendThrows = err(const WalletErrorKind.signFailed());
      final c = await reviewing(fake);
      await ctl(c).confirm();
      expect((st(c) as SendSent).outcome, isA<SendSignFailed>());
    });

    test('confirm is a no-op off the review screen', () async {
      final h = harness();
      await ctl(h.container).confirm(); // from the initial SendForm
      expect(st(h.container), isA<SendForm>());
      expect(h.fake.sendCount, 0);
    });
  });

  group('offline-first queue', () {
    test('queueOffline durably queues and lands on queued', () async {
      final h = harness();
      await ctl(
        h.container,
      ).queueOffline(address: 'u1x', amountText: '0.5', memo: null);
      expect(st(h.container), isA<SendQueued>());
      expect(h.fake.queueCount, 1);
      expect(h.fake.lastQueueUri, isNotNull);
    });

    test('queue-landing refreshes the parked surface (#309 H2) — queueing is '
        'OFFLINE, so no sync edge will do it', () async {
      // Without this, a freshly queued TEX is invisible on the wallet screen
      // until app resume — the double-QUEUE temptation (both drain when
      // connectivity returns → double pay).
      final h = harness();
      h.container.listen(walletParkedSendsProvider, (_, _) {});
      await pumpEventQueue();
      expect(h.fake.listParkedSendsCount, 1, reason: 'the mount read');

      await ctl(
        h.container,
      ).queueOffline(address: 'u1x', amountText: '0.5', memo: null);
      await pumpEventQueue();
      expect(
        h.fake.listParkedSendsCount,
        2,
        reason: 'queueOffline re-pulled the parked surface at landing',
      );
    });

    test('a full queue is an honest fault, never a silent drop', () async {
      final h = harness();
      h.fake.queueThrows = err(const WalletErrorKind.queuedSendsFull());
      await ctl(h.container).queueOffline(address: 'u1x', amountText: '1');
      final fault = (st(h.container) as SendForm).fault as SendCategoricalFault;
      expect(fault.reason, SendFaultReason.queueFull);
    });

    test('a bad amount is rejected before queuing', () async {
      final h = harness();
      await ctl(h.container).queueOffline(address: 'u1x', amountText: '');
      expect((st(h.container) as SendForm).fault, isA<SendAmountFault>());
      expect(h.fake.queueCount, 0);
    });
  });

  group('re-entrancy + lifecycle', () {
    test('double-tap prepare proposes exactly once', () async {
      final h = harness();
      // Both calls start synchronously; the first sets SendPreparing before its
      // first await, so the second sees the in-flight guard and no-ops.
      final f1 = ctl(h.container).prepare(address: 'u1x', amountText: '1');
      final f2 = ctl(h.container).prepare(address: 'u1x', amountText: '1');
      await Future.wait([f1, f2]);
      expect(h.fake.composeCount, 1);
      expect(h.fake.proposeCount, 1);
    });

    test('no live session → an honest unavailable fault', () async {
      final h = harness(nullSession: true);
      await ctl(h.container).prepare(address: 'u1x', amountText: '1');
      final fault = (st(h.container) as SendForm).fault as SendCategoricalFault;
      expect(fault.reason, SendFaultReason.walletUnavailable);
    });

    test('backToForm / resetToForm return to a clean form', () async {
      final h = harness();
      await ctl(h.container).prepare(address: 'u1x', amountText: '1');
      expect(st(h.container), isA<SendReview>());
      ctl(h.container).backToForm();
      expect(st(h.container), isA<SendForm>());
      expect((st(h.container) as SendForm).fault, isNull);

      // From a terminal result, resetToForm clears it.
      h.fake.sendResults = const [TxSubmitResult.success(txidHex: 'aa')];
      await ctl(h.container).prepare(address: 'u1x', amountText: '1');
      await ctl(h.container).confirm();
      expect(st(h.container), isA<SendSent>());
      ctl(h.container).resetToForm();
      expect(st(h.container), isA<SendForm>());
    });
  });

  group('the HOST send ceiling (walletSendCeilingZatProvider — S151 seam)', () {
    // A capped harness: the host wired a policy ceiling (e.g. a 1-ZEC alpha
    // cap). The default-null case (no ceiling) is exercised by every other
    // test in this file — none of them override the seam.
    ({ProviderContainer container, FakeWalletSession fake}) capped(
      int ceilingZat,
    ) {
      final fake = FakeWalletSession();
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendCeilingZatProvider.overrideWithValue(ceilingZat),
        ],
      );
      addTearDown(container.dispose);
      return (container: container, fake: fake);
    }

    test('an over-ceiling amount is refused BEFORE any bridge call, carrying '
        'the limit for the honest copy', () async {
      final h = capped(100000000); // 1 ZEC
      await ctl(h.container).prepare(address: 'u1x', amountText: '1.00000001');
      final s = st(h.container);
      expect(s, isA<SendForm>());
      final fault = (s as SendForm).fault;
      expect(fault, isA<SendOverCeiling>());
      expect((fault as SendOverCeiling).ceilingZat, 100000000);
      // Policy-refused host-side: nothing crossed the port.
      expect(h.fake.composeCount, 0);
      expect(h.fake.proposeCount, 0);
    });

    test('exactly the ceiling passes — the bound is inclusive (an "up to '
        '1 ZEC" policy must allow 1 ZEC)', () async {
      final h = capped(100000000);
      h.fake.proposeResult = sendProposalFixture(
        proposalId: 1,
        totalZat: 100000000,
      );
      await ctl(h.container).prepare(address: 'u1x', amountText: '1');
      expect(st(h.container), isA<SendReview>());
    });

    test('the OFFLINE queue path enforces the SAME ceiling — a cap that only '
        'guarded the online path would be a bypass', () async {
      final h = capped(100000000);
      await ctl(h.container).queueOffline(address: 'u1x', amountText: '2');
      final fault = (st(h.container) as SendForm).fault;
      expect(fault, isA<SendOverCeiling>());
      expect(h.fake.composeCount, 0);
      expect(h.fake.queueCount, 0);
    });
  });

  group(
    'the HOST send authorizer (walletSendAuthorizerProvider — #327 seam)',
    () {
      // An authorized harness: the host wired per-send authorization. The
      // default pass-through is exercised by every other test in this file —
      // none of them override the seam, and all their sends succeed.
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

      test('confirm routes through the authorizer EXACTLY once, carrying the '
          'send kind + the proposal total for the host prompt', () async {
        final h = authorized();
        h.fake.proposeResult = sendProposalFixture(
          proposalId: 3,
          totalZat: 250000,
        );
        await ctl(h.container).prepare(address: 'u1x', amountText: '0.0025');
        await ctl(h.container).confirm();
        expect(st(h.container), isA<SendSent>());
        expect(
          h.auth.intents,
          hasLength(1),
          reason: 'one confirmed action ⇒ one authorization',
        );
        expect(h.auth.intents.single.kind, WalletSpendKind.send);
        expect(h.auth.intents.single.amountZat, 250000);
        // #383 R3: the display-facts a per-spend prompt binds confirm→sign
        // to — the elided recipient, the proposal's own fee, and the
        // proposal's self-send verdict (an ordinary external send here).
        expect(
          h.auth.intents.single.recipientAbbrev,
          abbreviateWalletAddress('u1x'),
        );
        expect(
          h.auth.intents.single.recipientIsSelf,
          h.fake.proposeResult!.selfSend,
        );
        expect(h.auth.intents.single.feeZat, h.fake.proposeResult!.feeZat);
        expect(h.fake.sendCount, 1);
        // Propose is deterministic + local (nothing signs) — never authorized.
        expect(h.fake.proposeCount, 1);
      });

      test("confirm's intent carries the proposal's spend-binding nonce "
          'byte-for-byte (FR-17 #396) — a host-custody supplier records it at '
          'stage time and fail-closes any other sign', () async {
        final h = authorized();
        // A distinct fill (not the fixture default) so a pass-through of the
        // WRONG proposal's bytes cannot pass by coincidence.
        final binding = Uint8List.fromList(List.filled(32, 0xA5));
        h.fake.proposeResult = sendProposalFixture(
          proposalId: 13,
          binding: binding,
        );
        await ctl(h.container).prepare(address: 'u1x', amountText: '0.001');
        await ctl(h.container).confirm();
        expect(st(h.container), isA<SendSent>());
        expect(
          h.auth.intents.single.bindingToken,
          equals(binding),
          reason:
              'the seam must surface the EXACT nonce the sign-time seed '
              'pull will present',
        );
      });

      test('a DENIED confirm lands back on review with ZERO bridge calls — the '
          'one-shot token stays unconsumed and re-confirmable', () async {
        final h = authorized(denyAll: true);
        h.fake.proposeResult = sendProposalFixture(
          proposalId: 5,
          totalZat: 100,
        );
        await ctl(h.container).prepare(address: 'u1x', amountText: '0.000001');
        final review = st(h.container) as SendReview;
        await ctl(h.container).confirm();
        // Back on the SAME review (proposal + recipient intact) — the user can
        // simply confirm again; the host's own prompt was the communication.
        expect(st(h.container), same(review));
        expect(h.fake.sendCount, 0);
        // And a re-confirm after the host re-authorizes goes through.
        h.auth.denyAll = false;
        await ctl(h.container).confirm();
        expect(st(h.container), isA<SendSent>());
        expect(h.fake.sendCount, 1);
      });

      test(
        'the offline queue is authorized as a QUEUED send (the commit moment '
        '— signing is drain-time) with the parsed amount',
        () async {
          final h = authorized();
          await ctl(
            h.container,
          ).queueOffline(address: 'u1x', amountText: '0.5');
          expect(st(h.container), isA<SendQueued>());
          expect(h.auth.intents, hasLength(1));
          expect(h.auth.intents.single.kind, WalletSpendKind.queuedSend);
          expect(h.auth.intents.single.amountZat, 50000000);
          // #383 R3 / field pin: what is known at QUEUE time — the
          // TYPED recipient, elided. No proposal exists yet (signing is
          // drain-time), so this is the one display-fact the prompt gets.
          expect(
            h.auth.intents.single.recipientAbbrev,
            abbreviateWalletAddress('u1x'),
          );
          // FR-17 (#396): queue-time authorization carries NO binding — it is
          // minted at enqueue and surfaced on the parked row for a re-stage.
          expect(h.auth.intents.single.bindingToken, isNull);
          expect(h.fake.queueCount, 1);
        },
      );

      test('a DENIED queue persists NOTHING and returns to the form without a '
          'fault banner (the host prompt was the communication)', () async {
        final h = authorized(denyAll: true);
        await ctl(h.container).queueOffline(address: 'u1x', amountText: '1');
        final s = st(h.container);
        expect(s, isA<SendForm>());
        expect((s as SendForm).fault, isNull);
        expect(h.fake.queueCount, 0);
      });

      test('a DENIED queue PRESERVES the entry fault — a notSynced fault is '
          'what offered the queue button, so dropping it would hide the '
          'affordance the user just used (S152 wrap review)', () async {
        final h = authorized(denyAll: true);
        // Land on the form with the notSynced fault (the queue-offering state).
        h.fake.proposeThrows = err(const WalletErrorKind.proposalStale());
        await ctl(h.container).prepare(address: 'u1x', amountText: '1');
        final entry = st(h.container) as SendForm;
        expect(
          (entry.fault as SendCategoricalFault).reason,
          SendFaultReason.notSyncedYet,
        );
        // Queue from that state; the host prompt is dismissed.
        await ctl(h.container).queueOffline(address: 'u1x', amountText: '1');
        expect(
          st(h.container),
          same(entry),
          reason: 'the exact entry form, fault included, is restored',
        );
        expect(h.fake.queueCount, 0);
      });

      // (The session-flip races on this seam live in their own group below.)

      test('an authorizer failure that is NOT a denial classifies like a real '
          'signing failure — never silently swallowed', () async {
        // A host authorizer whose credential-staging step fails (not a user
        // cancel) must surface as an honest fault, not a silent review bounce.
        final h = authorized();
        h.fake.proposeResult = sendProposalFixture(
          proposalId: 9,
          totalZat: 100,
        );
        h.fake.sendThrows = err(const WalletErrorKind.signFailed());
        await ctl(h.container).prepare(address: 'u1x', amountText: '0.000001');
        await ctl(h.container).confirm();
        // The SDK's own error classified exactly as without the seam — the
        // authorizer passed it through untouched.
        expect((st(h.container) as SendSent).outcome, isA<SendSignFailed>());
        expect(
          h.auth.intents,
          hasLength(1),
          reason: 'the action ran (and failed) INSIDE the authorization',
        );
      });

      test('an authorizer failure BEFORE the action (a credential step, not a '
          'cancel) classifies honestly with ZERO bridge calls — never a wedged '
          'Submitting', () async {
        // The contract's other leg (review H3): the host's staging step
        // throws, so the action NEVER runs. "Couldn't complete; nothing was
        // sent" is the true story here, and the flow must leave the transient.
        final h = authorized();
        h.fake.proposeResult = sendProposalFixture(
          proposalId: 11,
          totalZat: 100,
        );
        await ctl(h.container).prepare(address: 'u1x', amountText: '0.000001');
        h.auth.throwBeforeAction = StateError('credential staging failed');
        await ctl(h.container).confirm();
        expect((st(h.container) as SendSent).outcome, isA<SendSignFailed>());
        expect(h.auth.intents, hasLength(1));
        expect(
          h.fake.sendCount,
          0,
          reason: 'the failure preceded the action — nothing crossed the port',
        );
      });
    },
  );

  group('session-flip liveness (#330)', () {
    // A host identity switch legitimately flips walletSessionProvider, which
    // RE-RUNS build() on the SAME (riverpod-reused) notifier. These pin the
    // two halves of the fix: `_disposed` resets each build (the controller
    // stays LIVE after a flip), and every post-await continuation guards on
    // the INSTANCE identity of its own transient (the dead cycle writes
    // NOTHING into the new one).
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
      // Keep the controller listened so the session-flip rebuild flushes.
      container.listen(sendControllerProvider, (_, _) {});
      return (container: container, sessionSwitch: sessionSwitch);
    }

    test('ONE session flip does not wedge the controller — a send on the NEW '
        'session still renders (the arm that used to stick _disposed true '
        'forever)', () async {
      final fakeA = FakeWalletSession();
      final fakeB = FakeWalletSession()
        ..proposeResult = sendProposalFixture(proposalId: 42);
      final h = flippable(fakeA);

      // The flip: build() re-runs on the same notifier (onDispose fired).
      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      await ctl(h.container).prepare(address: 'u1x', amountText: '1');
      final s = st(h.container);
      expect(
        s,
        isA<SendReview>(),
        reason: 'one identity switch must not leave the send screen dead',
      );
      expect((s as SendReview).proposal.proposalId, 42);
      expect(fakeB.proposeCount, 1);
      expect(fakeA.proposeCount, 0);
    });

    test('a propose resolving AFTER a mid-flight flip writes NOTHING into the '
        'new cycle — and the new session then works end-to-end', () async {
      final fakeA = FakeWalletSession()
        ..proposeGate = Completer<void>()
        ..proposeResult = sendProposalFixture(proposalId: 1);
      final fakeB = FakeWalletSession()
        ..proposeResult = sendProposalFixture(proposalId: 2);
      final h = flippable(fakeA);

      final pending = ctl(h.container).prepare(address: 'u1a', amountText: '1');
      expect(st(h.container), isA<SendPreparing>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);
      expect(
        st(h.container),
        isA<SendForm>(),
        reason: 'the flip resets the machine to a clean form',
      );

      fakeA.proposeGate!.complete();
      await pending;
      final s = st(h.container);
      expect(s, isA<SendForm>());
      expect(
        (s as SendForm).fault,
        isNull,
        reason: 'the dead cycle\'s proposal must never surface',
      );

      await ctl(h.container).prepare(address: 'u1b', amountText: '1');
      expect((st(h.container) as SendReview).proposal.proposalId, 2);
    });

    test('an APPROVAL landing after a mid-prompt flip is FENCED — the spend '
        'runs on NEITHER session (S153: a dead identity\'s money must never '
        'move with no surface able to show it)', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
      final fakeA = FakeWalletSession()
        ..proposeResult = sendProposalFixture(proposalId: 5);
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA, auth: auth);

      await ctl(h.container).prepare(address: 'u1a', amountText: '1');
      final confirm = ctl(h.container).confirm(); // parks at the host prompt
      expect(st(h.container), isA<SendSubmitting>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      gate.complete(); // the prompt approves AFTER the identity switch
      await confirm;
      expect(
        st(h.container),
        isA<SendForm>(),
        reason: 'no stale result screen over the fresh cycle',
      );
      expect(
        fakeA.sendCount,
        0,
        reason: 'the fence refused the dead identity\'s spend',
      );
      expect(
        fakeB.sendCount,
        0,
        reason: 'and it never re-targets the new identity',
      );
    });

    test('a denial landing while a NEW confirm is already Submitting restores '
        'NOTHING — the guard matches instance identity, not type', () async {
      // Call 1 (old cycle): parks, then denies late. Call 2 (new cycle):
      // parks forever — its own Submitting must survive the late denial.
      final gateA = Completer<void>();
      var calls = 0;
      final auth = FakeSendAuthorizer(
        prompt: (intent) async {
          calls++;
          if (calls == 1) {
            await gateA.future;
            throw const WalletSpendAuthorizationDenied();
          }
          await Completer<void>().future;
        },
      );
      final fakeA = FakeWalletSession()
        ..proposeResult = sendProposalFixture(proposalId: 7);
      final fakeB = FakeWalletSession()
        ..proposeResult = sendProposalFixture(proposalId: 8);
      final h = flippable(fakeA, auth: auth);

      await ctl(h.container).prepare(address: 'u1a', amountText: '1');
      final confirmA = ctl(h.container).confirm(); // parks (call 1)

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      // Drive the NEW cycle all the way into its own confirm-in-flight.
      await ctl(h.container).prepare(address: 'u1b', amountText: '1');
      unawaited(ctl(h.container).confirm()); // parks forever (call 2)
      final submittingB = st(h.container);
      expect(submittingB, isA<SendSubmitting>());

      // The OLD denial lands. A type-only guard would "restore" the DEAD
      // session's review right here, over the live confirm.
      gateA.complete();
      await confirmA;
      expect(
        st(h.container),
        same(submittingB),
        reason: 'the new cycle\'s own Submitting must survive untouched',
      );
      expect(fakeA.sendCount, 0);
      expect(fakeB.sendCount, 0);
    });

    test('a dead cycle\'s propose resolving while the NEW cycle is ITSELF '
        'Preparing writes nothing — the transients are distinct INSTANCES, '
        'not a canonical const (S153 review F2; this is the pin the const '
        'mutation survived without)', () async {
      final fakeA = FakeWalletSession()
        ..proposeGate = Completer<void>()
        ..proposeResult = sendProposalFixture(proposalId: 1);
      final fakeB = FakeWalletSession()
        ..proposeGate = Completer<void>()
        ..proposeResult = sendProposalFixture(proposalId: 2);
      final h = flippable(fakeA);

      final pendingA = ctl(
        h.container,
      ).prepare(address: 'u1a', amountText: '1');
      expect(st(h.container), isA<SendPreparing>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      // The NEW cycle starts its OWN prepare and parks mid-propose too — the
      // exact shape a canonicalized const transient would alias.
      final pendingB = ctl(
        h.container,
      ).prepare(address: 'u1b', amountText: '2');
      final preparingB = st(h.container);
      expect(preparingB, isA<SendPreparing>());

      fakeA.proposeGate!.complete(); // the DEAD cycle's propose resolves first
      await pendingA;
      expect(
        st(h.container),
        same(preparingB),
        reason:
            'the dead session\'s proposal must NOT render as a live '
            'review while the new cycle is still preparing',
      );

      fakeB.proposeGate!.complete();
      await pendingB;
      expect(
        (st(h.container) as SendReview).proposal.proposalId,
        2,
        reason: 'the NEW session\'s own proposal is what renders',
      );
    });

    test('a dead cycle\'s queue outcome landing while the NEW cycle is ITSELF '
        'Queuing writes nothing (the same instance-identity pin for the '
        'queue transient)', () async {
      // Call 1 (old cycle): parks, then denies late. Call 2 (new cycle):
      // parks forever — its own Queuing must survive.
      final gateA = Completer<void>();
      var calls = 0;
      final auth = FakeSendAuthorizer(
        prompt: (intent) async {
          calls++;
          if (calls == 1) {
            await gateA.future;
            throw const WalletSpendAuthorizationDenied();
          }
          await Completer<void>().future;
        },
      );
      final fakeA = FakeWalletSession();
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA, auth: auth);

      final queueA = ctl(
        h.container,
      ).queueOffline(address: 'u1a', amountText: '1');
      expect(st(h.container), isA<SendQueuing>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      unawaited(ctl(h.container).queueOffline(address: 'u1b', amountText: '2'));
      final queuingB = st(h.container);
      expect(queuingB, isA<SendQueuing>());

      gateA.complete();
      await queueA;
      expect(
        st(h.container),
        same(queuingB),
        reason:
            'the new cycle\'s own Queuing must survive the dead '
            'cycle\'s late denial untouched',
      );
      expect(fakeA.queueCount, 0);
      expect(fakeB.queueCount, 0);
    });

    test('screen re-entry mid-confirm RE-ATTACHES (S153 review F1): the entry '
        'reset is a no-op in flight, so the landing outcome still renders — '
        'never a silently-swallowed completed send', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
      final fake = FakeWalletSession()
        ..proposeResult = sendProposalFixture(proposalId: 3);
      final h = flippable(fake, auth: auth);

      await ctl(h.container).prepare(address: 'u1a', amountText: '1');
      final confirm = ctl(h.container).confirm(); // parks at the host prompt
      final submitting = st(h.container);
      expect(submitting, isA<SendSubmitting>());

      // The user pops the screen and re-enters — the screen's entry hook.
      ctl(h.container).resetToForm();
      expect(
        st(h.container),
        same(submitting),
        reason: 're-entry must not clobber an in-flight submit',
      );

      gate.complete();
      await confirm;
      expect(
        st(h.container),
        isA<SendSent>(),
        reason:
            'the completed send RENDERS — pre-S153 the clobber made '
            'the identity guard swallow it in the re-pay window',
      );
      expect(fake.sendCount, 1);
    });

    test('a queue APPROVAL landing after a mid-prompt flip is FENCED — '
        'nothing is queued on EITHER session (S153 wrap review: a queuedSend '
        'has no one-shot token, so this fence is its only backstop)', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
      final fakeA = FakeWalletSession();
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA, auth: auth);

      final queue = ctl(
        h.container,
      ).queueOffline(address: 'u1a', amountText: '1');
      expect(st(h.container), isA<SendQueuing>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      gate.complete(); // the prompt APPROVES after the identity switch
      await queue;
      expect(st(h.container), isA<SendForm>());
      expect(
        fakeA.queueCount,
        0,
        reason: 'the fence refused the dead identity\'s queue commit',
      );
      expect(fakeB.queueCount, 0);
    });

    test('a queue denial landing after a mid-prompt flip restores nothing — '
        'not even the captured entry form', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(
        denyAll: true,
        prompt: (_) => gate.future,
      );
      // Enter the queue from a form carrying the notSynced fault (the state
      // whose restore the denial arm normally preserves).
      final fakeA = FakeWalletSession()
        ..proposeThrows = err(const WalletErrorKind.proposalStale());
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA, auth: auth);

      await ctl(h.container).prepare(address: 'u1a', amountText: '1');
      expect((st(h.container) as SendForm).fault, isNotNull);
      final queue = ctl(
        h.container,
      ).queueOffline(address: 'u1a', amountText: '1');
      expect(st(h.container), isA<SendQueuing>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      gate.complete(); // the parked prompt resolves as a denial
      await queue;
      final s = st(h.container);
      expect(s, isA<SendForm>());
      expect(
        (s as SendForm).fault,
        isNull,
        reason:
            'restoring the DEAD cycle\'s faulted entry form here would '
            'resurrect a stale affordance over the fresh session',
      );
      expect(fakeA.queueCount, 0);
      expect(fakeB.queueCount, 0);
    });
  });

  group('S13 H1 — an abandoned flow', () {
    test('its approved spend refuses, and the machine RESTARTS on a new flow '
        'with a visible fault — never a Review whose Confirm is refused '
        'forever (the S303 diff review MEDIUM)', () async {
      final h = harness();
      await ctl(h.container).prepare(address: 'u1r', amountText: '1');
      expect(st(h.container), isA<SendReview>());
      final abandoned = ctl(h.container).flowId;
      // The screen that drove it is gone.
      ctl(h.container).abandonFlow(abandoned);

      await ctl(h.container).confirm();
      expect(h.fake.sendCount, 0, reason: 'the abandoned flow never enters');
      final s = st(h.container);
      expect(s, isA<SendForm>());
      expect(
        ((s as SendForm).fault as SendCategoricalFault?)?.reason,
        SendFaultReason.amountsExpired,
        reason: 'the live screen is told to review again',
      );
      expect(ctl(h.container).flowId, isNot(abandoned));

      // A live screen's next Review + Confirm pays, under the new flow.
      await ctl(h.container).prepare(address: 'u1r', amountText: '1');
      await ctl(h.container).confirm();
      expect(h.fake.sendCount, 1);
    });

    test('the queue refuses the same way, and restarts', () async {
      final h = harness();
      final abandoned = ctl(h.container).flowId;
      ctl(h.container).abandonFlow(abandoned);
      await ctl(h.container).queueOffline(address: 'u1r', amountText: '1');
      expect(h.fake.queueCount, 0);
      expect(st(h.container), isA<SendForm>());
      expect(ctl(h.container).flowId, isNot(abandoned));
      await ctl(h.container).queueOffline(address: 'u1r', amountText: '1');
      expect(h.fake.queueCount, 1);
    });

    test('an abandoned flow never opens the host prompt — refused before the '
        'authorizer is asked (the S303 fold review LOW)', () async {
      final fake = FakeWalletSession();
      final prompt = _CountingPrompt();
      final c = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(prompt),
        ],
      );
      addTearDown(c.dispose);
      await ctl(c).prepare(address: 'u1r', amountText: '1');
      ctl(c).abandonFlow(ctl(c).flowId);
      await ctl(c).confirm();
      expect(prompt.asked, 0, reason: 'confirm: no prompt for a dead spend');
      ctl(c).abandonFlow(ctl(c).flowId);
      await ctl(c).queueOffline(address: 'u1r', amountText: '1');
      expect(prompt.asked, 0, reason: 'queue: the same');
      expect(fake.sendCount + fake.queueCount, 0);
    });

    test('the queue: a flow abandoned DURING the prompt is refused inside '
        'the closure, and nothing is committed', () async {
      final fake = FakeWalletSession();
      final prompt = _HeldPrompt();
      final c = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(prompt),
        ],
      );
      addTearDown(c.dispose);
      final flow = ctl(c).flowId;
      final queued = ctl(c).queueOffline(address: 'u1r', amountText: '1');
      await Future<void>.delayed(Duration.zero);
      ctl(c).abandonFlow(flow); // the screen went while the prompt was up
      prompt.approve();
      await queued;
      expect(fake.queueCount, 0);
      expect(ctl(c).flowId, isNot(flow), reason: 'restarted on a new flow');
    });

    test('abandoning a flow that already ENTERED changes nothing', () async {
      final h = harness();
      final gate = Completer<void>();
      h.fake.sendGate = gate;
      await ctl(h.container).prepare(address: 'u1r', amountText: '1');
      final flow = ctl(h.container).flowId;
      final confirm = ctl(h.container).confirm();
      await Future<void>.delayed(Duration.zero);
      expect(ctl(h.container).hasEnteredSpend(flow), isTrue);
      ctl(h.container).abandonFlow(flow);
      gate.complete();
      await confirm;
      expect(h.fake.sendCount, 1);
      expect(st(h.container), isA<SendSent>());
    });
  });

  group('S7 U1 — a spend that RAN and then lost its answer', () {
    // A host authorizer wraps the spend closure; one that runs it and then
    // throws (its own bookkeeping) or declines leaves money that may have
    // moved. The screen must say neither "nothing was sent" nor offer the
    // form/review back (a second tap pays twice), and the wallet surfaces
    // that show the truth must be re-read.
    ({ProviderContainer c, FakeWalletSession fake}) wrapped(
      WalletSendAuthorizer authorizer,
    ) {
      final fake = FakeWalletSession();
      final c = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(authorizer),
        ],
      );
      addTearDown(c.dispose);
      // Kept alive, as the wallet screen beneath the send route keeps them.
      c.listen(walletSnapshotProvider, (_, _) {});
      c.listen(walletInFlightSendsProvider, (_, _) {});
      c.listen(walletParkedSendsProvider, (_, _) {});
      return (c: c, fake: fake);
    }

    /// Runs [act], then asserts the landing: the unknown terminal (on the
    /// right path), the host told "indeterminate", and all three wallet
    /// surfaces re-read. The state is asserted FIRST — it is the claim.
    Future<void> landsUnknown(
      ({ProviderContainer c, FakeWalletSession fake}) h,
      Future<void> Function() act, {
      required bool queued,
    }) async {
      await pumpEventQueue();
      final snapshots = h.fake.snapshotCount;
      final inFlight = h.fake.listInFlightSendsCount;
      final parked = h.fake.listParkedSendsCount;
      await act();
      await pumpEventQueue();
      final s = st(h.c);
      expect(s, isA<SendOutcomeUnknown>());
      expect((s as SendOutcomeUnknown).queued, queued);
      expect(h.c.read(sendFlowOutcomeProvider), isA<SendFlowIndeterminate>());
      expect(h.fake.snapshotCount, greaterThan(snapshots), reason: 'balance');
      expect(h.fake.listInFlightSendsCount, greaterThan(inFlight));
      expect(h.fake.listParkedSendsCount, greaterThan(parked));
    }

    /// R13 §4.2's landed-results rule: the SDK call RETURNED before the host
    /// threw or declined, so the answer was not lost — the screen and the
    /// host are told what landed, never "unknown", and never the form or a
    /// live Review whose second tap would pay again.
    Future<void> landsLanded(
      ({ProviderContainer c, FakeWalletSession fake}) h,
      Future<void> Function() act, {
      required bool queued,
    }) async {
      await pumpEventQueue();
      await act();
      await pumpEventQueue();
      final s = st(h.c);
      final flow = h.c.read(sendFlowOutcomeProvider);
      if (queued) {
        // The id is the SDK's own return, never the authorizer's.
        expect((s as SendQueued).queuedSendId, h.fake.queuedId);
        expect((flow as SendFlowQueued).queuedSendId, h.fake.queuedId);
      } else {
        expect(s, isA<SendSent>());
        expect(flow, isA<SendFlowSent>());
      }
    }

    test('confirm: the host throws AFTER a send that returned → the landed '
        'outcome, never SendSignFailed or a live Review', () async {
      final h = wrapped(const _RunsThenThrows());
      await ctl(h.c).prepare(address: 'u1x', amountText: '1');
      await landsLanded(h, () => ctl(h.c).confirm(), queued: false);
      expect(h.fake.sendCount, 1, reason: 'the spend really ran');
    });

    test('confirm: the host DENIES after a send that returned → the landed '
        'outcome, never a live Review whose Confirm would pay again', () async {
      final h = wrapped(const _RunsThenDenies());
      await ctl(h.c).prepare(address: 'u1x', amountText: '1');
      await landsLanded(h, () => ctl(h.c).confirm(), queued: false);
      expect(h.fake.sendCount, 1, reason: 'the spend really ran');
    });

    test('queue: the host throws AFTER an enqueue that returned → SendQueued, '
        'never a retryable form fault', () async {
      final h = wrapped(const _RunsThenThrows());
      await landsLanded(
        h,
        () => ctl(h.c).queueOffline(address: 'u1x', amountText: '1'),
        queued: true,
      );
      expect(h.fake.queueCount, 1, reason: 'the enqueue really ran');
    });

    test('queue: the host DENIES after an enqueue that returned → SendQueued, '
        'never the form whose Queue would commit again', () async {
      final h = wrapped(const _RunsThenDenies());
      await landsLanded(
        h,
        () => ctl(h.c).queueOffline(address: 'u1x', amountText: '1'),
        queued: true,
      );
      expect(h.fake.queueCount, 1, reason: 'the enqueue really ran');
    });

    test('confirm: a StoreCorrupt from send ITSELF (raised after the tx is '
        'persisted) → SendOutcomeUnknown, never SendSignFailed', () async {
      // The core's `broadcast_persisted` re-reads the just-persisted bytes
      // and answers StoreCorrupt on a miss; the two-step's `mark_sent_multi`
      // can err after the create committed. Neither is "nothing was sent".
      final h = wrapped(_CountingPrompt());
      h.fake.sendThrows = err(const WalletErrorKind.storeCorrupt());
      await ctl(h.c).prepare(address: 'u1x', amountText: '1');
      await landsUnknown(h, () => ctl(h.c).confirm(), queued: false);
      expect(h.fake.sendCount, 1, reason: 'the spend really ran');
    });

    test('queue: a StoreCorrupt from queueSend ITSELF → SendOutcomeUnknown '
        '(queued), never a retryable form fault', () async {
      // The enqueue is one transaction, but this layer cannot tell a fault
      // before its insert from one at its commit — only the pre-insert kinds
      // keep the form.
      final h = wrapped(_CountingPrompt());
      h.fake.queueThrows = err(const WalletErrorKind.storeCorrupt());
      await landsUnknown(
        h,
        () => ctl(h.c).queueOffline(address: 'u1x', amountText: '1'),
        queued: true,
      );
      expect(h.fake.queueCount, 1, reason: 'the enqueue really ran');
    });

    test(
      'confirm: a host that SWALLOWS what send threw and returns normally '
      '→ SendOutcomeUnknown, never "nothing created" or a live Review',
      () async {
        final h = wrapped(const _RunsAndSwallows());
        h.fake.sendThrows = err(const WalletErrorKind.storeCorrupt());
        await ctl(h.c).prepare(address: 'u1x', amountText: '1');
        await landsUnknown(h, () => ctl(h.c).confirm(), queued: false);
        expect(h.fake.sendCount, 1, reason: 'the spend really ran');
      },
    );

    test('queue: a host that SWALLOWS what queueSend threw and returns '
        'normally → SendOutcomeUnknown (queued), never a live form', () async {
      final h = wrapped(const _RunsAndSwallows());
      h.fake.queueThrows = err(const WalletErrorKind.storeCorrupt());
      await landsUnknown(
        h,
        () => ctl(h.c).queueOffline(address: 'u1x', amountText: '1'),
        queued: true,
      );
      expect(h.fake.queueCount, 1, reason: 'the enqueue really ran');
    });
  });
}

/// A host authorizer that runs the spend, swallows whatever it threw, and
/// returns a value of its own (S7 U1 fold) — `landed` stays null although
/// the closure was entered.
class _RunsAndSwallows implements WalletSendAuthorizer {
  const _RunsAndSwallows();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    try {
      return await action();
    } catch (_) {
      const Object fabricated = <TxSubmitResult>[];
      return (fabricated is T ? fabricated : 'host-made-up-id') as T;
    }
  }
}

/// A host authorizer that runs the spend and THEN throws (its own
/// bookkeeping failing) — the money moved before the throw (S7 U1).
class _RunsThenThrows implements WalletSendAuthorizer {
  const _RunsThenThrows();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    await action();
    throw StateError('host bookkeeping failed after the spend');
  }
}

/// A host authorizer that runs the spend and THEN reports a denial (S7 U1).
class _RunsThenDenies implements WalletSendAuthorizer {
  const _RunsThenDenies();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    await action();
    throw const WalletSpendAuthorizationDenied();
  }
}

/// Holds the prompt until [approve], then runs the action.
class _HeldPrompt implements WalletSendAuthorizer {
  final Completer<void> _ok = Completer<void>();
  void approve() => _ok.complete();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    await _ok.future;
    return action();
  }
}

/// Counts prompts and approves: the S13 rows ask whether it was asked at all.
class _CountingPrompt implements WalletSendAuthorizer {
  int asked = 0;

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) {
    asked += 1;
    return action();
  }
}
