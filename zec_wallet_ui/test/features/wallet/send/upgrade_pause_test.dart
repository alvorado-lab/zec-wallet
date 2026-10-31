import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_to_transparent_state.dart';
import 'package:zec_wallet_ui/features/wallet/send/form_fault_view.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/shield/shield_state.dart';
import 'package:zec_wallet_ui/features/wallet/upgrade_pause.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// ADR-0580, the pre-upgrade send pause (decided 2026-10-30): the SDK
/// refuses new sends in the last 20 blocks before a network upgrade's
/// activation with `WalletErrorKind.upgradeImminent { activationHeight,
/// blocksLeft }`, and every surface shows the decided sentence with the wait:
/// "The Zcash network upgrades in about N minutes. Sending is paused until then
/// so your payment can't get stuck." N is `blocksLeft` at 75 s a block, in
/// whole minutes rounded up, never 0.

WalletApiError _pause(int blocksLeft) => WalletApiError(
  code: 'RW-SEND-009',
  message: 'static',
  kind: WalletErrorKind.upgradeImminent(
    activationHeight: 3543405,
    blocksLeft: blocksLeft,
  ),
);

const _paused25 =
    'The Zcash network upgrades in about 25 minutes. Sending is paused until '
    "then so your payment can't get stuck.";

void main() {
  /// The wait in whole minutes, rounded UP from `blocksLeft` at 75 s a block:
  /// never sooner than the truth, never 0. Mutant: round down (`~/ 60` of the
  /// seconds alone); the one-block row reads 1, red.
  test('the_pause_names_the_wait_in_whole_minutes_rounded_up', () {
    expect(
      {
        for (final b in [1, 4, 8, 12, 19, 20]) b: upgradePauseMinutes(b),
      },
      {1: 2, 4: 5, 8: 10, 12: 15, 19: 24, 20: 25},
    );
    for (var blocks = 1; blocks <= 20; blocks++) {
      final minutes = upgradePauseMinutes(blocks);
      expect(
        minutes * 60 >= blocks * 75 && (minutes - 1) * 60 < blocks * 75,
        isTrue,
        reason: '$blocks blocks: $minutes is the smallest whole minute count',
      );
    }
    expect(upgradePauseMinutes(0), 1, reason: 'never "0 minutes"');
  });

  /// The window the copy falls back to is the core's K, read from its source,
  /// so the two cannot drift apart. Mutant: the Dart constant set to 21; red.
  test('the_window_constant_is_the_cores_k', () {
    final source = File(
      '../zec-wallet-core/src/constants.rs',
    ).readAsStringSync();
    final k = RegExp(
      r'pub const PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS: u32 = (\d+);',
    ).firstMatch(source);
    expect(k, isNotNull, reason: 'the core declares its K');
    expect(upgradePauseWindowBlocks, int.parse(k!.group(1)!));
  });

  /// Every classifier maps the pause to its wait, back to the form or the
  /// retryable sheet, never a failure. Mutants, each red: the propose arm, the
  /// send arm, the move arm, either shield arm, and the error-text helper's arm
  /// removed (each falls to its generic or failure state).
  test('every_classifier_maps_the_pause_to_its_wait_never_a_failure', () {
    final propose = classifyProposeFailure(_pause(20));
    expect(propose, isA<SendUpgradePausedFault>());
    expect((propose as SendUpgradePausedFault).blocksLeft, 20);

    final send = classifySendFailure(_pause(9));
    expect(send, isA<SendForm>(), reason: 'back to the form, nothing signed');
    final sendFault = (send as SendForm).fault;
    expect(sendFault, isA<SendUpgradePausedFault>());
    expect((sendFault as SendUpgradePausedFault).blocksLeft, 9);

    final move = classifyMoveSendFailure(_pause(4), 't1own');
    expect(move, isA<MoveAmountEntry>());
    final moveFault = (move as MoveAmountEntry).fault;
    expect(moveFault, isA<SendUpgradePausedFault>());
    expect((moveFault as SendUpgradePausedFault).blocksLeft, 4);

    for (final shield in [
      classifyShieldPrepareFailure(_pause(7)),
      classifyShieldSendFailure(_pause(7)),
    ]) {
      expect(shield, isA<ShieldUnavailable>());
      expect(
        (shield as ShieldUnavailable).reason,
        ShieldFaultReason.upgradePaused,
      );
      expect(shield.blocksLeft, 7);
    }

    final l10n = lookupWalletLocalizations(const Locale('en'));
    expect(upgradePausedTextFor(l10n, _pause(20)), _paused25);
    expect(
      upgradePausedTextFor(
        l10n,
        const WalletApiError(
          code: 'RW-SEND-004',
          message: 'static',
          kind: WalletErrorKind.proposalStale(),
        ),
      ),
      isNull,
      reason: 'any other error keeps its own copy',
    );
  });

  /// The send form renders the decided sentence, word for word, with the
  /// wait. Mutant: the view's pause arm removed (the fault falls to the
  /// generic "couldn't prepare"); red.
  testWidgets('the_send_form_shows_the_pause_sentence_with_the_wait', (
    tester,
  ) async {
    Widget host(int blocksLeft) => ProviderScope(
      child: MaterialApp(
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        theme: lightTheme,
        home: Scaffold(
          body: SendFormFaultView(
            fault: SendUpgradePausedFault(blocksLeft: blocksLeft),
          ),
        ),
      ),
    );

    await tester.pumpWidget(host(20));
    await tester.pumpAndSettle();
    expect(find.text(_paused25), findsOneWidget);

    await tester.pumpWidget(const SizedBox());
    await tester.pumpWidget(host(1));
    await tester.pumpAndSettle();
    expect(
      find.text(
        'The Zcash network upgrades in about 2 minutes. Sending is paused '
        "until then so your payment can't get stuck.",
      ),
      findsOneWidget,
      reason: 'one block, 75 s, rounds up to 2 minutes',
    );
  });
}
