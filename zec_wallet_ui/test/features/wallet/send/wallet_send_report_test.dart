import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_report.dart';

/// FR-26 — the two pure pieces under the host report: which ids a send set
/// yields, and how a terminal state becomes a report. Boundary tests, no widget
/// tree and no device.
void main() {
  group('sendTxids', () {
    test('collects the id from EVERY arm that has one, not only the accepted '
        'ones — an unbroadcast tx exists and the host must be able to cite '
        'it', () {
      expect(
        sendTxids(const [
          TxSubmitResult.success(txidHex: 'aa'),
          TxSubmitResult.grpcFailure(txidHex: 'bb'),
          TxSubmitResult.submitFailure(txidHex: 'cc', code: -26),
          TxSubmitResult.notAttempted(txidHex: 'dd'),
        ]),
        ['aa', 'bb', 'cc', 'dd'],
      );
    });

    test(
      'the forward-compat unknown arm carries no id and contributes none',
      () {
        expect(
          sendTxids(const [
            TxSubmitResult.success(txidHex: 'aa'),
            TxSubmitResult.unknown(),
          ]),
          ['aa'],
        );
      },
    );

    test('an empty result set yields no ids', () {
      expect(sendTxids(const []), isEmpty);
    });
  });

  group('walletSendReportFor', () {
    test('a fully broadcast send is a transaction that exists, all out', () {
      final report =
          walletSendReportFor(
                const SendFlowSent(
                  1,
                  outcome: SendSucceeded(2),
                  txids: ['aa', 'bb'],
                  isTwoStepTex: false,
                ),
                correlationId: 'row-1',
              )
              as WalletSendTransactionCreated;
      expect(report.txids, ['aa', 'bb']);
      expect(report.broadcastCount, 2);
      expect(report.motion, WalletSendMotion.notInMotion);
      expect(report.correlationId, 'row-1');
    });

    test('saved-for-retry is STILL a transaction that exists — the count says '
        'how much of it reached the network, and the wallet re-sends the '
        'rest', () {
      final report =
          walletSendReportFor(
                const SendFlowSent(
                  1,
                  outcome: SendSavedForRetry(broadcast: 1, total: 2),
                  txids: ['aa', 'bb'],
                  isTwoStepTex: false,
                ),
              )
              as WalletSendTransactionCreated;
      expect(report.txids, ['aa', 'bb']);
      expect(report.broadcastCount, 1);
      expect(report.motion, WalletSendMotion.notInMotion);
    });

    test('a two-step TEX partial flags funds IN MOTION — the host copy must '
        'not promise auto-completion', () {
      final report =
          walletSendReportFor(
                const SendFlowSent(
                  1,
                  outcome: SendTexInMotion(broadcast: 1, total: 2),
                  txids: ['aa', 'bb'],
                  isTwoStepTex: true,
                ),
              )
              as WalletSendTransactionCreated;
      expect(report.motion, WalletSendMotion.inMotion);
      expect(report.broadcastCount, 1);
    });

    test('a re-entry after submission is its own answer, never a second '
        'payment', () {
      expect(
        walletSendReportFor(
          const SendFlowSent(
            1,
            outcome: SendAlreadySubmitted(),
            txids: [],
            isTwoStepTex: false,
          ),
        ),
        isA<WalletSendAlreadySubmitted>(),
      );
    });

    test('a refused sign created nothing', () {
      expect(
        walletSendReportFor(
          const SendFlowSent(
            1,
            outcome: SendSignFailed(),
            txids: [],
            isTwoStepTex: false,
          ),
        ),
        isA<WalletSendNoTransaction>(),
      );
    });

    test('THE HONESTY GUARD: an outcome that claims a send but carries NO id '
        'is unclassified, never a transaction the host can never '
        'reconcile', () {
      // The defensive empty-result arm of the reducer reads as saved-for-retry.
      // There is nothing to cite, so it must not read as a payment.
      expect(
        walletSendReportFor(
          const SendFlowSent(
            1,
            outcome: SendSavedForRetry(broadcast: 0, total: 0),
            txids: [],
            isTwoStepTex: false,
          ),
        ),
        isA<WalletSendUnclassified>(),
      );
    });

    test('a tagged report names the amount as signed, an untagged one names '
        'none', () {
      // FR-46: the tag is what makes the host's join honest — the host already
      // knows the recipient it locked. An untagged push gets nothing new.
      const flow = SendFlowSent(
        1,
        outcome: SendTexInMotion(broadcast: 1, total: 2),
        txids: ['aa', 'bb'],
        isTwoStepTex: true,
        singleRecipientZat: 20000,
      );
      final tagged =
          walletSendReportFor(flow, correlationId: 'row-1')
              as WalletSendTransactionCreated;
      // Set whatever the motion — a statement about what was signed, never
      // about arrival.
      expect(tagged.motion, WalletSendMotion.inMotion);
      expect(tagged.recipientAmountZat, 20000);

      final untagged =
          walletSendReportFor(flow) as WalletSendTransactionCreated;
      expect(untagged.recipientAmountZat, isNull);

      // Two recipients: the proposal carried no figure, so neither does the
      // tagged report — never a partial sum.
      final twoRecipients =
          walletSendReportFor(
                const SendFlowSent(
                  1,
                  outcome: SendSucceeded(1),
                  txids: ['aa'],
                  isTwoStepTex: false,
                ),
                correlationId: 'row-1',
              )
              as WalletSendTransactionCreated;
      expect(twoRecipients.recipientAmountZat, isNull);
    });

    test('the correlation token rides onto every variant, untouched', () {
      const id = 'opaque/host token — never parsed';
      expect(
        walletSendReportFor(
          const SendFlowSent(
            1,
            outcome: SendAlreadySubmitted(),
            txids: [],
            isTwoStepTex: false,
          ),
          correlationId: id,
        ).correlationId,
        id,
      );
      expect(
        walletSendReportFor(
          const SendFlowSent(
            1,
            outcome: SendSignFailed(),
            txids: [],
            isTwoStepTex: false,
          ),
          correlationId: id,
        ).correlationId,
        id,
      );
      expect(
        walletSendReportFor(
          const SendFlowSent(
            1,
            outcome: SendSucceeded(1),
            txids: ['aa'],
            isTwoStepTex: false,
          ),
          correlationId: id,
        ).correlationId,
        id,
      );
    });
  });
}
