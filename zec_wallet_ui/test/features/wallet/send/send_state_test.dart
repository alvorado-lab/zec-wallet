import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Pure-function tests for the send-path mappers + outcome summarizer
/// (inc-2d-ui). These are total functions over a money-bearing error surface, so
/// EVERY `WalletErrorKind` arm the send path can throw is pinned to its honest
/// user-facing fault here (the controller tests exercise the common ones through
/// the state machine; this is the exhaustive boundary pass).
void main() {
  WalletApiError err(WalletErrorKind kind) =>
      WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

  SendFaultReason reasonOf(SendFormFault f) =>
      (f as SendCategoricalFault).reason;

  group('classifyProposeFailure — every reachable arm', () {
    test('insufficientFunds carries the figures', () {
      final f = classifyProposeFailure(
        err(
          const WalletErrorKind.insufficientFunds(
            availableZat: 1,
            requiredZat: 2,
            pendingIncomingZat: 3,
          ),
        ),
      );
      expect(f, isA<SendInsufficientFunds>());
      final i = f as SendInsufficientFunds;
      expect([i.availableZat, i.requiredZat, i.pendingIncomingZat], [1, 2, 3]);
    });

    final cases = <WalletErrorKind, SendFaultReason>{
      const WalletErrorKind.addressInvalid(): SendFaultReason.addressInvalid,
      const WalletErrorKind.memoRequiresShieldedRecipient():
          SendFaultReason.memoToTransparent,
      // BigInt.zero is not a const, so this key is built at runtime (the map
      // literal is non-const anyway).
      WalletErrorKind.memoTooLong(len: BigInt.zero, max: BigInt.zero):
          SendFaultReason.memoTooLong,
      const WalletErrorKind.reservedMemoNotSendable():
          SendFaultReason.memoNotSendable,
      const WalletErrorKind.memoInvalid(): SendFaultReason.memoNotSendable,
      // DISTINCT from memoNotSendable, deliberately: a both-memos refusal is a
      // CALLER bug (neither memo is individually invalid), and folding it into
      // the memo-is-corrupt copy told the user to fix something they did not do
      // and could not fix. Sharing a reason here is the regression this row
      // catches (post-build review; closed).
      const WalletErrorKind.memoConflict(): SendFaultReason.memoConflict,
      const WalletErrorKind.amountOutOfRange():
          SendFaultReason.amountOutOfRange,
      const WalletErrorKind.networkMismatch(): SendFaultReason.networkMismatch,
      // #397 §3.7 D3: a watch-only wallet refuses every spend at the SDK entry
      // (RW-VIEW-001) — mapped to the honest permanent fact, never the generic
      // couldNotPrepare (defense-in-depth; the chrome + screen gate normally make
      // it unreachable).
      const WalletErrorKind.watchOnly(): SendFaultReason.watchOnly,
      const WalletErrorKind.paymentUriInvalid(): SendFaultReason.uriInvalid,
      const WalletErrorKind.proposalStale(): SendFaultReason.notSyncedYet,
      const WalletErrorKind.walletBusy(phase: LifecyclePhase.closing):
          SendFaultReason.walletBusy,
      const WalletErrorKind.invalidState(phase: LifecyclePhase.wiped):
          SendFaultReason.walletBusy,
      // W-swap-4-a-4: a store write that lost its race to a sync commit past
      // the SDK's bounded retry is the SAME honest "try again in a moment" —
      // never the corruption journey.
      const WalletErrorKind.storeBusy(): SendFaultReason.walletBusy,
      // #373: out of disk persisting the compose → the honest "free up space",
      // never the generic "couldn't prepare" that retries into a re-fail.
      const WalletErrorKind.diskFull(): SendFaultReason.storageFull,
      // Explicitly mapped (not silently to the wildcard) — honest generic, never
      // a wrong "larger than supply".
      const WalletErrorKind.zeroValuedTransparentOutput():
          SendFaultReason.couldNotPrepare,
      const WalletErrorKind.proposeFailed(): SendFaultReason.couldNotPrepare,
      // INC-018 (b), phase-2 P2-2: the retryable class the SDK types apart
      // reaches its OWN reason ("try again in a moment"), never the generic
      // "check the details" that blamed correct input on the device proof.
      // Mutant: the `proposeTransient` arm deleted from classifyProposeFailure
      // → the wildcard maps it to couldNotPrepare and this row reds.
      const WalletErrorKind.proposeTransient():
          SendFaultReason.couldNotPrepareTransient,
      // A forward-compat / unmapped kind falls to the honest generic.
      const WalletErrorKind.unknown(): SendFaultReason.couldNotPrepare,
    };
    cases.forEach((kind, reason) {
      test('$kind → $reason', () {
        expect(reasonOf(classifyProposeFailure(err(kind))), reason);
      });
    });

    test('a non-FRB error is the honest generic, never a crash', () {
      expect(
        reasonOf(classifyProposeFailure(StateError('x'))),
        SendFaultReason.couldNotPrepare,
      );
    });
  });

  group('classifyQueueFailure', () {
    test('queuedSendsFull is the one queue-specific arm', () {
      expect(
        reasonOf(
          classifyQueueFailure(err(const WalletErrorKind.queuedSendsFull())),
        ),
        SendFaultReason.queueFull,
      );
    });
    test('every other kind delegates to the propose mapping', () {
      expect(
        reasonOf(
          classifyQueueFailure(err(const WalletErrorKind.addressInvalid())),
        ),
        SendFaultReason.addressInvalid,
      );
    });
  });

  group('classifySendFailure — routes to result OR back to form', () {
    test('alreadyUsed → already-submitted (no double-send)', () {
      final s = classifySendFailure(
        err(const WalletErrorKind.proposalAlreadyUsed()),
      );
      expect((s as SendSent).outcome, isA<SendAlreadySubmitted>());
    });
    test('stale → back to the form with the amounts-expired reason (NOT '
        'not-synced — the wallet is synced; the numbers aged out)', () {
      final s = classifySendFailure(err(const WalletErrorKind.proposalStale()));
      expect(reasonOf((s as SendForm).fault!), SendFaultReason.amountsExpired);
    });
    test('busy/invalid → back to the form (retry)', () {
      final s = classifySendFailure(
        err(const WalletErrorKind.invalidState(phase: LifecyclePhase.closing)),
      );
      expect(reasonOf((s as SendForm).fault!), SendFaultReason.walletBusy);
    });
    test('storeBusy → back to the form (orange-transient), NEVER the red '
        'sign-failed dead-end — nothing was written (S183 fold)', () {
      final s = classifySendFailure(err(const WalletErrorKind.storeBusy()));
      expect(reasonOf((s as SendForm).fault!), SendFaultReason.walletBusy);
    });
    test('queuedSendsFull → back to the form (orange-transient outbox-full), '
        'NEVER the red sign-failed dead-end (2e-2b)', () {
      final s = classifySendFailure(
        err(const WalletErrorKind.queuedSendsFull()),
      );
      expect(s, isA<SendForm>());
      expect(reasonOf((s as SendForm).fault!), SendFaultReason.queueFull);
    });
    test('texSendLimitReached → back to the form (one-time-address ceiling), '
        'NEVER the red sign-failed dead-end (2e-2b)', () {
      final s = classifySendFailure(
        err(const WalletErrorKind.texSendLimitReached()),
      );
      expect(s, isA<SendForm>());
      expect(
        reasonOf((s as SendForm).fault!),
        SendFaultReason.oneTimeAddressLimit,
      );
    });
    test(
      'diskFull → back to the form with the free-up-space reason (#373), '
      'NEVER the red sign-failed dead-end — nothing broadcast, funds safe',
      () {
        final s = classifySendFailure(err(const WalletErrorKind.diskFull()));
        expect(s, isA<SendForm>());
        expect(reasonOf((s as SendForm).fault!), SendFaultReason.storageFull);
      },
    );
    test('watchOnly → back to the form with the honest can\'t-send fact '
        '(#397 D3), NEVER the red sign-failed dead-end', () {
      final s = classifySendFailure(err(const WalletErrorKind.watchOnly()));
      expect(s, isA<SendForm>());
      expect(reasonOf((s as SendForm).fault!), SendFaultReason.watchOnly);
    });
    test('signFailed → couldn\'t-complete result', () {
      final s = classifySendFailure(err(const WalletErrorKind.signFailed()));
      expect((s as SendSent).outcome, isA<SendSignFailed>());
    });
    test('a non-FRB error → couldn\'t-complete result', () {
      final s = classifySendFailure(StateError('x'));
      expect((s as SendSent).outcome, isA<SendSignFailed>());
    });
  });

  group('classifyRecipient — live recipient classification (slice 2)', () {
    test('a blank / whitespace field is RecipientEmpty (no bridge call)', () {
      final fake = FakeWalletSession();
      expect(classifyRecipient(fake, ''), isA<RecipientEmpty>());
      expect(classifyRecipient(fake, '   '), isA<RecipientEmpty>());
      expect(
        fake.validateRecipientCount,
        0,
        reason: 'an empty field never crosses the bridge',
      );
    });

    test(
      'a memo-capable address is RecipientShielded (private, memo-able)',
      () {
        final fake = FakeWalletSession()
          ..validateRecipientResult = const ValidatedAddress(memoCapable: true);
        final s = classifyRecipient(fake, 'u1shielded');
        expect(s, isA<RecipientShielded>());
        expect(s.isSendable, isTrue);
        expect(s.allowsMemo, isTrue);
        expect(fake.lastValidatedRecipient, 'u1shielded');
      },
    );

    test(
      'a non-memo-capable address is RecipientTransparent (public, no memo)',
      () {
        final fake = FakeWalletSession()
          ..validateRecipientResult = const ValidatedAddress(
            memoCapable: false,
          );
        final s = classifyRecipient(fake, 't1transparent');
        expect(s, isA<RecipientTransparent>());
        expect(
          s.isSendable,
          isTrue,
          reason: 'transparent is allowed (warned at Review, never blocked)',
        );
        expect(s.allowsMemo, isFalse, reason: 'memo gated off for transparent');
      },
    );

    test('the address is trimmed before validation', () {
      final fake = FakeWalletSession()
        ..validateRecipientResult = const ValidatedAddress(memoCapable: true);
      classifyRecipient(fake, '  u1padded  ');
      expect(fake.lastValidatedRecipient, 'u1padded');
    });

    test('a typed addressInvalid → RecipientInvalid (not sendable)', () {
      final fake = FakeWalletSession()
        ..validateRecipientThrows = err(const WalletErrorKind.addressInvalid());
      final s = classifyRecipient(fake, 'garbage');
      expect(s, isA<RecipientInvalid>());
      expect(s.isSendable, isFalse);
      expect(
        s.allowsMemo,
        isTrue,
        reason: 'memo stays neutral while the recipient is invalid',
      );
    });

    test(
      'a typed networkMismatch → RecipientWrongNetwork (distinct, not sendable)',
      () {
        final fake = FakeWalletSession()
          ..validateRecipientThrows = err(
            const WalletErrorKind.networkMismatch(),
          );
        final s = classifyRecipient(fake, 'utestnetaddr');
        expect(s, isA<RecipientWrongNetwork>());
        expect(s.isSendable, isFalse);
      },
    );

    test(
      'any other typed kind degrades to RecipientInvalid (honest, never a code)',
      () {
        final fake = FakeWalletSession()
          ..validateRecipientThrows = err(const WalletErrorKind.unknown());
        expect(classifyRecipient(fake, 'x'), isA<RecipientInvalid>());
      },
    );

    test('a non-FRB error degrades to RecipientInvalid, never a crash', () {
      final fake = FakeWalletSession()
        ..validateRecipientThrows = StateError('boom');
      expect(classifyRecipient(fake, 'x'), isA<RecipientInvalid>());
    });
  });

  group('summarizeSendOutcome — broadcast failure is never lost money', () {
    test('all-success → succeeded', () {
      final o = summarizeSendOutcome(
        const [
          TxSubmitResult.success(txidHex: 'a'),
          TxSubmitResult.success(txidHex: 'b'),
        ],
        isTwoStepTex: false,
        delivery: const {},
      );
      expect((o as SendSucceeded).txCount, 2);
    });
    test(
      'partial, the rest retry-pending → saved-for-retry with the counts',
      () {
        final o = summarizeSendOutcome(
          const [
            TxSubmitResult.success(txidHex: 'a'),
            TxSubmitResult.grpcFailure(txidHex: 'b'),
            TxSubmitResult.notAttempted(txidHex: 'c'),
          ],
          isTwoStepTex: false,
          delivery: const {
            'b': DeliveryState.retryPending,
            'c': DeliveryState.retryPending,
          },
        );
        final r = o as SendSavedForRetry;
        expect([r.broadcast, r.total], [1, 3]);
      },
    );
    test('total failure, retry-pending → saved-for-retry, broadcast 0', () {
      final o = summarizeSendOutcome(
        const [TxSubmitResult.grpcFailure(txidHex: 'a')],
        isTwoStepTex: false,
        delivery: const {'a': DeliveryState.retryPending},
      );
      final r = o as SendSavedForRetry;
      expect([r.broadcast, r.total], [0, 1]);
    });
    test(
      'empty (defensive) → kept 0/0, never a retry promise over nothing',
      () {
        final r =
            summarizeSendOutcome(
                  const [],
                  isTwoStepTex: false,
                  delivery: const {},
                )
                as SendKept;
        expect([r.broadcast, r.total], [0, 0]);
      },
    );
  });

  group('summarizeSendOutcome — a two-step TEX partial is IN MOTION, not '
      'ordinary saved-for-retry', () {
    test('first leg out, forwarding tail not → in-motion (1/2), NOT '
        'saved-for-retry', () {
      // tx0 (the unshield to the wallet's one-time address) broadcast; tx1 (the
      // forward to the recipient) did not. The legs are SEQUENCED — funds are in
      // motion on a wallet-controlled ephemeral, so the honest copy must be the
      // distinct in-motion arm (no over-promise of auto-completion), never the
      // ordinary "the rest will complete on the next sync".
      final o = summarizeSendOutcome(
        const [
          TxSubmitResult.success(txidHex: 'tx0'),
          TxSubmitResult.grpcFailure(txidHex: 'tx1'),
        ],
        isTwoStepTex: true,
        delivery: const {},
      );
      expect(o, isA<SendTexInMotion>());
      final r = o as SendTexInMotion;
      expect([r.broadcast, r.total], [1, 2]);
    });
    test('both legs out → ordinary succeeded (no in-motion ambiguity)', () {
      final o = summarizeSendOutcome(
        const [
          TxSubmitResult.success(txidHex: 'tx0'),
          TxSubmitResult.success(txidHex: 'tx1'),
        ],
        isTwoStepTex: true,
        delivery: const {},
      );
      expect((o as SendSucceeded).txCount, 2);
    });
    test('nothing went out → saved-for-retry, NOT in-motion (no funds in '
        'motion on a zero-broadcast two-step)', () {
      // tx0 never reached the network ⇒ broadcast latched ⇒ tx1 not attempted.
      // No ephemeral funded, nothing in motion: the accurate "nothing went out"
      // saved-for-retry copy, not the in-motion arm (which would falsely claim
      // money already left).
      // S8: "saved for retry" is said only when the core reports it will retry
      // — both legs read retry-pending here, as an owed two-step does.
      final o = summarizeSendOutcome(
        const [
          TxSubmitResult.grpcFailure(txidHex: 'tx0'),
          TxSubmitResult.notAttempted(txidHex: 'tx1'),
        ],
        isTwoStepTex: true,
        delivery: const {
          'tx0': DeliveryState.retryPending,
          'tx1': DeliveryState.retryPending,
        },
      );
      expect(o, isA<SendSavedForRetry>());
      expect([(o as SendSavedForRetry).broadcast, o.total], [0, 2]);
    });
    test('a partial that is NOT a two-step TEX stays ordinary saved-for-retry '
        '(the SSOT flag is the only discriminator)', () {
      final o = summarizeSendOutcome(
        const [
          TxSubmitResult.success(txidHex: 'a'),
          TxSubmitResult.grpcFailure(txidHex: 'b'),
        ],
        isTwoStepTex: false,
        delivery: const {'b': DeliveryState.retryPending},
      );
      expect(o, isA<SendSavedForRetry>());
    });
    test('tx0 out, tx1 endpoint-REJECTED (SubmitFailure, not just a transport '
        'miss) → still in-motion (the summarizer counts only Success)', () {
      // tx0 landed; tx1 was actively rejected from the mempool (a code, not a
      // dropped link). "out" is Success only — tx1 is not out — so the funds are
      // still in motion on the ephemeral and the routing is the in-motion arm,
      // identical to the transport-miss case.
      final o = summarizeSendOutcome(
        const [
          TxSubmitResult.success(txidHex: 'tx0'),
          TxSubmitResult.submitFailure(txidHex: 'tx1', code: 16),
        ],
        isTwoStepTex: true,
        delivery: const {},
      );
      expect(o, isA<SendTexInMotion>());
      expect([(o as SendTexInMotion).broadcast, o.total], [1, 2]);
    });
    test('tx0 already-known-REJECTED, tx1 out (the core-#307 race shape) → '
        'in-motion, NOT saved-for-retry', () {
      // The wallet's own background re-broadcast raced the send and landed tx0
      // first: our submit reads tx0 Rejected (already known), then tx1 is
      // accepted (an honest endpoint can only accept tx1 if it knows tx0). Money
      // HAS left the shielded pool — the in-motion copy is the honest arm; the
      // ordinary "saved for retry" would falsely claim nothing is in motion.
      final o = summarizeSendOutcome(
        const [
          TxSubmitResult.submitFailure(txidHex: 'tx0', code: 18),
          TxSubmitResult.success(txidHex: 'tx1'),
        ],
        isTwoStepTex: true,
        delivery: const {},
      );
      expect(o, isA<SendTexInMotion>());
      expect([(o as SendTexInMotion).broadcast, o.total], [1, 2]);
    });
    test('both legs REJECTED → saved-for-retry (the recorded residual: reject '
        'codes are uninterpreted, the #309 cue owns that window)', () {
      // Both legs reading submitFailure CAN be the already-known race shape
      // (the background pass landed both first — money in motion), but the pure
      // reducer deliberately does not parse backend-specific reject codes, so
      // it stays the conservative saved-for-retry. This pins that the routing
      // is DELIBERATE — the durable wallet-screen in-flight cue carries the
      // honest "don't re-send" through the window either way.
      final o = summarizeSendOutcome(
        const [
          TxSubmitResult.submitFailure(txidHex: 'tx0', code: 18),
          TxSubmitResult.submitFailure(txidHex: 'tx1', code: 18),
        ],
        isTwoStepTex: true,
        delivery: const {
          'tx0': DeliveryState.retryPending,
          'tx1': DeliveryState.retryPending,
        },
      );
      expect(o, isA<SendSavedForRetry>());
      expect([(o as SendSavedForRetry).broadcast, o.total], [0, 2]);
    });
  });
}
