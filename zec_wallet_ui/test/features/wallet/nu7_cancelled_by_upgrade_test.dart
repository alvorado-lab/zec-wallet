// The NU7 wave's step 4 Dart row, written blind against the bridge names the
// step adds (`TxStatus.cancelledByUpgrade`, `CancelPhase`) and joined after the
// regen (renamed from `.dart.pending` by the adjudicator).
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/tx_detail_sheet.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// The NU7 wave's step 4, the activation check (`docs/plan/nu7-wave.md` §3 step
/// 4, S-M3 (a) to (c), M-3; ADR-0579 Decision 4, "what the wallet says"):
/// `a_cancelled_by_upgrade_send_never_renders_as_pending_or_retrying`.
///
/// A send the network upgrade cancelled reads as cancelled, never as on its
/// way. Until the bindings regenerate, a `TxStatus_Unknown` renders
/// `walletActivityPending` (`tx_detail_sheet.dart`, D-L6), so this drives the
/// `TxStatus` switch with the NEW arm, not the `DeliveryState` one:
/// - its status word is neither Pending nor Retrying;
/// - with `retryPending` (a live intent will send the payment again) its
///   explanation is its own (`walletTxExplainRetryingCancelled`, step 4's
///   fold), never `walletTxExplainRetrying`, whose "tries again on each sync
///   until it goes through" is untrue for a transaction that can never go
///   through;
/// - without a delivery reading it has its own sentence in every phase;
/// - before the released phase it promises no spend (never the Expired
///   explanation's "still yours to spend"), and the strike-through and the
///   `walletTxFundsKept` banner appear ONLY in the released phase, because
///   before it the funds are not back (`txIsCancelled` reads the phase);
/// - the released phase's sentence is the variant's own, and differs from the
///   not-final and held ones: only it may say the amount is back. Its wording
///   is the copy review's (`docs/REVIEW.md`), so no phrase of it is pinned here.
///
/// Names assumed (decided by the orchestrator on 2026-10-30):
/// `TxStatus.cancelledByUpgrade(stoppedAt: int, releasedAt: int?, phase:
/// CancelPhase)` (`releasedAt` is the stored expiry, null for a stored 0), its
/// class `TxStatus_CancelledByUpgrade`, and `CancelPhase.{notFinal,
/// finalNotReleased, released, unknown}`. The copy is asserted by behaviour,
/// never by a new key's name.
///
/// Graded by its frame (`docs/plan/stage-harness.md` §7).
///
/// Mutants, each red: the new arm falls to `Unknown` (the label reads
/// Pending); its explanation is routed to `walletTxExplainRetrying`;
/// `txIsCancelled` ignores the phase (the banner shows before release); the
/// released phase reuses the not-final sentence.
Widget _harness(TxSummary tx) => ProviderScope(
  child: MaterialApp(
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    theme: lightTheme,
    home: Scaffold(body: TxDetailSheet(tx: tx)),
  ),
);

WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(TxDetailSheet)));

TxSummary _cancelled(CancelPhase phase, {DeliveryState? delivery}) => TxSummary(
  txidHex: 'cc' * 32,
  status: TxStatus.cancelledByUpgrade(
    stoppedAt: 3543405,
    releasedAt: 3543435,
    phase: phase,
  ),
  netAmountZat: -150000,
  hasMemo: false,
  hasTransparentOutput: false,
  delivery: delivery,
  expiryHeight: 3543435,
);

void main() {
  testWidgets(
    'a_cancelled_by_upgrade_send_never_renders_as_pending_or_retrying',
    (tester) async {
      final earlier = <CancelPhase, String>{};
      for (final phase in [
        CancelPhase.notFinal,
        CancelPhase.finalNotReleased,
      ]) {
        // No delivery reading: the cancelled send is not on its way, and its
        // funds are not back yet.
        await tester.pumpWidget(const SizedBox());
        await tester.pumpWidget(_harness(_cancelled(phase)));
        await tester.pumpAndSettle();
        final l10n = _l10n(tester);
        final row = _cancelled(phase);
        expect(
          txStatusLabel(l10n, row.status),
          isNot(anyOf(l10n.walletActivityPending, l10n.walletActivityRetrying)),
          reason: '$phase: a cancelled send never reads as pending or retrying',
        );
        expect(
          txRowStatusLabel(l10n, row),
          isNot(anyOf(l10n.walletActivityPending, l10n.walletActivityRetrying)),
        );
        final sentence = txRowStatusExplanation(l10n, row);
        expect(
          sentence,
          isNot(
            anyOf(
              l10n.walletTxExplainPending,
              l10n.walletTxExplainRetrying,
              l10n.walletTxExplainUnknown,
            ),
          ),
          reason: '$phase: the cancelled reading has its own sentence',
        );
        earlier[phase] = sentence;
        expect(
          sentence,
          isNot(contains('still yours to spend')),
          reason: '$phase: no spend promise before the released phase',
        );
        expect(find.textContaining('still yours to spend'), findsNothing);
        expect(
          txIsCancelled(row),
          isFalse,
          reason: '$phase: txIsCancelled reads the phase',
        );
        expect(find.text(l10n.walletTxFundsKept), findsNothing);

        // A live intent will send the payment again: retrying, with the
        // cancelled-attempt explanation, never "tries again until it goes
        // through".
        final retrying = _cancelled(
          phase,
          delivery: DeliveryState.retryPending,
        );
        expect(
          txRowStatusExplanation(l10n, retrying),
          l10n.walletTxExplainRetryingCancelled,
        );
        expect(
          txRowStatusExplanation(l10n, retrying),
          isNot(l10n.walletTxExplainRetrying),
        );
      }

      // Released: both buried and released, the amount is back.
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(_harness(_cancelled(CancelPhase.released)));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      final released = _cancelled(CancelPhase.released);
      expect(txIsCancelled(released), isTrue);
      expect(find.text(l10n.walletTxFundsKept), findsOneWidget);
      final releasedSentence = txRowStatusExplanation(l10n, released);
      expect(
        releasedSentence,
        isNot(
          anyOf(
            l10n.walletTxExplainPending,
            l10n.walletTxExplainRetrying,
            l10n.walletTxExplainRetryingExpired,
            l10n.walletTxExplainExpired,
            l10n.walletTxExplainUnknown,
          ),
        ),
        reason: 'released: the cancelled reading has its own sentence',
      );
      expect(earlier.keys, [
        CancelPhase.notFinal,
        CancelPhase.finalNotReleased,
      ]);
      for (final entry in earlier.entries) {
        expect(
          releasedSentence,
          isNot(entry.value),
          reason:
              'released: the amount is back now, and only now, so its sentence '
              'differs from the ${entry.key} one',
        );
      }
      expect(
        txStatusLabel(l10n, released.status),
        isNot(anyOf(l10n.walletActivityPending, l10n.walletActivityRetrying)),
      );
    },
  );

  /// Step 4's fold (the code review's owed item): a retrying row whose status
  /// is `CancelledByUpgrade` reads its OWN sentence in every phase. The expired
  /// one, "This attempt expired before the network confirmed it", is false for
  /// it: nothing expired, the upgrade ended the attempt. Its sentence names the
  /// upgrade and keeps the "don't send it again yourself" clause (a manual
  /// re-send pays twice); a retrying Expired row keeps the expired sentence.
  ///
  /// Mutant: the cancelled arm routed back to `walletTxExplainRetryingExpired`.
  /// Red.
  testWidgets('a_retrying_upgrade_cancel_says_the_upgrade_ended_the_attempt', (
    tester,
  ) async {
    await tester.pumpWidget(
      _harness(
        _cancelled(CancelPhase.notFinal, delivery: DeliveryState.retryPending),
      ),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);
    for (final phase in CancelPhase.values) {
      final sentence = txRowStatusExplanation(
        l10n,
        _cancelled(phase, delivery: DeliveryState.retryPending),
      );
      expect(
        sentence,
        isNot(l10n.walletTxExplainRetryingExpired),
        reason: '$phase: the attempt did not expire, an upgrade ended it',
      );
      expect(sentence, l10n.walletTxExplainRetryingCancelled, reason: '$phase');
    }
    expect(l10n.walletTxExplainRetryingCancelled, contains('network upgrade'));
    expect(
      l10n.walletTxExplainRetryingCancelled,
      contains("don't send it again yourself"),
    );
    final expired = TxSummary(
      txidHex: 'dd' * 32,
      status: const TxStatus.expired(),
      netAmountZat: -150000,
      hasMemo: false,
      hasTransparentOutput: false,
      delivery: DeliveryState.retryPending,
      expiryHeight: 3543435,
    );
    expect(
      txRowStatusExplanation(l10n, expired),
      l10n.walletTxExplainRetryingExpired,
      reason: 'an expired retrying row keeps the expired sentence',
    );
  });
}
