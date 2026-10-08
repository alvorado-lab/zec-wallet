import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_assets.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_controller.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_state.dart';
import 'package:zec_wallet_ui/features/wallet/send/zec_amount.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Swap state-machine tests (D-2b-1 / §3.3b). The whole flow runs on the host VM
/// against `FakeWalletSession` behind the `WalletSession` port — no native
/// library, no device. The money-shaping assertions (the exact [QuoteRequest] each
/// direction lowers to) and the re-quote-on-execute-failure invariant are the
/// load-bearing ones.
void main() {
  ({ProviderContainer container, FakeWalletSession fake}) harness({
    bool nullSession = false,
    int? sendCeilingZat,
  }) {
    final fake = FakeWalletSession();
    final container = ProviderContainer(
      overrides: [
        walletSessionProvider.overrideWithValue(nullSession ? null : fake),
        if (sendCeilingZat != null)
          walletSendCeilingZatProvider.overrideWithValue(sendCeilingZat),
      ],
    );
    addTearDown(container.dispose);
    return (container: container, fake: fake);
  }

  SwapController ctl(ProviderContainer c) =>
      c.read(swapControllerProvider.notifier);
  SwapFlowState st(ProviderContainer c) => c.read(swapControllerProvider);

  SwapApiError swapErr(SwapErrorKind kind) =>
      SwapApiError(code: 'RW-TEST', message: 'static', kind: kind);

  // A picked OutOfZec TARGET asset (the picker maps SwapToken → SwapAsset; there
  // is no static default menu any more — the form requires a pick).
  const usdc = SwapAsset(
    chain: 'eth',
    symbol: 'usdc',
    label: 'USDC on Ethereum',
  );
  // A picked IntoZec SOURCE asset (the picker maps SwapToken → SwapAsset).
  const btc = SwapAsset(chain: 'btc', symbol: 'btc', label: 'BTC on BTC');

  Future<void> quoteOut(
    ProviderContainer c, {
    SwapAsset? asset,
    String amountText = '0.5',
    String destination = '0xd',
    int slippageBps = referenceSwapSlippageBps,
  }) => ctl(c).quote(
    OutOfZecInput(
      asset: asset ?? usdc,
      amountText: amountText,
      destination: destination,
      slippageBps: slippageBps,
    ),
  );

  Future<void> quoteIn(
    ProviderContainer c, {
    SwapAsset? token,
    String amountText = '100',
    String refundAddress = 'bc1qrefund',
    int slippageBps = 200,
  }) => ctl(c).quote(
    IntoZecInput(
      token: token ?? btc,
      amountText: amountText,
      refundAddress: refundAddress,
      slippageBps: slippageBps,
    ),
  );

  /// An IntoZec-shaped quote (ends shielded, no de-shield, refund echoed, the
  /// ZEC side is the min-out floor).
  SwapQuote intoZecQuote() => swapQuoteFixture(
    id: 'swap-iz',
    amountIn: '100',
    minAmountOut: '4.12248474',
    zecSideZat: 412248474,
    refundTo: 'bc1qrefund',
    deshields: false,
    endsShielded: true,
    providerSees: const [
      DisclosureItem.amounts,
      DisclosureItem.crossAssetLink,
      DisclosureItem.sourceAddress,
      DisclosureItem.ipUnlessTor,
    ],
  );

  group('OutOfZec quote → review', () {
    test('lowers the form to an exact OutOfZec quote request', () async {
      final h = harness();
      h.fake.swapQuoteResult = swapQuoteFixture(id: 'swap-9');
      await quoteOut(h.container, amountText: '0.5', destination: '  0xdest  ');

      final s = st(h.container);
      expect(s, isA<SwapReview>());
      expect((s as SwapReview).counter, usdc);
      expect(s.direction, SwapFlowDirection.outOfZec);
      expect(s.quote.id, 'swap-9');

      // The money-shaping: OutOfZec, exact ZEC IN, integer-exact zat, the
      // requested slippage, the trimmed destination, and NO host refund address
      // (the SDK derives a fresh single-use one).
      final req = h.fake.lastSwapQuoteRequest!;
      expect(req.direction, isA<SwapDirection_OutOfZec>());
      final dir = req.direction as SwapDirection_OutOfZec;
      expect(dir.to.chain, usdc.chain);
      expect(dir.to.symbol, usdc.symbol);
      expect(req.exact, isA<ExactSide_In>());
      final amount = (req.exact as ExactSide_In).amount;
      expect(amount, isA<SwapAmount_Zec>());
      expect((amount as SwapAmount_Zec).zat, 50000000); // 0.5 ZEC
      expect(req.slippageToleranceBps, referenceSwapSlippageBps);
      expect(req.destination, '0xdest');
      expect(req.refundAddress, isNull);
      expect(h.fake.swapQuoteCount, 1);
    });

    test('an over-ceiling OutOfZec deposit is refused at review (FR-23)', () async {
      // The alpha send ceiling BOUNDS the swap deposit: an OutOfZec quote whose ZEC
      // side exceeds the cap lands on the form with the honest over-ceiling fault
      // (carrying the limit), BEFORE any authorize-spend bracket — never a
      // reviewable quote the user can't execute under the cap.
      final h = harness(sendCeilingZat: 100000000); // 1 ZEC cap
      h.fake.swapQuoteResult = swapQuoteFixture(
        id: 'swap-cap',
        zecSideZat: 500000000, // 5 ZEC deposit — over the cap
      );
      await quoteOut(h.container);

      final s = st(h.container);
      expect(s, isA<SwapFormState>());
      final fault = (s as SwapFormState).fault;
      expect(fault, isA<SwapOverCeiling>());
      expect((fault as SwapOverCeiling).ceilingZat, 100000000);
    });

    test(
      'an at-cap OutOfZec deposit reviews normally (the bound is strict >)',
      () async {
        // Exactly at the ceiling is allowed (`>`, not `>=`) — parity with the send form.
        final h = harness(sendCeilingZat: 100000000);
        h.fake.swapQuoteResult = swapQuoteFixture(
          id: 'swap-atcap',
          zecSideZat: 100000000, // exactly 1 ZEC == the cap
        );
        await quoteOut(h.container);
        expect(st(h.container), isA<SwapReview>());
      },
    );

    test('IntoZec is unaffected by the send ceiling (money-OUT only)', () async {
      // IntoZec moves no wallet funds (the user sends the deposit externally), so
      // the ceiling never applies even when the ZEC min-out exceeds the cap.
      final h = harness(sendCeilingZat: 100000000);
      h.fake.swapQuoteResult = intoZecQuote(); // zecSideZat 412248474 > cap
      await quoteIn(h.container);
      expect(st(h.container), isA<SwapReview>());
    });

    test('a bad amount fails host-side — no bridge call at all', () async {
      final h = harness();
      await quoteOut(h.container, amountText: 'not-a-number');
      final s = st(h.container) as SwapFormState;
      expect(s.fault, isA<SwapAmountFault>());
      expect((s.fault as SwapAmountFault).fault, ZecAmountFault.notANumber);
      expect(h.fake.swapQuoteCount, 0);
    });

    test('a zero amount is rejected before any bridge call', () async {
      final h = harness();
      await quoteOut(h.container, amountText: '0');
      final s = st(h.container) as SwapFormState;
      expect((s.fault as SwapAmountFault).fault, ZecAmountFault.notPositive);
      expect(h.fake.swapQuoteCount, 0);
    });

    test(
      'an empty destination is rejected host-side (OutOfZec needs it)',
      () async {
        final h = harness();
        await quoteOut(h.container, destination: '   ');
        final s = st(h.container) as SwapFormState;
        expect(s.fault, isA<SwapCategoricalFault>());
        expect(
          (s.fault as SwapCategoricalFault).reason,
          SwapFaultReason.destinationRequired,
        );
        expect(h.fake.swapQuoteCount, 0);
      },
    );

    test(
      'a repeated same-reason validation fault is a FRESH instance — the '
      'screen\'s scroll-to-fault listener re-fires on identity (#364 S5)',
      () async {
        final h = harness();
        await quoteOut(h.container, destination: '   ');
        final first = (st(h.container) as SwapFormState).fault;
        await quoteOut(h.container, destination: '   ');
        final second = (st(h.container) as SwapFormState).fault;
        expect(
          (second! as SwapCategoricalFault).reason,
          SwapFaultReason.destinationRequired,
        );
        expect(
          identical(first, second),
          isFalse,
          reason:
              'a const-canonicalized fault makes the repeat invisible to '
              'the identity-keyed scroll/announce listener (the send form\'s '
              '#329-2 invariant)',
        );
      },
    );

    test('the requested slippage flows into the quote request', () async {
      final h = harness();
      h.fake.swapQuoteResult = swapQuoteFixture();
      await quoteOut(h.container, slippageBps: 350);
      expect(h.fake.lastSwapQuoteRequest!.slippageToleranceBps, 350);
    });

    test(
      'a typed quote failure returns to the form with an honest fault',
      () async {
        final h = harness();
        h.fake.swapQuoteThrows = swapErr(
          const SwapErrorKind.quoteOutOfBounds(side: QuoteBoundSide.foreign),
        );
        await quoteOut(h.container);
        final s = st(h.container) as SwapFormState;
        expect(
          (s.fault as SwapCategoricalFault).reason,
          SwapFaultReason.quoteOutOfBounds,
        );
      },
    );

    test(
      'a second quote while one is in flight is ignored (double-tap)',
      () async {
        final h = harness();
        final f1 = quoteOut(h.container);
        // Synchronously re-tap: the first set SwapQuoting before its await.
        final f2 = quoteOut(h.container);
        await f1;
        await f2;
        expect(h.fake.swapQuoteCount, 1);
      },
    );
  });

  group('IntoZec quote → review', () {
    test('lowers the form to an exact IntoZec quote request', () async {
      final h = harness();
      h.fake.swapQuoteResult = intoZecQuote();
      await quoteIn(h.container, amountText: '100', refundAddress: '  bc1qr  ');

      final s = st(h.container);
      expect(s, isA<SwapReview>());
      expect((s as SwapReview).direction, SwapFlowDirection.intoZec);
      expect(s.counter, btc);

      // The money-shaping: IntoZec, exact FOREIGN in (the decimal string verbatim
      // — NOT parsed to zatoshis host-side), NO destination (delivery is the
      // wallet's own fresh address), the trimmed refund address, the slippage.
      final req = h.fake.lastSwapQuoteRequest!;
      expect(req.direction, isA<SwapDirection_IntoZec>());
      final dir = req.direction as SwapDirection_IntoZec;
      expect(dir.from.chain, btc.chain);
      expect(dir.from.symbol, btc.symbol);
      final amount = (req.exact as ExactSide_In).amount;
      expect(amount, isA<SwapAmount_Foreign>());
      expect((amount as SwapAmount_Foreign).amount, '100');
      expect(req.destination, isNull);
      expect(req.refundAddress, 'bc1qr');
      expect(req.slippageToleranceBps, 200);
    });

    test(
      'an empty foreign amount is rejected host-side (no bridge call)',
      () async {
        final h = harness();
        await quoteIn(h.container, amountText: '   ');
        final s = st(h.container) as SwapFormState;
        expect(
          (s.fault as SwapCategoricalFault).reason,
          SwapFaultReason.foreignAmountRequired,
        );
        expect(h.fake.swapQuoteCount, 0);
      },
    );

    test(
      'an empty refund address is rejected host-side (no bridge call)',
      () async {
        final h = harness();
        await quoteIn(h.container, refundAddress: '  ');
        final s = st(h.container) as SwapFormState;
        expect(
          (s.fault as SwapCategoricalFault).reason,
          SwapFaultReason.refundAddressRequired,
        );
        expect(h.fake.swapQuoteCount, 0);
      },
    );

    test(
      'the foreign amount is NOT parsed to zatoshis — letters reach the SDK',
      () async {
        // A foreign decimal the host must NOT reject (it's validated Rust-side):
        // a value with many decimals that `parseZecAmount` would reject must still
        // pass through. The fake accepts it; the assertion is that it crossed.
        final h = harness();
        h.fake.swapQuoteResult = intoZecQuote();
        await quoteIn(h.container, amountText: '0.000000001234');
        final amount =
            (h.fake.lastSwapQuoteRequest!.exact as ExactSide_In).amount;
        expect((amount as SwapAmount_Foreign).amount, '0.000000001234');
      },
    );
  });

  group('execute → tracking', () {
    Future<void> reviewedOut(
      FakeWalletSession fake,
      ProviderContainer c,
    ) async {
      fake.swapQuoteResult = swapQuoteFixture();
      await quoteOut(c);
      expect(c.read(swapControllerProvider), isA<SwapReview>());
    }

    test(
      'OutOfZec executes the reviewed quote and tracks the minted id',
      () async {
        final h = harness();
        h.fake.swapExecuteResult = 'swap-exec-1';
        await reviewedOut(h.fake, h.container);
        await ctl(h.container).execute();
        final s = st(h.container);
        expect(s, isA<SwapExecuted>());
        expect((s as SwapExecuted).swapId, 'swap-exec-1');
        expect(s.direction, SwapFlowDirection.outOfZec);
        expect(h.fake.swapExecuteCount, 1);
        expect(h.fake.lastSwapExecuteQuote, isNotNull);
      },
    );

    test(
      'IntoZec executes to the deposit screen, NOT straight to tracking',
      () async {
        final h = harness();
        h.fake.swapQuoteResult = intoZecQuote();
        h.fake.swapExecuteResult = 'swap-iz-1';
        await quoteIn(h.container);
        expect(st(h.container), isA<SwapReview>());
        await ctl(h.container).execute();
        final s = st(h.container);
        // IntoZec lands on the D7 deposit screen (the USER sends the deposit).
        expect(s, isA<SwapAwaitingDeposit>());
        expect((s as SwapAwaitingDeposit).swapId, 'swap-iz-1');
        expect(s.counter, btc);
        expect(s.quote.id, 'swap-iz');
      },
    );

    test('markDepositSent advances IntoZec deposit → live tracking', () async {
      final h = harness();
      h.fake.swapQuoteResult = intoZecQuote();
      h.fake.swapExecuteResult = 'swap-iz-2';
      await quoteIn(h.container);
      await ctl(h.container).execute();
      expect(st(h.container), isA<SwapAwaitingDeposit>());
      ctl(h.container).markDepositSent();
      final s = st(h.container);
      expect(s, isA<SwapExecuted>());
      expect((s as SwapExecuted).swapId, 'swap-iz-2');
      expect(s.direction, SwapFlowDirection.intoZec);
    });

    test('markDepositSent is a no-op off the deposit screen', () async {
      final h = harness();
      ctl(h.container).markDepositSent();
      expect(st(h.container), isA<SwapFormState>());
    });

    test('execute is a no-op unless on the review screen', () async {
      final h = harness();
      await ctl(h.container).execute();
      expect(st(h.container), isA<SwapFormState>());
      expect(h.fake.swapExecuteCount, 0);
    });

    test('any execute failure routes back to the form (re-quote)', () async {
      final h = harness();
      h.fake.swapExecuteThrows = swapErr(const SwapErrorKind.quoteExpired());
      await reviewedOut(h.fake, h.container);
      await ctl(h.container).execute();
      final s = st(h.container) as SwapFormState;
      expect(
        (s.fault as SwapCategoricalFault).reason,
        SwapFaultReason.quoteExpired,
      );
    });

    test(
      'a deposit-send failure is the honest "no ZEC moved, re-quote" fault',
      () async {
        final h = harness();
        h.fake.swapExecuteThrows = swapErr(
          const SwapErrorKind.depositSendFailed(),
        );
        await reviewedOut(h.fake, h.container);
        await ctl(h.container).execute();
        final s = st(h.container) as SwapFormState;
        expect(
          (s.fault as SwapCategoricalFault).reason,
          SwapFaultReason.depositFailed,
        );
      },
    );

    test(
      'a second execute while one is in flight is ignored (double-tap)',
      () async {
        final h = harness();
        await reviewedOut(h.fake, h.container);
        final f1 = ctl(h.container).execute();
        final f2 = ctl(h.container).execute(); // state is SwapExecuting now
        await f1;
        await f2;
        expect(h.fake.swapExecuteCount, 1);
      },
    );

    test('an OutOfZec execute TIMEOUT tracks by quote id — never the re-quote '
        'form (W-swap-4-a-2 timeout→track)', () async {
      // Dart's .timeout ABANDONS the Rust future: by the timeout the durable
      // deposit enqueue has committed and the sign may still be proving —
      // money may be in motion. The re-quotable form would invite a NEW
      // quote id (the double-deposit door); tracking is the honest arm, and
      // the provider's execute returns the quote id verbatim so the quote
      // id IS the tracking id.
      final h = harness();
      h.fake.swapExecuteThrows = TimeoutException('execute overran');
      await reviewedOut(h.fake, h.container);
      await ctl(h.container).execute();
      final s = st(h.container);
      expect(s, isA<SwapExecuted>(), reason: 'timeout ⇒ tracking, not form');
      expect(
        (s as SwapExecuted).swapId,
        'swap-1',
        reason: 'tracked by the QUOTE id (execute returned nothing)',
      );
      expect(s.direction, SwapFlowDirection.outOfZec);
    });

    test('an IntoZec execute TIMEOUT lands the HEDGED timeout form — never a '
        'firm connection blame (#367; no wallet funds moved)', () async {
      final h = harness();
      h.fake.swapQuoteResult = intoZecQuote();
      h.fake.swapExecuteThrows = TimeoutException('execute overran');
      await quoteIn(h.container);
      expect(st(h.container), isA<SwapReview>());
      await ctl(h.container).execute();
      final s = st(h.container);
      expect(s, isA<SwapFormState>(), reason: 'IntoZec commits no funds');
      expect(
        ((s as SwapFormState).fault as SwapCategoricalFault).reason,
        SwapFaultReason.executeTimedOut,
        reason:
            'a local ~30s store stall can consume the window — the copy '
            'hedges connection vs busy-wallet instead of blaming transport',
      );
    });

    test('the SDK in-flight guard refusal renders the non-re-quote '
        '"swap in progress" fault', () async {
      final h = harness();
      h.fake.swapExecuteThrows = swapErr(
        const SwapErrorKind.swapAlreadyInFlight(),
      );
      await reviewedOut(h.fake, h.container);
      await ctl(h.container).execute();
      final s = st(h.container) as SwapFormState;
      expect(
        (s.fault as SwapCategoricalFault).reason,
        SwapFaultReason.swapInFlight,
      );
    });
  });

  group('navigation + guards', () {
    test('backToForm resets from review', () async {
      final h = harness();
      h.fake.swapQuoteResult = swapQuoteFixture();
      await quoteOut(h.container);
      expect(st(h.container), isA<SwapReview>());
      ctl(h.container).backToForm();
      expect(st(h.container), isA<SwapFormState>());
    });

    test(
      'resetToForm RE-ATTACHES to a live executed swap on re-entry (W-swap-5)',
      () async {
        // INVERTED at #366: pre-#366 re-entry wiped the live tracking state
        // (the one surface following money in motion) — now a committed swap
        // survives re-entry, and leaving it is the explicit startNewSwap.
        final h = harness();
        h.fake.swapQuoteResult = swapQuoteFixture();
        h.fake.swapExecuteResult = 'swap-exec';
        await quoteOut(h.container);
        await ctl(h.container).execute();
        expect(st(h.container), isA<SwapExecuted>());
        ctl(h.container).resetToForm();
        expect(
          st(h.container),
          isA<SwapExecuted>(),
          reason: 're-entry must not wipe a live committed swap',
        );
      },
    );

    test('startNewSwap explicitly leaves a live executed swap', () async {
      final h = harness();
      h.fake.swapQuoteResult = swapQuoteFixture();
      h.fake.swapExecuteResult = 'swap-exec';
      await quoteOut(h.container);
      await ctl(h.container).execute();
      expect(st(h.container), isA<SwapExecuted>());
      ctl(h.container).startNewSwap();
      final s = st(h.container) as SwapFormState;
      expect(s.fault, isNull);
    });

    test('resetToForm still clears a stale review (pre-commitment)', () async {
      final h = harness();
      h.fake.swapQuoteResult = swapQuoteFixture();
      await quoteOut(h.container);
      expect(st(h.container), isA<SwapReview>());
      ctl(h.container).resetToForm();
      expect(st(h.container), isA<SwapFormState>());
    });

    test('attachTo re-attaches tracking from a durable record (#366)', () {
      final h = harness();
      ctl(h.container).attachTo(
        swapId: 'swap-durable-1',
        direction: SwapFlowDirection.outOfZec,
      );
      final s = st(h.container) as SwapExecuted;
      expect(s.swapId, 'swap-durable-1');
      expect(s.direction, SwapFlowDirection.outOfZec);
      // Idempotent second tap: same id → no state churn (same instance).
      final before = st(h.container);
      ctl(h.container).attachTo(
        swapId: 'swap-durable-1',
        direction: SwapFlowDirection.outOfZec,
      );
      expect(identical(st(h.container), before), isTrue);
    });

    test(
      'attachTo NEVER clobbers a live SwapAwaitingDeposit for the same swap '
      '(crypto-review H2) — the deposit instructions are the only copy',
      () async {
        final h = harness();
        h.fake.swapQuoteResult = intoZecQuote();
        h.fake.swapExecuteResult = 'swap-iz-live';
        await quoteIn(h.container);
        await ctl(h.container).execute();
        final deposit = st(h.container);
        expect(deposit, isA<SwapAwaitingDeposit>());
        // The home row lists this swap; a "View swap" tap must RE-ATTACH to
        // the richer deposit state, never replace it with bare tracking.
        ctl(h.container).attachTo(
          swapId: 'swap-iz-live',
          direction: SwapFlowDirection.intoZec,
        );
        expect(
          identical(st(h.container), deposit),
          isTrue,
          reason:
              'the quote (address/amount/memo) is not persisted — '
              'replacing the deposit screen would destroy it',
        );
        // Arch review H-A1 (probe-confirmed pre-fix): a DIFFERENT record's
        // "View swap" must refuse too — the richer state wins unconditionally.
        ctl(h.container).attachTo(
          swapId: 'some-other-swap',
          direction: SwapFlowDirection.outOfZec,
        );
        expect(
          identical(st(h.container), deposit),
          isTrue,
          reason:
              'a different row\'s tap must not clobber live deposit '
              'instructions either',
        );
      },
    );

    test(
      'no live session surfaces walletUnavailable, no bridge call',
      () async {
        final h = harness(nullSession: true);
        await quoteOut(h.container);
        final s = st(h.container) as SwapFormState;
        expect(
          (s.fault as SwapCategoricalFault).reason,
          SwapFaultReason.walletUnavailable,
        );
        expect(h.fake.swapQuoteCount, 0);
      },
    );

    test('a successful re-quote after a fault clears the fault', () async {
      final h = harness();
      await quoteOut(h.container, destination: '');
      expect((st(h.container) as SwapFormState).fault, isNotNull);
      h.fake.swapQuoteResult = swapQuoteFixture();
      await quoteOut(h.container);
      expect(st(h.container), isA<SwapReview>());
    });

    test('the chosen asset flows into the quote request', () async {
      final h = harness();
      const base = SwapAsset(
        chain: 'base',
        symbol: 'usdc',
        label: 'USDC on Base',
      );
      h.fake.swapQuoteResult = swapQuoteFixture();
      await quoteOut(h.container, asset: base);
      final dir =
          h.fake.lastSwapQuoteRequest!.direction as SwapDirection_OutOfZec;
      expect(dir.to.chain, base.chain);
      expect(dir.to.symbol, base.symbol);
    });

    test(
      'disposing during an in-flight quote is safe (no post-dispose write)',
      () async {
        // Own container (no addTearDown auto-dispose) so the test controls dispose.
        final fake = FakeWalletSession();
        final container = ProviderContainer(
          overrides: [walletSessionProvider.overrideWithValue(fake)],
        );
        fake.swapQuoteResult = swapQuoteFixture();
        final f = container
            .read(swapControllerProvider.notifier)
            .quote(
              OutOfZecInput(
                asset: usdc,
                amountText: '0.5',
                destination: '0xd',
                slippageBps: referenceSwapSlippageBps,
              ),
            );
        // Dispose mid-flight: the `_disposed` latch (via ref.onDispose) must make
        // the post-await continuation a no-op — never a throw, never a stale write.
        container.dispose();
        await f; // must not throw
        expect(fake.swapQuoteCount, 1);
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

    test('an OutOfZec execute routes through the authorizer EXACTLY once as a '
        'SWAP-DEPOSIT intent carrying the ZEC side', () async {
      final h = authorized();
      h.fake.swapQuoteResult = swapQuoteFixture(zecSideZat: 50000000);
      await quoteOut(h.container);
      final review = st(h.container) as SwapReview;
      await ctl(h.container).execute();
      expect(st(h.container), isA<SwapExecuted>());
      expect(h.auth.intents, hasLength(1));
      expect(h.auth.intents.single.kind, WalletSpendKind.swapDeposit);
      expect(h.auth.intents.single.amountZat, 50000000);
      // #383 R3: the prompt binds to WHAT is signed — the quote's deposit
      // address, elided for display.
      expect(
        h.auth.intents.single.recipientAbbrev,
        abbreviateWalletAddress(review.quote.depositAddress),
      );
      expect(h.fake.swapExecuteCount, 1);
      // The quote round-trip moves no funds — never authorized.
      expect(h.fake.swapQuoteCount, 1);
    });

    test('the OutOfZec review keeps the exact payout address the quote was '
        'requested for, and the intent keeps it apart from the deposit '
        'address (N03)', () async {
      final h = authorized();
      h.fake.swapQuoteResult = swapQuoteFixture();
      const payout = '0x52908400098527886E0F7030069857D2E4169EE7';
      // Pasted with stray whitespace: the review shows what was QUOTED (the
      // trimmed request value), never the raw field.
      await quoteOut(h.container, destination: '  $payout\n');
      final review = st(h.container) as SwapReview;
      expect(review.payoutAddress, payout);
      await ctl(h.container).execute();
      final intent = h.auth.intents.single;
      expect(intent.payoutAbbrev, abbreviateWalletAddress(payout));
      expect(
        intent.recipientAbbrev,
        abbreviateWalletAddress(review.quote.depositAddress),
      );
      expect(intent.payoutAbbrev, isNot(intent.recipientAbbrev));
    });

    test('a denied prompt restores the review WITH its payout address '
        '(N03)', () async {
      final h = authorized(denyAll: true);
      h.fake.swapQuoteResult = swapQuoteFixture();
      await quoteOut(h.container, destination: '0xpayout');
      await ctl(h.container).execute();
      final restored = st(h.container) as SwapReview;
      expect(restored.payoutAddress, '0xpayout');
    });

    test('an IntoZec review carries no payout address (N03: its payout is '
        "the wallet's own)", () async {
      final h = authorized();
      h.fake.swapQuoteResult = swapQuoteFixture(refundTo: 'bc1qrefund');
      await quoteIn(h.container);
      expect((st(h.container) as SwapReview).payoutAddress, isNull);
    });

    test("the OutOfZec execute-bracket intent carries the quote's "
        'spend-binding nonce byte-for-byte (FR-17 #396)', () async {
      final h = authorized();
      // A distinct fill (not the fixture default) so a pass-through of the
      // WRONG quote's bytes cannot pass by coincidence.
      final binding = Uint8List.fromList(List.filled(32, 0x5A));
      h.fake.swapQuoteResult = swapQuoteFixture(binding: binding);
      await quoteOut(h.container);
      await ctl(h.container).execute();
      expect(st(h.container), isA<SwapExecuted>());
      expect(
        h.auth.intents.single.bindingToken,
        equals(binding),
        reason:
            "the seam must surface the EXACT nonce the deposit's "
            'sign-time seed pull will present',
      );
    });

    test('an IntoZec execute NEVER prompts — no wallet funds move (the user '
        'deposits externally), so a spend prompt would be dishonest', () async {
      // denyAll proves the authorizer is not merely pass-through-invoked:
      // were the IntoZec leg routed through it, the execute would bounce.
      final h = authorized(denyAll: true);
      h.fake.swapQuoteResult = intoZecQuote();
      await quoteIn(h.container);
      await ctl(h.container).execute();
      expect(st(h.container), isA<SwapAwaitingDeposit>());
      expect(h.auth.intents, isEmpty);
      expect(h.fake.swapExecuteCount, 1);
    });

    test(
      'a DENIED OutOfZec execute lands back on the review with ZERO bridge '
      'calls — the single-flight is unclaimed, the quote stays executable',
      () async {
        final h = authorized(denyAll: true);
        h.fake.swapQuoteResult = swapQuoteFixture();
        await quoteOut(h.container);
        final review = st(h.container) as SwapReview;
        await ctl(h.container).execute();
        // A FRESH fault-free review since the #367 review folds (no longer the
        // same instance): a stale busy banner from an earlier attempt must not
        // survive the denial. Same quote, same framing, no fault.
        final restored = st(h.container);
        expect(restored, isA<SwapReview>());
        expect((restored as SwapReview).quote.id, review.quote.id);
        expect(restored.direction, review.direction);
        expect(restored.fault, isNull);
        expect(h.fake.swapExecuteCount, 0);
        // The host re-authorizes → the SAME quote executes (never a re-quote).
        h.auth.denyAll = false;
        await ctl(h.container).execute();
        expect(st(h.container), isA<SwapExecuted>());
        expect(h.fake.swapExecuteCount, 1);
      },
    );

    test("the HOST authorizer's OWN TimeoutException lands on the form — never "
        'the tracking screen (nothing executed; review MED fold)', () async {
      // A host that bounds its biometric/prompt with a Dart .timeout throws
      // the same TimeoutException the SDK-call timeout does — but BEFORE the
      // action ever ran: quote unconsumed, nothing enqueued, no money in
      // motion. Only the SDK call's own overrun (the private sentinel) may
      // route to tracking; this one must classify like any failure.
      final h = authorized();
      h.auth.throwBeforeAction = TimeoutException('host prompt bounded');
      h.fake.swapQuoteResult = swapQuoteFixture();
      await quoteOut(h.container);
      await ctl(h.container).execute();
      final s = st(h.container);
      expect(s, isA<SwapFormState>(), reason: 'never a fake money screen');
      expect(
        ((s as SwapFormState).fault as SwapCategoricalFault).reason,
        SwapFaultReason.connectionFailed,
      );
      expect(h.fake.swapExecuteCount, 0, reason: 'the action never ran');
    });
  });

  group('session-flip liveness (#330)', () {
    // The same two-halves pin as the send controller's group: `_disposed`
    // resets each build (no wedge after a host identity switch), and the
    // post-await identity guards keep the dead cycle from writing into the
    // new one. See send_controller_test.dart for the full race matrix; here
    // the swap-specific arms.
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
      container.listen(swapControllerProvider, (_, _) {});
      return (container: container, sessionSwitch: sessionSwitch);
    }

    test('ONE session flip does not wedge the controller — a quote on the '
        'NEW session still renders', () async {
      final fakeA = FakeWalletSession();
      final fakeB = FakeWalletSession()..swapQuoteResult = swapQuoteFixture();
      final h = flippable(fakeA);

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      await quoteOut(h.container);
      expect(
        st(h.container),
        isA<SwapReview>(),
        reason: 'one identity switch must not leave the swap screen dead',
      );
      expect(fakeB.swapQuoteCount, 1);
      expect(fakeA.swapQuoteCount, 0);
    });

    test('a quote resolving AFTER a mid-flight flip writes NOTHING into the '
        'new cycle — a dead session\'s quote is never executable', () async {
      final fakeA = FakeWalletSession()
        ..swapQuoteGate = Completer<void>()
        ..swapQuoteResult = swapQuoteFixture();
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA);

      final pending = quoteOut(h.container);
      expect(st(h.container), isA<SwapQuoting>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);
      expect(st(h.container), isA<SwapFormState>());

      fakeA.swapQuoteGate!.complete();
      await pending;
      final s = st(h.container);
      expect(s, isA<SwapFormState>());
      expect(
        (s as SwapFormState).fault,
        isNull,
        reason:
            'the dead cycle\'s review (an EXECUTABLE quote) must never '
            'surface over the fresh session',
      );
    });

    test('a denial landing AFTER a mid-prompt flip never restores the dead '
        'session\'s review', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(
        denyAll: true,
        prompt: (_) => gate.future,
      );
      final fakeA = FakeWalletSession()..swapQuoteResult = swapQuoteFixture();
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA, auth: auth);

      await quoteOut(h.container);
      final execute = ctl(h.container).execute(); // parks at the host prompt
      expect(st(h.container), isA<SwapExecuting>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      gate.complete();
      await execute;
      final s = st(h.container);
      expect(s, isA<SwapFormState>());
      expect(
        (s as SwapFormState).fault,
        isNull,
        reason: 'no dead-session SwapReview over the fresh cycle',
      );
      expect(fakeA.swapExecuteCount, 0);
      expect(fakeB.swapExecuteCount, 0);
    });

    test(
      'a denial landing while the NEW cycle\'s OWN execute is in flight '
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
        final fakeA = FakeWalletSession()..swapQuoteResult = swapQuoteFixture();
        final fakeB = FakeWalletSession()..swapQuoteResult = swapQuoteFixture();
        final h = flippable(fakeA, auth: auth);

        await quoteOut(h.container);
        final executeA = ctl(h.container).execute(); // parks (call 1)

        h.container.read(h.sessionSwitch.notifier).state = fakeB;
        await Future<void>.delayed(Duration.zero);

        await quoteOut(h.container);
        unawaited(ctl(h.container).execute()); // parks forever (call 2)
        final executingB = st(h.container);
        expect(executingB, isA<SwapExecuting>());

        gateA
            .complete(); // a type-only guard would restore the DEAD review here
        await executeA;
        expect(st(h.container), same(executingB));
        expect(fakeA.swapExecuteCount, 0);
        expect(fakeB.swapExecuteCount, 0);
      },
    );

    test('a dead cycle\'s quote resolving while the NEW cycle is ITSELF '
        'Quoting writes nothing — distinct transient INSTANCES, not a '
        'canonical const (S153 review F2)', () async {
      final fakeA = FakeWalletSession()
        ..swapQuoteGate = Completer<void>()
        ..swapQuoteResult = swapQuoteFixture(id: 'swap-a');
      final fakeB = FakeWalletSession()
        ..swapQuoteGate = Completer<void>()
        ..swapQuoteResult = swapQuoteFixture(id: 'swap-b');
      final h = flippable(fakeA);

      final pendingA = quoteOut(h.container);
      expect(st(h.container), isA<SwapQuoting>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      final pendingB = quoteOut(h.container);
      final quotingB = st(h.container);
      expect(quotingB, isA<SwapQuoting>());

      fakeA.swapQuoteGate!.complete();
      await pendingA;
      expect(
        st(h.container),
        same(quotingB),
        reason:
            'the dead session\'s EXECUTABLE quote must not render '
            'while the new cycle is still quoting',
      );

      fakeB.swapQuoteGate!.complete();
      await pendingB;
      expect((st(h.container) as SwapReview).quote.id, 'swap-b');
    });

    test('an APPROVAL landing after a mid-prompt flip is FENCED — the deposit '
        'commits on NEITHER session (S153 wrap review: swapExecute is '
        'tokenless, so the fence is its only backstop)', () async {
      final gate = Completer<void>();
      final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
      final fakeA = FakeWalletSession()..swapQuoteResult = swapQuoteFixture();
      final fakeB = FakeWalletSession();
      final h = flippable(fakeA, auth: auth);

      await quoteOut(h.container);
      final execute = ctl(h.container).execute(); // parks at the host prompt
      expect(st(h.container), isA<SwapExecuting>());

      h.container.read(h.sessionSwitch.notifier).state = fakeB;
      await Future<void>.delayed(Duration.zero);

      gate.complete(); // the prompt APPROVES after the identity switch
      await execute;
      expect(st(h.container), isA<SwapFormState>());
      expect(
        fakeA.swapExecuteCount,
        0,
        reason: 'the fence refused the dead identity\'s deposit commit',
      );
      expect(fakeB.swapExecuteCount, 0);
    });

    test(
      'screen re-entry mid-execute RE-ATTACHES (S153 review F1): '
      'resetToForm() is a no-op in flight, so the landing outcome still '
      'renders — a swallowed SwapExecuted would invite a SECOND deposit',
      () async {
        final gate = Completer<void>();
        final auth = FakeSendAuthorizer(prompt: (_) => gate.future);
        final fake = FakeWalletSession()..swapQuoteResult = swapQuoteFixture();
        final h = flippable(fake, auth: auth);

        await quoteOut(h.container);
        final execute = ctl(h.container).execute(); // parks at the host prompt
        final executing = st(h.container);
        expect(executing, isA<SwapExecuting>());

        ctl(h.container).resetToForm(); // the screen's entry hook, re-entered
        expect(st(h.container), same(executing));

        gate.complete();
        await execute;
        expect(st(h.container), isA<SwapExecuted>());
        expect(fake.swapExecuteCount, 1);
      },
    );
  });

  group('#367 spendable pre-check + busy honesty', () {
    /// Load the snapshot VIEW so the pre-check sees a value (in production the
    /// wallet screen keeps it warm; the pre-check itself fails OPEN on a
    /// still-loading snapshot — the engine gate is the money backstop). The
    /// view is push-fed off an autoDispose read, so it needs a LIVE listener
    /// in a bare container (a plain read would dispose between polls).
    Future<void> warmSnapshot(ProviderContainer c) async {
      final sub = c.listen(walletSnapshotProvider, (_, _) {});
      addTearDown(sub.close);
      for (
        var i = 0;
        i < 20 && c.read(walletSnapshotProvider).value == null;
        i++
      ) {
        await Future<void>.delayed(Duration.zero);
      }
      expect(c.read(walletSnapshotProvider).value, isNotNull);
    }

    test(
      'an OutOfZec quote is REFUSED at review when spendable cannot cover '
      'the deposit plus the fee allowance (the S190-b blast radius)',
      () async {
        final h = harness();
        h.fake.setSnapshot(
          walletStateFixture(balance: balanceFixture(spendableZat: 1000)),
        );
        await warmSnapshot(h.container);
        h.fake.swapQuoteResult = swapQuoteFixture(
          id: 'swap-poor',
          zecSideZat: 50000000,
        );
        await quoteOut(h.container);
        final s = st(h.container);
        expect(s, isA<SwapFormState>(), reason: 'refused before review');
        final fault = (s as SwapFormState).fault;
        expect(fault, isA<SwapInsufficientSpendable>());
        final f = fault as SwapInsufficientSpendable;
        expect(f.neededZat, 50000000 + kSwapDepositFeeAllowanceZat);
        expect(f.spendableZat, 1000);
        expect(f.catchingUp, isFalse);
      },
    );

    test('the refusal HEDGES while the balance is catching up', () async {
      final fake = FakeWalletSession()
        ..setSnapshot(
          walletStateFixture(balance: balanceFixture(spendableZat: 0)),
        );
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletCatchUpCueProvider.overrideWithValue(
            const WalletCatchUpSyncing(),
          ),
        ],
      );
      addTearDown(container.dispose);
      await warmSnapshot(container);
      fake.swapQuoteResult = swapQuoteFixture(
        id: 'swap-catching',
        zecSideZat: 50000000,
      );
      await ctl(container).quote(
        OutOfZecInput(
          asset: usdc,
          amountText: '0.5',
          destination: '0xd',
          slippageBps: referenceSwapSlippageBps,
        ),
      );
      final fault = (st(container) as SwapFormState).fault;
      expect((fault as SwapInsufficientSpendable).catchingUp, isTrue);
    });

    test('a sufficient balance sails through to review (deposit + allowance '
        'exactly covered)', () async {
      final h = harness();
      h.fake.setSnapshot(
        walletStateFixture(
          balance: balanceFixture(
            spendableZat: 50000000 + kSwapDepositFeeAllowanceZat,
          ),
        ),
      );
      await warmSnapshot(h.container);
      h.fake.swapQuoteResult = swapQuoteFixture(
        id: 'swap-ok',
        zecSideZat: 50000000,
      );
      await quoteOut(h.container);
      expect(st(h.container), isA<SwapReview>());
    });

    test('a store-busy EXECUTE keeps the review up with the retryable fault — '
        'the quote is unconsumed, a forced re-quote would burn it', () async {
      final h = harness();
      h.fake.swapQuoteResult = swapQuoteFixture(id: 'swap-busy');
      await quoteOut(h.container);
      expect(st(h.container), isA<SwapReview>());
      h.fake.swapExecuteThrows = swapErr(const SwapErrorKind.swapStateBusy());
      await ctl(h.container).execute();
      final s = st(h.container);
      expect(s, isA<SwapReview>(), reason: 'the review survives a busy');
      final review = s as SwapReview;
      expect(review.quote.id, 'swap-busy', reason: 'same quote, retryable');
      expect(
        (review.fault as SwapCategoricalFault?)?.reason,
        SwapFaultReason.storeBusy,
      );
      // The retry succeeds against the SAME review (nothing was consumed).
      h.fake.swapExecuteThrows = null;
      await ctl(h.container).execute();
      expect(st(h.container), isA<SwapExecuted>());
      expect(h.fake.swapExecuteCount, 2);
    });

    test(
      'a store-busy QUOTE lands on the form with the retryable fault',
      () async {
        final h = harness();
        h.fake.swapQuoteThrows = swapErr(const SwapErrorKind.swapStateBusy());
        await quoteOut(h.container);
        final fault = (st(h.container) as SwapFormState).fault;
        expect(
          (fault as SwapCategoricalFault).reason,
          SwapFaultReason.storeBusy,
        );
      },
    );

    test(
      'the pre-check FAILS OPEN on a still-loading snapshot — review renders, '
      'never a refusal over a figure we do not have (review-fold pin)',
      () async {
        // NO snapshot warm-up: the view provider has no value when the
        // pre-check reads it. The check exists for honesty, not custody — the
        // engine's sign-time gate is the money backstop and the durable home
        // keeps a doomed deposit visible; a fail-CLOSED "fix" here would block
        // quoting on every cold screen entry.
        final h = harness();
        h.fake.setSnapshot(
          walletStateFixture(balance: balanceFixture(spendableZat: 0)),
        );
        h.fake.swapQuoteResult = swapQuoteFixture(
          id: 'swap-cold',
          zecSideZat: 50000000,
        );
        await quoteOut(h.container);
        expect(st(h.container), isA<SwapReview>());
      },
    );
  });
}
