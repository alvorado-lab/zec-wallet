import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';

/// Stage S8 `obligation`, §3.2 row 10, the Dart half — the TEST AUTHOR's row,
/// written blind against the contract (IT-2a) and against the bridge names the
/// author declared: `DeliveryState` (`persisted` / `retryPending` / `accepted`
/// / `confirmed`, the core's per-transaction delivery state after the fold's
/// regen) and a `delivery` argument on [summarizeSendOutcome] — the core's
/// reading per transaction, keyed by the txid every result carries, `null`
/// for a reading that could not be taken (the implementer's shape, ruled for
/// at the adjudication over the author's positional list: a list cannot say
/// "the read failed for `b`" and misaligns silently on a length mismatch).
///
/// "Saved for retry" is a PROMISE ("your wallet will send it on a later sync",
/// `walletSendSavedBody`), and the reducer used to make it from the result
/// list alone: any non-success, or an empty list, read as saved-for-retry. The
/// row pins that the promise is now made only when the core says it WILL
/// retry — `retryPending` — and never as a fallback for a shape the core did
/// not vouch for.
void main() {
  group(
    'summarizeSendOutcome — the saved-for-retry promise is the core\'s',
    () {
      test('saved_for_retry_is_shown_only_when_the_core_will_retry', () {
        // The core says it will retry the one transaction that got no verdict:
        // the promise is true, and the counts are the results'.
        final retrying = summarizeSendOutcome(
          const [TxSubmitResult.grpcFailure(txidHex: 'a')],
          isTwoStepTex: false,
          delivery: const {'a': DeliveryState.retryPending},
        );
        expect(retrying, isA<SendSavedForRetry>());
        final r = retrying as SendSavedForRetry;
        expect([r.broadcast, r.total], [0, 1]);

        // The same result list, but the core reports the bytes merely KEPT — it
        // is not retrying: the promise must not be made.
        final kept = summarizeSendOutcome(
          const [TxSubmitResult.grpcFailure(txidHex: 'a')],
          isTwoStepTex: false,
          delivery: const {'a': DeliveryState.persisted},
        );
        expect(
          kept,
          isNot(isA<SendSavedForRetry>()),
          reason: 'no retry-pending from the core, no "we will finish sending"',
        );

        // A partial pool-crossing send: one accepted, one the core will retry.
        final partial = summarizeSendOutcome(
          const [
            TxSubmitResult.success(txidHex: 'a'),
            TxSubmitResult.grpcFailure(txidHex: 'b'),
          ],
          isTwoStepTex: false,
          delivery: const {
            'a': DeliveryState.accepted,
            'b': DeliveryState.retryPending,
          },
        );
        expect(partial, isA<SendSavedForRetry>());
        expect(
          [(partial as SendSavedForRetry).broadcast, partial.total],
          [1, 2],
        );

        // The empty list was the old defensive fallback INTO saved-for-retry;
        // with nothing for the core to vouch for, the promise is not made.
        final empty = summarizeSendOutcome(
          const [],
          isTwoStepTex: false,
          delivery: const {},
        );
        expect(
          empty,
          isNot(isA<SendSavedForRetry>()),
          reason: 'an empty result list is not a retry the core is making',
        );

        // Every transaction accepted: succeeded, whatever the delivery reads —
        // the promise arm is only for what did not go out.
        final all = summarizeSendOutcome(
          const [
            TxSubmitResult.success(txidHex: 'a'),
            TxSubmitResult.success(txidHex: 'b'),
          ],
          isTwoStepTex: false,
          delivery: const {
            'a': DeliveryState.accepted,
            'b': DeliveryState.accepted,
          },
        );
        expect((all as SendSucceeded).txCount, 2);
      });

      test('a two-step TEX partial stays in-motion, never saved-for-retry, '
          'whatever the tail\'s delivery state', () {
        // The in-motion arm is decided by the SSOT shape signal and an accepted
        // leg; the core's retry state for the tail does not turn it into the
        // ordinary saved-for-retry copy (which would over-promise completion).
        final o = summarizeSendOutcome(
          const [
            TxSubmitResult.success(txidHex: 'tx0'),
            TxSubmitResult.grpcFailure(txidHex: 'tx1'),
          ],
          isTwoStepTex: true,
          delivery: const {
            'tx0': DeliveryState.accepted,
            'tx1': DeliveryState.retryPending,
          },
        );
        expect(o, isA<SendTexInMotion>());
      });
    },
  );
}
