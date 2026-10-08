import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_address_scanner.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_assets.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_controller.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_enabled_provider.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_screen.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_state.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_token_icon.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// IZ-3 real-world-edge swap tests (§3.3b D4/D5/D6/D7 / L6/L7/L8). These fill the
/// MONEY/MOBILE/UNSTABLE-NETWORK gaps the happy-path suites leave open: the
/// guaranteed-floor money-display line, the char-by-char refund-echo render, the
/// direction-flip and re-quote ack/fault hygiene, the defensive em-dash echo path,
/// the IntoZec execute-failure routing, the picker retry re-fetch, the foreign
/// decimal reaching the SDK verbatim, and the odd-symbol/null-price picker labels.
///
/// Style mirrors `swap_screen_test.dart`: l10n via the screen element, the review
/// ListView gets a tall surface so its bottom widgets lay out, and the deposit
/// screen's 1s `Timer.periodic` is torn down with `pumpWidget(const SizedBox())`
/// (never `pumpAndSettle` while it is mounted).
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(SwapScreen)));

/// Key-based IntoZec field finders — reorder-proof (the order is asset →
/// amount → refund; positional `.at(i)` would silently target the wrong field).
final _intoAmountField = find.byKey(const Key('swap-into-amount'));
final _intoRefundField = find.byKey(const Key('swap-into-refund'));

/// Key-based OutOfZec field finders — symmetric with the IntoZec ones (the
/// order is target asset → amount → destination).
final _outAmountField = find.byKey(const Key('swap-out-amount'));
final _outDestinationField = find.byKey(const Key('swap-out-destination'));

Widget _harness({
  required FakeWalletSession session,
  bool swapEnabled = true,
  bool? scannerSupported,
  AddressScanner? scanner,
}) {
  return ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(session),
      swapEnabledProvider.overrideWithValue(swapEnabled),
      screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
      // Scan injection: drive the destination QR scan without a camera (the real
      // ReaderWidget never mounts). The test host is desktop, so the support gate
      // is false unless overridden.
      if (scannerSupported != null)
        addressScannerSupportedProvider.overrideWithValue(scannerSupported),
      if (scanner != null) addressScannerProvider.overrideWithValue(scanner),
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: const SwapScreen(),
    ),
  );
}

FakeWalletSession _funded() => FakeWalletSession(
  current: const SyncStatus.upToDate(tip: 1),
  snapshotValue: walletStateFixture(
    syncStatus: const SyncStatus.upToDate(tip: 1),
    balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
  ),
);

/// An IntoZec-shaped quote: ends shielded, no de-shield, the refund echoed back,
/// the ZEC side is the guaranteed MIN-out floor (the worst case).
SwapQuote _intoZecQuote({
  String id = 'swap-1',
  String refundTo = 'bc1qrefundxyz',
  int zecSideZat = 412248474, // 4.12248474 ZEC
  String amountIn = '100',
}) => swapQuoteFixture(
  id: id,
  amountIn: amountIn,
  minAmountOut: '4.12248474',
  zecSideZat: zecSideZat,
  refundTo: refundTo,
  deshields: false,
  endsShielded: true,
  providerSees: const [
    DisclosureItem.amounts,
    DisclosureItem.crossAssetLink,
    DisclosureItem.sourceAddress,
    DisclosureItem.ipUnlessTor,
  ],
);

Future<void> _tallSurface(WidgetTester tester) async {
  await tester.binding.setSurfaceSize(const Size(1200, 2600));
  addTearDown(() => tester.binding.setSurfaceSize(null));
}

Future<void> _pickSource(
  WidgetTester tester, {
  String label = 'USDC on Ethereum',
}) async {
  await tester.tap(find.byKey(const Key('swap-source-asset-field')));
  await tester.pumpAndSettle();
  // Picker rows show the symbol + a chain chip; the full "SYMBOL on CHAIN" is the
  // row's a11y label — tap by that (robust to the visual declutter).
  await tester.tap(find.bySemanticsLabel(label));
  await tester.pumpAndSettle();
}

/// IntoZec form → review (pick source, fill refund + foreign amount, quote).
Future<void> _intoZecToReview(
  WidgetTester tester, {
  String refund = 'bc1qrefundxyz',
  String amount = '100',
}) async {
  await _pickSource(tester);
  // IntoZec field order: asset → amount → refund. Find by key, not index.
  await tester.enterText(_intoAmountField, amount);
  await tester.enterText(_intoRefundField, refund);
  await tester.tap(find.text(_l10n(tester).walletSwapQuoteButton));
  await tester.pumpAndSettle();
}

/// Flip the form to OutOfZec (Sell ZEC).
Future<void> _selectSell(WidgetTester tester) async {
  await tester.tap(find.text(_l10n(tester).walletSwapDirectionSell));
  await tester.pumpAndSettle();
}

/// Pick an OutOfZec TARGET asset from the shared token picker (Get-quote is gated
/// on it now — there is no static default menu).
Future<void> _pickTarget(
  WidgetTester tester, {
  String label = 'USDC on Ethereum',
}) async {
  await tester.tap(find.byKey(const Key('swap-target-asset-field')));
  await tester.pumpAndSettle();
  await tester.tap(find.bySemanticsLabel(label));
  await tester.pumpAndSettle();
}

void main() {
  // -------------------------------------------------------------------------
  // Money-display correctness: the IntoZec guaranteed-floor line.
  // -------------------------------------------------------------------------
  group('IntoZec guaranteed-floor money line (§3.3b L8)', () {
    testWidgets('renders the exact floor value + the slippage percent', (
      tester,
    ) async {
      await _tallSurface(tester);
      final fake = _funded()..swapQuoteResult = _intoZecQuote();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _intoZecToReview(tester);
      final l10n = _l10n(tester);

      // The floor IS the `zecSideZat` worst case (no fabricated delta): the line
      // pins the exact ZEC floor + the form's slippage % (default 2%). A wrong
      // floor would mislead the user about what they are guaranteed to receive.
      final expected = l10n.walletSwapIntoZecFloorNote('4.12248474', '2');
      expect(find.text(expected), findsOneWidget);
    });

    testWidgets(
      'the floor tracks the quoted zecSideZat, not the min-out string',
      (tester) async {
        await _tallSurface(tester);
        // A quote whose human min-out string differs from the integer-exact floor:
        // the floor line must render the integer-exact `formatZec(zecSideZat)`,
        // never the provider's display string.
        final fake = _funded()
          ..swapQuoteResult = _intoZecQuote(zecSideZat: 50000001); // 0.50000001
        await tester.pumpWidget(_harness(session: fake));
        await tester.pumpAndSettle();
        await _intoZecToReview(tester);
        final l10n = _l10n(tester);
        expect(
          find.text(l10n.walletSwapIntoZecFloorNote('0.50000001', '2')),
          findsOneWidget,
        );
      },
    );

    testWidgets('the floor note is absent for OutOfZec (it is an IntoZec line)', (
      tester,
    ) async {
      await _tallSurface(tester);
      final fake = _funded()..swapQuoteResult = swapQuoteFixture();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _selectSell(tester);
      await _pickTarget(tester);
      await tester.enterText(_outAmountField, '0.5');
      await tester.enterText(_outDestinationField, '0xdest');
      await tester.tap(find.text(_l10n(tester).walletSwapQuoteButton));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      // OutOfZec exact-in has no min-receive floor note (the ZEC side is exact).
      expect(
        find.text(l10n.walletSwapIntoZecFloorNote('4.12248474', '2')),
        findsNothing,
      );
    });
  });

  // -------------------------------------------------------------------------
  // Refund-verification: the FULL echoed address, chunked, char-by-char.
  // -------------------------------------------------------------------------
  group('IntoZec refund-verification echo (§3.3b D6)', () {
    testWidgets('renders the FULL refundTo echo chunked (not truncated)', (
      tester,
    ) async {
      await _tallSurface(tester);
      // A realistic, long bech32 refund address. A truncated/ellipsised render
      // would defeat the char-by-char verification the whole step exists for.
      const refund = 'bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq';
      final fake = _funded()..swapQuoteResult = _intoZecQuote(refundTo: refund);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _intoZecToReview(tester, refund: refund);

      // The screen renders the address in 4-char space-separated chunks; recompute
      // that exact form and assert it is present in full (every char survives).
      final buf = StringBuffer();
      for (var i = 0; i < refund.length; i += 4) {
        if (i > 0) buf.write(' ');
        buf.write(
          refund.substring(i, i + 4 > refund.length ? refund.length : i + 4),
        );
      }
      expect(find.text(buf.toString()), findsOneWidget);
      // And the verification title + the distinct ack are present.
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapRefundVerifyTitle), findsOneWidget);
      expect(find.text(l10n.walletSwapRefundVerifyAck), findsOneWidget);
    });

    testWidgets('an empty refundTo echo renders the em dash and never enables '
        'Start (defensive)', (tester) async {
      await _tallSurface(tester);
      // Defensive: IntoZec always echoes refundTo, but a missing echo must NOT
      // crash and must NOT pass the verification gate — it shows an em dash,
      // and an ack against a dash verifies nothing, so Start stays off.
      final fake = _funded()..swapQuoteResult = _intoZecQuote(refundTo: '');
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _intoZecToReview(tester);
      final l10n = _l10n(tester);

      expect(find.text('—'), findsOneWidget);
      expect(find.text(l10n.walletSwapRefundVerifyTitle), findsOneWidget);

      // Start stays blocked even with BOTH acks ticked.
      FilledButton start() => tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
      );
      expect(start().onPressed, isNull);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapRefundVerifyAck));
      await tester.pumpAndSettle();
      expect(start().onPressed, isNull, reason: 'an ack against a dash');
    });
  });

  // -------------------------------------------------------------------------
  // Direction-flip hygiene: a stale fault/ack never leaks across directions.
  // -------------------------------------------------------------------------
  group('direction flip resets stale state (§3.3b L8)', () {
    testWidgets('flipping Sell→Buy clears a fault from the OutOfZec form', (
      tester,
    ) async {
      await _tallSurface(tester);
      final fake = _funded();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();

      // OutOfZec with a picked target + amount but an empty destination → an
      // honest inline fault.
      await _selectSell(tester);
      await _pickTarget(tester);
      var l10n = _l10n(tester);
      await tester.enterText(_outAmountField, '0.5');
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletSwapFaultDestinationRequired),
        findsOneWidget,
      );

      // Flip back to Buy (IntoZec): `_onDirectionChanged` resets to a clean form
      // so the OutOfZec fault never lingers into the IntoZec form.
      await tester.tap(find.text(l10n.walletSwapDirectionBuy));
      await tester.pumpAndSettle();
      l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapFaultDestinationRequired), findsNothing);
      // Back on the IntoZec form (the source-asset picker cue is present).
      expect(find.text(l10n.walletSwapSourceAssetLabel), findsWidgets);
    });

    testWidgets('flipping Buy→Sell clears a fault from the IntoZec form', (
      tester,
    ) async {
      await _tallSurface(tester);
      final fake = _funded()..swapQuoteResult = _intoZecQuote();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();

      // IntoZec: pick a source, leave refund empty → refund-required fault.
      await _pickSource(tester);
      var l10n = _l10n(tester);
      await tester.enterText(_intoAmountField, '100'); // amount only, no refund
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletSwapFaultRefundAddressRequired),
        findsOneWidget,
      );

      // Flip to Sell (OutOfZec): the IntoZec fault must not leak in.
      await tester.tap(find.text(l10n.walletSwapDirectionSell));
      await tester.pumpAndSettle();
      l10n = _l10n(tester);
      expect(
        find.text(l10n.walletSwapFaultRefundAddressRequired),
        findsNothing,
      );
      expect(find.text(l10n.walletSwapDestinationLabel), findsOneWidget);
    });
  });

  // -------------------------------------------------------------------------
  // Re-quote: a fresh IntoZec review re-requires BOTH acks (no stale survivor).
  // -------------------------------------------------------------------------
  group('re-quote re-requires both IntoZec acks (§2.6 / D6)', () {
    testWidgets('a fresh review after editing inputs clears BOTH prior acks', (
      tester,
    ) async {
      await _tallSurface(tester);
      final fake = _funded()..swapQuoteResult = _intoZecQuote();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _intoZecToReview(tester);
      var l10n = _l10n(tester);

      // Check BOTH acks on the first review.
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapRefundVerifyAck));
      await tester.pumpAndSettle();
      FilledButton start() => tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
      );
      expect(start().onPressed, isNotNull); // both acked → enabled

      // Edit the amount and re-quote: a NEW review must re-require BOTH acks (a
      // stale ack on fresh numbers would defeat the blocking-UX integrity).
      // The re-quote registers a NEW provider order (#364 F10: the ack reset
      // keys on quote OBJECT IDENTITY — never the provider-controlled id; a
      // same-INSTANCE restore after a host-prompt denial keeps the acks, any
      // freshly-quoted review resets — see swap_screen_test's recycled-id
      // attack pin).
      await tester.tap(find.text(l10n.walletSwapBackButton));
      await tester.pumpAndSettle();
      fake.swapQuoteResult = _intoZecQuote(id: 'swap-2');
      await tester.enterText(_intoAmountField, '250');
      await tester.tap(find.text(_l10n(tester).walletSwapQuoteButton));
      await tester.pumpAndSettle();
      l10n = _l10n(tester);

      expect(start().onPressed, isNull); // BOTH reset — Start blocked again
      // Re-checking only ONE leaves it blocked (proves both were cleared).
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      expect(start().onPressed, isNull);
      await tester.tap(find.text(l10n.walletSwapRefundVerifyAck));
      await tester.pumpAndSettle();
      expect(start().onPressed, isNotNull);
    });
  });

  // -------------------------------------------------------------------------
  // IntoZec execute failure routes back to the FORM (re-quote), not the deposit
  // screen — the core may have consumed the single-flight.
  // -------------------------------------------------------------------------
  group('IntoZec execute-failure routing (controller)', () {
    ({ProviderContainer container, FakeWalletSession fake}) ctlHarness() {
      final fake = FakeWalletSession();
      final container = ProviderContainer(
        overrides: [walletSessionProvider.overrideWithValue(fake)],
      );
      addTearDown(container.dispose);
      return (container: container, fake: fake);
    }

    const btc = SwapAsset(chain: 'btc', symbol: 'btc', label: 'BTC on Bitcoin');

    test('an IntoZec execute failure lands on the form fault, NOT the deposit '
        'screen', () async {
      final h = ctlHarness();
      h.fake.swapQuoteResult = _intoZecQuote();
      h.fake.swapExecuteThrows = SwapApiError(
        code: 'RW-TEST',
        message: 'static',
        kind: const SwapErrorKind.requestInvalid(reason: 'already-executed'),
      );
      final ctl = h.container.read(swapControllerProvider.notifier);
      await ctl.quote(
        const IntoZecInput(
          token: btc,
          amountText: '100',
          refundAddress: 'bc1qrefund',
          slippageBps: 200,
        ),
      );
      expect(h.container.read(swapControllerProvider), isA<SwapReview>());

      await ctl.execute();
      final s = h.container.read(swapControllerProvider);
      // Re-quote path: back to the form with an honest fault — never the deposit
      // screen (a failed execute may have consumed the durable single-flight, so
      // re-executing the same quote would be RequestInvalid; re-quote is the only
      // safe path → never a double-deposit).
      expect(s, isA<SwapFormState>());
      expect((s as SwapFormState).fault, isA<SwapCategoricalFault>());
      expect(
        (s.fault as SwapCategoricalFault).reason,
        SwapFaultReason.requestInvalid,
      );
      expect(
        h.container.read(swapControllerProvider),
        isNot(isA<SwapAwaitingDeposit>()),
      );
    });

    test('an IntoZec provider-unavailable execute failure is also re-quote, '
        'not a stuck deposit screen', () async {
      final h = ctlHarness();
      h.fake.swapQuoteResult = _intoZecQuote();
      h.fake.swapExecuteThrows = SwapApiError(
        code: 'RW-TEST',
        message: 'static',
        kind: const SwapErrorKind.providerUnavailable(),
      );
      final ctl = h.container.read(swapControllerProvider.notifier);
      await ctl.quote(
        const IntoZecInput(
          token: btc,
          amountText: '100',
          refundAddress: 'bc1qrefund',
          slippageBps: 200,
        ),
      );
      await ctl.execute();
      final s = h.container.read(swapControllerProvider) as SwapFormState;
      expect(
        (s.fault as SwapCategoricalFault).reason,
        SwapFaultReason.providerUnavailable,
      );
    });
  });

  // -------------------------------------------------------------------------
  // The picker error → retry actually re-invokes the fetch.
  // -------------------------------------------------------------------------
  group('picker error retry re-fetches (§3.3b L6)', () {
    testWidgets('tapping Try again re-invokes swapListTokens and recovers', (
      tester,
    ) async {
      await _tallSurface(tester);
      // First fetch errors (a no-cache first fault); after the user fixes the
      // network, the retry re-invokes the fetch and the list loads.
      final fake = _funded()
        ..swapListTokensThrows = const SwapApiError(
          code: 'RW-SWAP-009',
          message: 'static',
          kind: SwapErrorKind.providerUnavailable(),
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('swap-source-asset-field')));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapPickerError), findsOneWidget);
      final firstCount = fake.swapListTokensCount;

      // The link is healed; tapping Try again re-runs the fetch (ref.invalidate).
      fake.swapListTokensThrows = null;
      await tester.tap(find.text(l10n.walletSwapPickerRetry));
      await tester.pumpAndSettle();

      // The fetch was re-invoked AND the recovered list now renders.
      expect(fake.swapListTokensCount, greaterThan(firstCount));
      expect(find.bySemanticsLabel('USDC on Ethereum'), findsOneWidget);
      expect(find.text(l10n.walletSwapPickerError), findsNothing);
    });
  });

  // -------------------------------------------------------------------------
  // Foreign decimal reaches the SDK verbatim (never zatoshi-parsed/mangled).
  // -------------------------------------------------------------------------
  group('foreign amount passthrough is verbatim (§3.3b L8)', () {
    ({ProviderContainer container, FakeWalletSession fake}) ctlHarness() {
      final fake = FakeWalletSession();
      final container = ProviderContainer(
        overrides: [walletSessionProvider.overrideWithValue(fake)],
      );
      addTearDown(container.dispose);
      return (container: container, fake: fake);
    }

    const btc = SwapAsset(chain: 'btc', symbol: 'btc', label: 'BTC on Bitcoin');

    // Real-world foreign-decimal shapes the host must NOT reject or reformat:
    // many decimals (an 18-dp ERC-20), a leading zero, a trailing dot, and a
    // bare-dot fraction. parseZecAmount would reject several of these — they must
    // still cross to the SDK byte-for-byte (the SDK is the only amount gate).
    const cases = <String>[
      '0.000000000000000001', // 18-dp wei-scale
      '0123.40', // leading zero kept
      '100.', // trailing dot kept (UX-allowed by the input formatter)
      '.5', // bare-dot fraction
      '00100', // multiple leading zeros
    ];

    for (final amount in cases) {
      test('"$amount" reaches the SDK byte-for-byte', () async {
        final h = ctlHarness();
        h.fake.swapQuoteResult = _intoZecQuote();
        await h.container
            .read(swapControllerProvider.notifier)
            .quote(
              IntoZecInput(
                token: btc,
                amountText: amount,
                refundAddress: 'bc1qrefund',
                slippageBps: 200,
              ),
            );
        final exact = h.fake.lastSwapQuoteRequest!.exact as ExactSide_In;
        final fa = exact.amount as SwapAmount_Foreign;
        // The exact characters the user typed — no zatoshi parse, no normalize.
        expect(fa.amount, amount);
        expect(h.fake.swapQuoteCount, 1);
      });
    }

    test(
      'surrounding whitespace is trimmed but the inner decimal is verbatim',
      () async {
        final h = ctlHarness();
        h.fake.swapQuoteResult = _intoZecQuote();
        await h.container
            .read(swapControllerProvider.notifier)
            .quote(
              const IntoZecInput(
                token: btc,
                amountText: '  0.000000001234  ',
                refundAddress: 'bc1qrefund',
                slippageBps: 200,
              ),
            );
        final exact = h.fake.lastSwapQuoteRequest!.exact as ExactSide_In;
        expect((exact.amount as SwapAmount_Foreign).amount, '0.000000001234');
      },
    );
  });

  // -------------------------------------------------------------------------
  // The picker renders odd-symbol / null-price tokens without crashing, and the
  // monogram never throws on a short/odd symbol.
  // -------------------------------------------------------------------------
  group('picker renders odd-symbol / null-price tokens (§3.3b D5)', () {
    testWidgets('a token with a null price still renders its label', (
      tester,
    ) async {
      await _tallSurface(tester);
      // A token the SDK forwarded with no USD price (rare, but possible): the
      // picker label is symbol+chain only, so it must render regardless.
      final fake = _funded()
        ..swapListTokensResult = SwapTokenList(
          fresh: true,
          tokens: const [
            SwapToken(
              chain: 'sol',
              symbol: 'wen',
              decimals: 5,
              providerAssetId: 'nep141:sol.wen',
              priceUsd: null,
            ),
          ],
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('swap-source-asset-field')));
      await tester.pumpAndSettle();
      // Label is uppercased symbol on uppercased chain — no price involved.
      expect(find.bySemanticsLabel('WEN on Solana'), findsOneWidget);
    });

    testWidgets('the monogram never crashes on a single-char or odd symbol', (
      tester,
    ) async {
      await _tallSurface(tester);
      // A 1-char symbol exercises the monogram's `_initials` boundary (>=2 vs 1);
      // an odd symbol exercises the deterministic tint seed. Neither may throw.
      final fake = _funded()
        ..swapListTokensResult = SwapTokenList(
          fresh: true,
          tokens: const [
            SwapToken(
              chain: 'eth',
              symbol: 'x',
              decimals: 18,
              providerAssetId: 'nep141:eth.x',
              priceUsd: 0.01,
            ),
            SwapToken(
              chain: 'eth',
              symbol: r'$pepe',
              decimals: 18,
              providerAssetId: r'nep141:eth.$pepe',
              priceUsd: null,
            ),
          ],
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('swap-source-asset-field')));
      await tester.pumpAndSettle();
      // Both rows render (the monograms drew without throwing).
      expect(find.bySemanticsLabel('X on Ethereum'), findsOneWidget);
      expect(find.bySemanticsLabel(r'$PEPE on Ethereum'), findsOneWidget);
      // The monogram widget itself is mounted for each token.
      expect(find.byType(SwapTokenIcon), findsNWidgets(2));
    });
  });

  // -------------------------------------------------------------------------
  // MONEY / cross-chain residual: re-picking a TARGET on a DIFFERENT chain
  // KEEPS the typed destination but re-aims the chain-aware label/helper/icon.
  // Pins TODAY's accepted residual (the field is NOT cleared on a target swap);
  // if the team later decides to clear-on-change, this test flips deliberately.
  // -------------------------------------------------------------------------
  group('OutOfZec cross-chain re-pick residual (§3.3b L8 / D5)', () {
    testWidgets('re-picking a target on a different chain KEEPS the destination text '
        'and re-aims the chain-aware label, helper, and icon', (tester) async {
      await _tallSurface(tester);
      await tester.pumpWidget(_harness(session: _funded()));
      await tester.pumpAndSettle();
      await _selectSell(tester);
      final l10n = _l10n(tester);

      // Pick target A (USDC on Ethereum) and type an ETH-shaped destination.
      await _pickTarget(tester); // default: USDC on Ethereum
      const dest = '0x742d35Cc6634C0532925a3b844Bc454e4438f44e';
      await tester.enterText(_outDestinationField, dest);
      await tester.pumpAndSettle();
      // The field is chain-aware for Ethereum now (label + the funds-safety helper).
      expect(
        find.text(l10n.walletSwapDestinationLabelChain('Ethereum')),
        findsOneWidget,
      );
      expect(
        find.text(l10n.walletSwapDestinationHelperChain('Ethereum')),
        findsOneWidget,
      );

      // Re-pick target B on a DIFFERENT chain (BTC on Bitcoin).
      await _pickTarget(tester, label: 'BTC on Bitcoin');

      // RESIDUAL (pinned): the destination text SURVIVES the target change — the
      // field is not cleared. A user who changes their mind about the asset keeps
      // what they typed; the SAFETY net is the now-Bitcoin-aware label/helper/icon
      // shouting "double-check the chain", not a silent wipe.
      expect(
        tester.widget<TextField>(_outDestinationField).controller!.text,
        dest,
      );

      // The chain-aware UI RE-AIMS to Bitcoin (the cross-chain-mistake guard now
      // names the NEW chain), and the stale Ethereum copy is gone.
      expect(
        find.text(l10n.walletSwapDestinationLabelChain('Bitcoin')),
        findsOneWidget,
      );
      expect(
        find.text(l10n.walletSwapDestinationHelperChain('Bitcoin')),
        findsOneWidget,
      );
      expect(
        find.text(l10n.walletSwapDestinationLabelChain('Ethereum')),
        findsNothing,
      );

      // The destination prefix icon now marks the BTC chain (not the prior ETH).
      final destIcon = find.descendant(
        of: _outDestinationField,
        matching: find.byType(SwapTokenIcon),
      );
      expect(tester.widget<SwapTokenIcon>(destIcon).chain, 'btc');
    });
  });

  // -------------------------------------------------------------------------
  // UNSTABLE NETWORK: the OutOfZec TARGET picker inherits the shared token
  // provider's honest-degradation states — reached via the Sell direction it
  // must degrade exactly like the IntoZec source picker (§3.3b L6).
  // -------------------------------------------------------------------------
  group('OutOfZec target picker degrades honestly (§3.3b L6)', () {
    Future<void> openTargetPicker(
      WidgetTester tester,
      FakeWalletSession fake,
    ) async {
      await _tallSurface(tester);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _selectSell(tester);
      await tester.tap(find.byKey(const Key('swap-target-asset-field')));
      await tester.pumpAndSettle();
    }

    testWidgets('a stale list shows the "couldn\'t refresh" banner via Sell', (
      tester,
    ) async {
      final fake = _funded()
        ..swapListTokensResult = swapTokenListFixture(fresh: false);
      await openTargetPicker(tester, fake);
      expect(find.text(_l10n(tester).walletSwapPickerStale), findsOneWidget);
      // The cached tokens are still usable (serve-stale, not serve-nothing).
      expect(find.bySemanticsLabel('USDC on Ethereum'), findsOneWidget);
    });

    testWidgets(
      'an empty fresh list shows the honest no-assets state via Sell',
      (tester) async {
        final fake = _funded()
          ..swapListTokensResult = swapTokenListFixture(
            tokens: const [],
            fresh: true,
          );
        await openTargetPicker(tester, fake);
        expect(find.text(_l10n(tester).walletSwapPickerEmpty), findsOneWidget);
      },
    );

    testWidgets('a no-cache fetch error shows error + retry via Sell, and retry '
        're-invokes the fetch', (tester) async {
      final fake = _funded()
        ..swapListTokensThrows = const SwapApiError(
          code: 'RW-SWAP-009',
          message: 'static',
          kind: SwapErrorKind.providerUnavailable(),
        );
      await openTargetPicker(tester, fake);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapPickerError), findsOneWidget);
      expect(find.text(l10n.walletSwapPickerRetry), findsOneWidget);
      final firstCount = fake.swapListTokensCount;

      // Heal the link; Try again re-runs the fetch and the list recovers — the
      // OutOfZec picker is not wedged on the first fault.
      fake.swapListTokensThrows = null;
      await tester.tap(find.text(l10n.walletSwapPickerRetry));
      await tester.pumpAndSettle();
      expect(fake.swapListTokensCount, greaterThan(firstCount));
      expect(find.bySemanticsLabel('USDC on Ethereum'), findsOneWidget);
    });
  });

  // -------------------------------------------------------------------------
  // MONEY-correctness: a long, mixed-case typed destination crosses to the
  // quote request byte-for-byte — the OutOfZec analog of the IntoZec
  // refund-verbatim assertion, on the `destination` money field (§3.3b L8).
  // -------------------------------------------------------------------------
  group('OutOfZec destination passthrough is verbatim (§3.3b L8)', () {
    testWidgets('a long EIP-55 mixed-case destination reaches the quote request '
        'byte-for-byte (no truncation, no lower-casing)', (tester) async {
      await _tallSurface(tester);
      // A full 42-char checksummed ETH address: truncating it (the single-line
      // tail-scroll trap) OR lower-casing it (defeating the EIP-55 checksum) would
      // send funds to a wrong/invalid address. It must cross verbatim.
      const dest = '0x742d35Cc6634C0532925a3b844Bc454e4438f44e';
      final fake = _funded()..swapQuoteResult = swapQuoteFixture();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _selectSell(tester);
      await _pickTarget(tester);
      await tester.enterText(_outAmountField, '0.5');
      await tester.enterText(_outDestinationField, dest);
      await tester.tap(find.text(_l10n(tester).walletSwapQuoteButton));
      await tester.pumpAndSettle();

      expect(fake.lastSwapQuoteRequest!.destination, dest);
      expect(fake.swapQuoteCount, 1);
    });
  });

  // -------------------------------------------------------------------------
  // MOBILE input hygiene: the destination field's multiline/monospace config
  // (a long address is fully visible, not tail-scrolled) and the amount
  // field's digits+dot input formatter (a pasted non-numeric is filtered).
  // -------------------------------------------------------------------------
  group('OutOfZec field input hygiene (mobile)', () {
    testWidgets('the destination field is multiline monospace and accepts a '
        'long address in full', (tester) async {
      await _tallSurface(tester);
      await tester.pumpWidget(_harness(session: _funded()));
      await tester.pumpAndSettle();
      await _selectSell(tester);
      await _pickTarget(tester);

      final field = tester.widget<TextField>(_outDestinationField);
      expect(field.minLines, 1);
      expect(
        field.maxLines,
        3,
      ); // wraps up to 3 lines, never a 1-line tail-scroll
      expect(field.style?.fontFamily, 'monospace'); // legibility for hex/base58

      // A long address is accepted in full — no maxLength clips a real address.
      const longDest =
          '0x742d35Cc6634C0532925a3b844Bc454e4438f44e0000aabbccddeeff';
      await tester.enterText(_outDestinationField, longDest);
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(_outDestinationField).controller!.text,
        longDest,
      );
    });

    testWidgets('the amount field refuses a pasted non-numeric whole, and '
        'reads a comma as the point', (tester) async {
      await _tallSurface(tester);
      await tester.pumpWidget(_harness(session: _funded()));
      await tester.pumpAndSettle();
      await _selectSell(tester);

      // A fat-fingered / pasted value mixing letters into the ZEC amount is
      // refused WHOLE (S13 M1): stripping the letters would make a different
      // number out of it. UX guard; parseZecAmount is the real gate.
      await tester.enterText(_outAmountField, '1a2b.3c4');
      await tester.pumpAndSettle();
      expect(tester.widget<TextField>(_outAmountField).controller!.text, '');
      await tester.enterText(_outAmountField, '12,34');
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(_outAmountField).controller!.text,
        '12.34',
      );
    });
  });

  // -------------------------------------------------------------------------
  // Re-entrancy/UX: a SCANNED destination is read LIVE at quote time, never
  // captured at the scan moment — editing the amount and re-quoting still
  // sends the scanned destination (no stale capture).
  // -------------------------------------------------------------------------
  group('OutOfZec destination is read live at quote time', () {
    testWidgets('scanning a destination, then editing the amount and '
        're-quoting, still sends the scanned destination', (tester) async {
      await _tallSurface(tester);
      final fake = _funded()..swapQuoteResult = swapQuoteFixture();
      await tester.pumpWidget(
        _harness(
          session: fake,
          scannerSupported: true,
          // A standard EIP-681 wallet QR → URI-unwrapped to the bare 0x address.
          scanner: (_) async => 'ethereum:0xLIVEDEST',
        ),
      );
      await tester.pumpAndSettle();
      await _selectSell(tester);
      await _pickTarget(tester);
      final l10n = _l10n(tester);

      // Scan the destination, then enter the amount and quote.
      await tester.tap(find.byTooltip(l10n.walletSwapDestinationScanTooltip));
      await tester.pumpAndSettle();
      await tester.enterText(_outAmountField, '0.5');
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      expect(fake.lastSwapQuoteRequest!.destination, '0xLIVEDEST');
      expect(fake.swapQuoteCount, 1);

      // Back to the form, change ONLY the amount, re-quote: the destination is
      // re-read LIVE from the field — still the scanned value, never a stale
      // capture from the first quote or the scan moment.
      await tester.tap(find.text(l10n.walletSwapBackButton));
      await tester.pumpAndSettle();
      await tester.enterText(_outAmountField, '1.5');
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      expect(fake.lastSwapQuoteRequest!.destination, '0xLIVEDEST');
      expect(fake.swapQuoteCount, 2);
    });
  });

  testWidgets(
    'S13: IntoZec "0,5" reaches the SDK as the foreign decimal "0.5"',
    (tester) async {
      await _tallSurface(tester);
      final fake = _funded()..swapQuoteResult = _intoZecQuote();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _intoZecToReview(tester, amount: '0,5');
      final exact = fake.lastSwapQuoteRequest!.exact as ExactSide_In;
      expect((exact.amount as SwapAmount_Foreign).amount, '0.5');
    },
  );

  testWidgets('S13: OutOfZec "0,5" quotes 0.5 ZEC (50,000,000 zat)', (
    tester,
  ) async {
    await _tallSurface(tester);
    final fake = _funded();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _selectSell(tester);
    await tester.enterText(_outAmountField, '0,5');
    await tester.pump();
    expect(tester.widget<TextField>(_outAmountField).controller!.text, '0.5');
  });
}
