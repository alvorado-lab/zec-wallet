import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_state.dart';
import 'package:zec_wallet/zec_wallet.dart';

/// Pure mapper tests (D-2b-1) for `classifySwapFailure`. A wrong mapping would
/// show the user the wrong honest message (or a misleading "try again" on a
/// fail-closed money path), so every typed `SwapErrorKind` is pinned to its
/// reason at the boundary — no device, no harness.
void main() {
  SwapApiError err(SwapErrorKind kind) =>
      SwapApiError(code: 'RW-TEST', message: 'static', kind: kind);

  SwapFaultReason reasonOf(Object error) {
    final fault = classifySwapFailure(error);
    expect(
      fault,
      isA<SwapCategoricalFault>(),
      reason: 'every SDK kind maps to a categorical reason',
    );
    return (fault as SwapCategoricalFault).reason;
  }

  group('classifySwapFailure — every typed kind to its reason', () {
    test('slippage too high', () {
      expect(
        reasonOf(
          err(
            const SwapErrorKind.slippageToleranceTooHigh(
              requestedBps: 9000,
              maxBps: 500,
            ),
          ),
        ),
        SwapFaultReason.slippageTooHigh,
      );
    });

    test('quote out of bounds (protective rejection)', () {
      expect(
        reasonOf(
          err(const SwapErrorKind.quoteOutOfBounds(side: QuoteBoundSide.zec)),
        ),
        SwapFaultReason.quoteOutOfBounds,
      );
    });

    test('quote expired → re-quote', () {
      expect(
        reasonOf(err(const SwapErrorKind.quoteExpired())),
        SwapFaultReason.quoteExpired,
      );
    });

    test('destination invalid', () {
      expect(
        reasonOf(
          err(
            const SwapErrorKind.destinationInvalid(
              reason: DestinationInvalidReason.missing,
            ),
          ),
        ),
        SwapFaultReason.destinationInvalid,
      );
    });

    test('request invalid → re-quote', () {
      expect(
        reasonOf(err(const SwapErrorKind.requestInvalid(reason: 'shape'))),
        SwapFaultReason.requestInvalid,
      );
    });

    test('provider unavailable (retryable)', () {
      expect(
        reasonOf(err(const SwapErrorKind.providerUnavailable())),
        SwapFaultReason.providerUnavailable,
      );
    });

    test('provider protocol → provider misbehaved', () {
      expect(
        reasonOf(
          err(
            const SwapErrorKind.providerProtocol(
              reason: ProviderProtocolReason.refundAddressMismatch,
            ),
          ),
        ),
        SwapFaultReason.providerMisbehaved,
      );
    });

    test('swap disabled → swap off', () {
      expect(
        reasonOf(err(const SwapErrorKind.swapDisabled())),
        SwapFaultReason.swapOff,
      );
    });

    test('deposit send failed → our side, re-quote (no ZEC moved)', () {
      expect(
        reasonOf(err(const SwapErrorKind.depositSendFailed())),
        SwapFaultReason.depositFailed,
      );
    });

    test('refund address unavailable → our side', () {
      expect(
        reasonOf(err(const SwapErrorKind.refundAddressUnavailable())),
        SwapFaultReason.refundAddressUnavailable,
      );
    });

    test('destination address unavailable → its OWN wait-for-sync reason, '
        'never the generic could-not-quote (#382)', () {
      // Pre-#382 this kind fell through classify's `_` arm to couldNotQuote,
      // hiding the pre-first-sync cause the copy now names.
      expect(
        reasonOf(err(const SwapErrorKind.destinationAddressUnavailable())),
        SwapFaultReason.destinationAddressUnavailable,
      );
    });

    test('swap state unavailable → our side, fail-closed', () {
      expect(
        reasonOf(err(const SwapErrorKind.swapStateUnavailable())),
        SwapFaultReason.swapStateUnavailable,
      );
    });

    test('swap already in flight → its OWN reason, never depositFailed', () {
      // The remedy differs (W-swap-4-a-2): depositFailed says "re-quote";
      // this one must NOT (re-quoting is the double-deposit door the SDK
      // guard closes) — a collapse into depositFailed would put the
      // re-quote invitation back on screen.
      expect(
        reasonOf(err(const SwapErrorKind.swapAlreadyInFlight())),
        SwapFaultReason.swapInFlight,
      );
    });

    test(
      'an unknown forward-compat kind → generic, never a wrong specific',
      () {
        expect(
          reasonOf(err(const SwapErrorKind.unknown())),
          SwapFaultReason.couldNotQuote,
        );
      },
    );

    test(
      'watch-only → swap-off, never the re-quote framing (#397 D3, S234)',
      () {
        // Defensively-unreachable today (the kind is produced only at
        // enableNearSwap, and activation degrades to swap-off before any swap
        // screen exists) — but if it ever surfaces, "swap is off" is true and
        // permanent for this wallet; couldNotQuote's "try again" would be the
        // exact retry lie the typed pass-through exists to kill.
        expect(
          reasonOf(err(const SwapErrorKind.watchOnly())),
          SwapFaultReason.swapOff,
        );
      },
    );
  });

  test('a non-SDK error is the honest generic', () {
    expect(reasonOf(StateError('boom')), SwapFaultReason.couldNotQuote);
  });

  test('a host-side network timeout maps to the connection-failed reason', () {
    // kSwapNetworkTimeout wraps the quote/execute await; a broken/slow link to
    // 1Click surfaces as a TimeoutException (never a typed SwapApiError), so the
    // user gets the connectivity-focused message, not the bare generic.
    expect(
      reasonOf(TimeoutException('slow', const Duration(seconds: 60))),
      SwapFaultReason.connectionFailed,
    );
  });
}
