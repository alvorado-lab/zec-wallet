import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet_ui/core/router/wallet_routes.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_controller.dart'
    show kSwapNetworkTimeout, swapControllerProvider;
import 'package:zec_wallet_ui/features/wallet/swap/swap_state.dart'
    show SwapExecuted, SwapFlowDirection, SwapFormState;
import 'package:zec_wallet_ui/features/wallet/swap/swap_enabled_provider.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_address_scanner.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_screen.dart';
import 'package:zec_wallet_ui/features/wallet/sync_status_presentation.dart'
    show walletCompactTimeFormat;
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/address_text.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

import '../../../support/a11y_activation.dart';

/// Swap-screen widget tests (D-2b-1). The §2.6 privacy disclosure (the de-shield
/// warning + the provider-sees list + the BLOCKING acknowledgment) and the §3.5
/// kill-aware tracking are the load-bearing assertions: a missing disclosure or a
/// silently-trackable killed swap would pass a happy-path review and mislead the
/// user about privacy or funds.
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(SwapScreen)));

/// Key-based finders for the IntoZec fields — reorder-proof (the form is asset →
/// amount → refund as; positional `.at(i)` would silently target the
/// wrong field if the order ever changes again).
final _intoAmountField = find.byKey(const Key('swap-into-amount'));
final _intoRefundField = find.byKey(const Key('swap-into-refund'));

/// Key-based finders for the OutOfZec fields — reorder-proof, symmetric with the
/// IntoZec ones. The form is target asset → amount → destination.
final _outAmountField = find.byKey(const Key('swap-out-amount'));
final _outDestinationField = find.byKey(const Key('swap-out-destination'));

/// Flip-able host kill-state, so a test can turn swap off mid-tracking (§3.5).
final _killHolder = StateProvider<bool>((_) => true);

Widget _harness({
  required FakeWalletSession session,
  bool nullSession = false,
  bool swapEnabled = true,
  bool flippable = false,
  bool? scannerSupported,
  AddressScanner? scanner,
  List<Override> extraOverrides = const [],
}) {
  return ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(nullSession ? null : session),
      if (flippable)
        swapEnabledProvider.overrideWith((ref) => ref.watch(_killHolder))
      else
        swapEnabledProvider.overrideWithValue(swapEnabled),
      screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
      // IZ-4 scan injection: drive the flow without a camera (the real
      // ReaderWidget never mounts). The test host is desktop, so the support
      // gate is false unless overridden.
      if (scannerSupported != null)
        addressScannerSupportedProvider.overrideWithValue(scannerSupported),
      if (scanner != null) addressScannerProvider.overrideWithValue(scanner),
      ...extraOverrides,
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

/// Like [_harness] but ROUTER-backed, for tests that exercise a Done tap:
/// `leaveToWalletRoot` needs a live GoRouter (`go(WalletRoutes.wallet)` on an
/// empty stack), which the plain-MaterialApp harness cannot serve.
Widget _routedHarness({required FakeWalletSession session}) {
  final router = GoRouter(
    initialLocation: '/swap-test',
    routes: [
      GoRoute(path: '/swap-test', builder: (_, _) => const SwapScreen()),
      GoRoute(
        path: WalletRoutes.wallet,
        builder: (_, _) =>
            const Scaffold(body: Text('wallet-root-placeholder')),
      ),
    ],
  );
  return ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(session),
      swapEnabledProvider.overrideWithValue(true),
      screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
    ],
    child: MaterialApp.router(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      routerConfig: router,
    ),
  );
}

/// Flip the form to OutOfZec (Sell ZEC) — the default landing is now IntoZec
/// (§3.3b L8), so the OutOfZec flow tests select it first.
Future<void> _selectSell(WidgetTester tester) async {
  await tester.tap(find.text(_l10n(tester).walletSwapDirectionSell));
  await tester.pumpAndSettle();
}

/// Pick an OutOfZec TARGET asset from the shared token picker (mirrors the IntoZec
/// `pickSource`). The form has no static default any more, so Get-quote stays
/// disabled until this runs.
Future<void> _pickTarget(
  WidgetTester tester, {
  String label = 'USDC on Ethereum',
}) async {
  await tester.tap(find.byKey(const Key('swap-target-asset-field')));
  await tester.pumpAndSettle();
  // The picker row shows the symbol + chain chip but keeps the full label as its
  // a11y semantics — tap by that (matches `pickSource`).
  await tester.tap(find.bySemanticsLabel(label));
  await tester.pumpAndSettle();
}

Future<void> _toReview(WidgetTester tester) async {
  // The review screen is a tall ListView (de-shield + numbers + disclosure + ack
  // + Start); enlarge the surface so its bottom (ack + Start) actually lays out
  // — a ListView lazily builds, so off-screen widgets aren't in the tree to tap.
  await tester.binding.setSurfaceSize(const Size(1200, 2600));
  addTearDown(() => tester.binding.setSurfaceSize(null));
  await _selectSell(tester);
  final l10n = _l10n(tester);
  // OutOfZec: target asset → amount → destination. Pick the asset first
  // (Get-quote is gated on it now — no static default), then fill the keyed
  // fields (reorder-proof, like the IntoZec finders).
  await _pickTarget(tester);
  await tester.enterText(_outAmountField, '0.5');
  await tester.enterText(_outDestinationField, '0xdest');
  await tester.tap(find.text(l10n.walletSwapQuoteButton));
  await tester.pumpAndSettle();
  // Tick the payout-address verification (N03) whenever the review is up, so
  // the tests below exercise the §2.6 privacy ack exactly as before. The
  // payout gate itself is pinned by its own tests.
  final payoutAck = find.text(l10n.walletSwapPayoutVerifyAck);
  if (payoutAck.evaluate().isNotEmpty) {
    await tester.tap(payoutAck);
    await tester.pumpAndSettle();
  }
}

/// The §2.6 privacy acknowledgment's tile (the review also carries the
/// address-verification tile, so a bare `byType` would be ambiguous).
Finder _privacyAck(WidgetTester tester) =>
    find.widgetWithText(CheckboxListTile, _l10n(tester).walletSwapAckLabel);

/// form → quote → ack → execute → tracking.
Future<void> _executeToTracking(WidgetTester tester) async {
  await _toReview(tester);
  final l10n = _l10n(tester);
  await tester.tap(find.text(l10n.walletSwapAckLabel));
  await tester.pumpAndSettle();
  await tester.tap(find.text(l10n.walletSwapConfirmButton));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('default landing is the IntoZec (buy ZEC) form', (tester) async {
    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    // The direction toggle, with the IntoZec fields in reading order: the
    // source-asset picker, the foreign amount, the refund address, the slippage
    // control, and the quote action.
    expect(find.text(l10n.walletSwapDirectionBuy), findsOneWidget);
    expect(find.text(l10n.walletSwapDirectionSell), findsOneWidget);
    expect(find.text(l10n.walletSwapRefundLabel), findsWidgets);
    expect(find.text(l10n.walletSwapSourceAssetLabel), findsWidgets);
    expect(find.text(l10n.walletSwapSlippageLabel), findsOneWidget);
    expect(find.text(l10n.walletSwapQuoteButton), findsOneWidget);

    // Get-quote is DISABLED until a source asset is picked (the cue is shown).
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, l10n.walletSwapQuoteButton),
          )
          .onPressed,
      isNull,
    );
  });

  testWidgets('flipping to Sell shows the OutOfZec target picker + destination, '
      'Get-quote gated until a target asset is picked', (tester) async {
    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();
    await _selectSell(tester);
    final l10n = _l10n(tester);
    expect(
      find.text(l10n.walletSwapAssetLabel),
      findsOneWidget,
    ); // "Receive asset"
    // No static default any more — the picker shows the "select an asset" cue.
    expect(find.text(l10n.walletSwapTargetAssetHint), findsWidgets);
    expect(find.text(l10n.walletSwapAmountLabel), findsOneWidget);
    expect(find.text(l10n.walletSwapDestinationLabel), findsOneWidget);
    // Get-quote is DISABLED until a target asset is picked (symmetric with the
    // IntoZec source gate — a quote is never attempted with no asset).
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, l10n.walletSwapQuoteButton),
          )
          .onPressed,
      isNull,
    );
  });

  testWidgets('picking a target asset enables Get-quote and shows the label', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();
    await _selectSell(tester);
    await _pickTarget(tester);
    final l10n = _l10n(tester);
    // The picked asset's label now shows in the target field (the cue is gone).
    expect(find.text('USDC on Ethereum'), findsWidgets);
    expect(find.text(l10n.walletSwapTargetAssetHint), findsNothing);
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, l10n.walletSwapQuoteButton),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('the token picker sheet heading is direction-aware '
      '(swap-from for Buy, receive for Sell)', (tester) async {
    // Device-found: the SHARED picker sheet must not say "Choose an asset
    // to swap from" on the Sell form — there the asset is what you RECEIVE. Each
    // direction supplies its own framing through `showSwapTokenPicker(title:)`.
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);

    // IntoZec (Buy, the default landing): "…to swap from".
    await tester.tap(find.byKey(const Key('swap-source-asset-field')));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSwapPickerTitle), findsOneWidget);
    expect(find.text(l10n.walletSwapPickerTitleReceive), findsNothing);
    // Dismiss the sheet (picking a token pops it).
    await tester.tap(find.bySemanticsLabel('USDC on Ethereum'));
    await tester.pumpAndSettle();

    // OutOfZec (Sell): "…to receive" (NOT "swap from").
    await _selectSell(tester);
    await tester.tap(find.byKey(const Key('swap-target-asset-field')));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSwapPickerTitleReceive), findsOneWidget);
    expect(find.text(l10n.walletSwapPickerTitle), findsNothing);
  });

  // The compose path, driven the way a screen reader drives it. The rows above
  // reach the picker through `tester.tap` — a POINTER event that hits the
  // InkWell/ListTile beneath and never consults the semantics tree, including
  // `_pickTarget`'s `tester.tap(find.bySemanticsLabel(…))`, which finds the
  // node's WIDGET and then taps its centre. All green while
  // `excludeSemantics: true` dropped BOTH nodes' tap actions, which made the
  // asset picker unreachable and every row of it unpickable, and so made the
  // swap uncomposable without sighted pointing.
  // Contract: `test/support/a11y_activation.dart`.
  testWidgets(
    'the asset field and the token rows are ACTIVATABLE by a screen reader — '
    'the semantics actions alone open the picker and pick an asset',
    (tester) async {
      final handle = tester.ensureSemantics();
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));

      await tester.pumpWidget(_harness(session: _funded()));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      // 1. The field: label is the empty-state cue while nothing is picked.
      expectActivatable(
        tester,
        find.semantics.byLabel(l10n.walletSwapSourceAssetHint),
        reason: 'the asset picker is the first step of composing a swap',
      );
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSwapPickerTitle), findsOneWidget);

      // 2. A row of the picker it just opened.
      expectActivatable(
        tester,
        find.semantics.byLabel('USDC on Ethereum'),
        reason:
            'a picker whose rows announce and cannot be chosen is a dead end',
      );
      await tester.pumpAndSettle();

      // The sheet popped AND the field took the asset — the actions are wired
      // to the same callbacks the finger runs, not merely present.
      expect(find.text(l10n.walletSwapPickerTitle), findsNothing);
      expect(
        find.semantics.byLabel(l10n.walletSwapSourceAssetHint),
        findsNothing,
        reason: 'the empty-state cue is replaced by the picked asset',
      );
      expect(find.text('USDC on Ethereum'), findsWidgets);
      handle.dispose();
    },
  );

  testWidgets('swap off → the whole screen is the honest unavailable state', (
    tester,
  ) async {
    await tester.pumpWidget(_harness(session: _funded(), swapEnabled: false));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);
    expect(find.text(l10n.walletSwapUnavailableOff), findsOneWidget);
    // No form surface at all (§3.5 UI isolation).
    expect(find.text(l10n.walletSwapQuoteButton), findsNothing);
  });

  testWidgets('no wallet session → honest unavailable, not a stale form', (
    tester,
  ) async {
    await tester.pumpWidget(_harness(session: _funded(), nullSession: true));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);
    expect(find.text(l10n.walletSwapUnavailableWallet), findsOneWidget);
  });

  testWidgets(
    'watch-only + deep link → the PERMANENT view-only unavailable copy, '
    'never the transient "right now" (#397 §3.7 D3, S234 review)',
    (tester) async {
      // A host-mounted route can land a watch-only user here even though the
      // chrome hides the entry button. Swap can never enable on a watch-only
      // wallet, so the unavailable state is permanent — the copy must not
      // imply it will come back.
      await tester.pumpWidget(
        _harness(
          session: _funded(),
          swapEnabled: false,
          extraOverrides: [isWatchOnlyProvider.overrideWithValue(true)],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapUnavailableWatchOnly), findsOneWidget);
      expect(find.text(l10n.walletSwapUnavailableOff), findsNothing);
      expect(find.text(l10n.walletSwapQuoteButton), findsNothing);
    },
  );

  testWidgets(
    'review renders the de-shield warning + the provider disclosure',
    (tester) async {
      final fake = _funded();
      fake.swapQuoteResult = swapQuoteFixture(); // deshields, sees amounts+dest
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _toReview(tester);
      final l10n = _l10n(tester);

      // The privacy centerpiece: the de-shield warning + every disclosed item.
      expect(find.text(l10n.walletSwapDeshieldTitle), findsOneWidget);
      expect(find.text(l10n.walletSwapDiscloseTitle), findsOneWidget);
      expect(find.text(l10n.walletSwapDiscloseAmounts), findsOneWidget);
      expect(find.text(l10n.walletSwapDiscloseDestination), findsOneWidget);
      // The asset label is echoed back so the user verifies what they receive.
      expect(find.textContaining('USDC on Ethereum'), findsWidgets);
    },
  );

  testWidgets('Start swap is BLOCKED until the disclosure is acknowledged', (
    tester,
  ) async {
    final fake = _funded();
    fake.swapQuoteResult = swapQuoteFixture();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);

    FilledButton startButton() => tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
    );

    // Disabled before acknowledgment (§2.6 disclosures-as-blocking-UX).
    expect(startButton().onPressed, isNull);
    expect(fake.swapExecuteCount, 0);

    // Acknowledge → enabled.
    await tester.tap(find.text(l10n.walletSwapAckLabel));
    await tester.pumpAndSettle();
    expect(startButton().onPressed, isNotNull);
  });

  testWidgets('the OutOfZec review shows the payout address the quote was '
      'requested for (the 2026-10-06 follow-up review, N03)', (tester) async {
    final fake = _funded();
    fake.swapQuoteResult = swapQuoteFixture();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSwapReviewTitle), findsOneWidget);
    // The input form is gone, so a match below cannot be the typed field itself.
    expect(_outDestinationField, findsNothing);
    expect(find.text(l10n.walletSwapPayoutVerifyTitle), findsOneWidget);
    expect(
      find.text(groupAddress('0xdest')),
      findsOneWidget,
      reason: 'the user must see where the ZEC goes before confirming',
    );
  });

  testWidgets('OutOfZec Start stays disabled until the payout address is '
      'verified, even with the privacy ack (N03)', (tester) async {
    final fake = _funded()..swapQuoteResult = swapQuoteFixture();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester); // ticks the payout ack…
    final l10n = _l10n(tester);
    await tester.tap(find.text(l10n.walletSwapPayoutVerifyAck)); // …untick it
    await tester.pumpAndSettle();
    FilledButton start() => tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
    );
    await tester.tap(find.text(l10n.walletSwapAckLabel)); // privacy only
    await tester.pumpAndSettle();
    expect(start().onPressed, isNull);
    await tester.tap(find.text(l10n.walletSwapPayoutVerifyAck));
    await tester.pumpAndSettle();
    expect(start().onPressed, isNotNull);
  });

  testWidgets('tapping the checkbox control itself toggles ack ONCE (no '
      'double-toggle)', (tester) async {
    final fake = _funded();
    fake.swapQuoteResult = swapQuoteFixture();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);

    // Tap the Checkbox control directly (the real device tap target, not the
    // label) — the CheckboxListTile brick toggles exactly once, so Start enables.
    await tester.tap(
      find.descendant(of: _privacyAck(tester), matching: find.byType(Checkbox)),
    );
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('acknowledge → execute → live tracking shows the swap status', (
    tester,
  ) async {
    final fake = _funded();
    fake.swapQuoteResult = swapQuoteFixture();
    fake.swapExecuteResult = 'swap-1';
    // The status the fake replays on subscribe.
    fake.swapCurrent = const SwapStatus.pendingDeposit(expiresAt: 2000000000);
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);

    await tester.tap(find.text(l10n.walletSwapAckLabel));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSwapConfirmButton));
    await tester.pumpAndSettle();

    expect(fake.swapExecuteCount, 1);
    expect(fake.lastWatchedSwapId, 'swap-1');
    // The tracking view renders the live PendingDeposit status.
    expect(find.text(l10n.walletSwapStatusPendingTitle), findsOneWidget);
    // #366-e: the card names the deposit window it references — a LIVE window
    // renders the ends-at stamp, so a pending swap is never open-ended.
    final expected = l10n.walletSwapPendingWindowEndsAt(
      walletCompactTimeFormat(l10n.localeName).format(
        DateTime.fromMillisecondsSinceEpoch(2000000000 * 1000).toLocal(),
      ),
    );
    expect(find.text(expected), findsOneWidget);
  });

  testWidgets(
    '#366-e: a LAPSED deposit window renders the honest window-passed line, '
    'never an eternal "swap started"',
    (tester) async {
      final fake = _funded();
      fake.swapQuoteResult = swapQuoteFixture();
      fake.swapExecuteResult = 'swap-1';
      // A quote whose deposit window is already in the past (the dead-quote
      // shape that pre-#366 rendered as pending FOREVER).
      fake.swapCurrent = const SwapStatus.pendingDeposit(expiresAt: 1700000000);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _toReview(tester);
      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapConfirmButton));
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletSwapPendingWindowPassedOutOfZec),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    'a NON-terminal tracking card offers "Start another swap" back to the form '
    '(S192-b HIGH-1) — a never-terminal re-attach must never brick the feature',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-stuck-1'
        // A window-passed PendingDeposit: a real dead quote that stays
        // non-terminal (the core keeps polling a GC'd/dead order forever).
        ..swapCurrent = const SwapStatus.pendingDeposit(expiresAt: 1700000000);
      await tester.pumpWidget(_routedHarness(session: fake));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await _toReview(tester);
      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapConfirmButton));
      await tester.pumpAndSettle();
      // Tracking renders the (window-passed) pending card WITH the escape.
      expect(find.text(l10n.walletSwapStatusPendingTitle), findsOneWidget);
      expect(find.text(l10n.walletSwapStartAnother), findsOneWidget);

      await tester.tap(find.text(l10n.walletSwapStartAnother));
      await tester.pumpAndSettle();
      expect(
        container.read(swapControllerProvider),
        isA<SwapFormState>(),
        reason: 'the escape returns to the form; the swap keeps its home row',
      );
    },
  );

  testWidgets(
    'the tracking LOADING arm offers the escape too (S192-b HIGH-1) — a '
    're-attach whose first status never arrives is not a spinner-forever trap',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-loading-1'
        ..holdSwapSubscribe = true; // connected but silent → stays loading
      await tester.pumpWidget(_routedHarness(session: fake));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await _toReview(tester);
      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapConfirmButton));
      // The loading spinner animates forever (no status arrives), so pump
      // fixed frames — pumpAndSettle would never settle.
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));
      // The busy spinner is up (no status ever arrived) AND the escape is there.
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      expect(find.text(l10n.walletSwapStartAnother), findsOneWidget);
      // The cold-attach title is NEUTRAL — never the over-claiming "Swap
      // started"; we have not heard back on the status yet (#347).
      expect(find.text(l10n.walletSwapStatusCheckingTitle), findsOneWidget);
      expect(find.text(l10n.walletSwapStatusPendingTitle), findsNothing);
      await tester.tap(find.text(l10n.walletSwapStartAnother));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));
      expect(container.read(swapControllerProvider), isA<SwapFormState>());
      // Dispose the tree so the (now unmounted) spinner ticker is cancelled.
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'the tracking ERROR card\'s Done escapes to a clean form (arch M-A2) — '
    'the widened re-attach guard must never wedge a dead-end card forever',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-err-1'
        ..failSwapOnSubscribe = true;
      await tester.pumpWidget(_routedHarness(session: fake));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await _toReview(tester);
      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapConfirmButton));
      await tester.pumpAndSettle();
      // The establish-time failure renders the honest error card. Its button
      // says "Back to wallet", never "Done" (#364 F12 — nothing completed),
      // and its body must NOT read a swap-failure verdict into a TRACKING
      // failure (M1 — the failed-status body contradicted the title).
      expect(find.text(l10n.walletSwapTrackingError), findsOneWidget);
      expect(find.text(l10n.walletSwapTrackingErrorBody), findsOneWidget);
      expect(find.text(l10n.walletSwapStatusFailedBody), findsNothing);
      expect(find.text(l10n.walletSwapDone), findsNothing);

      await tester.tap(find.text(l10n.walletSwapBackToWallet));
      await tester.pumpAndSettle();
      expect(find.text('wallet-root-placeholder'), findsOneWidget);
      expect(
        container.read(swapControllerProvider),
        isA<SwapFormState>(),
        reason:
            'Done on the error card must clear the live state — otherwise '
            're-entry re-attaches the dead-end card for the process lifetime',
      );
    },
  );

  testWidgets('a host kill mid-tracking shows tracking-unavailable (§3.5)', (
    tester,
  ) async {
    final fake = _funded();
    fake.swapQuoteResult = swapQuoteFixture();
    fake.swapExecuteResult = 'swap-1';
    // Each ProviderScope has its own container, so `_killHolder` starts at its
    // default (ON) here.
    await tester.pumpWidget(_harness(session: fake, flippable: true));
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(SwapScreen)),
    );

    await _toReview(tester);
    final l10n = _l10n(tester);
    await tester.tap(find.text(l10n.walletSwapAckLabel));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSwapConfirmButton));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSwapStatusPendingTitle), findsOneWidget);

    // Host kills swap (manifest flip) while a swap is in flight: the tracking
    // view renders the funds-safety "tracking unavailable", NOT a generic "off".
    container.read(_killHolder.notifier).state = false;
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSwapTrackingUnavailableTitle), findsOneWidget);
    expect(find.text(l10n.walletSwapStatusPendingTitle), findsNothing);
    // Crucially NOT the generic pre-execute "off" copy.
    expect(find.text(l10n.walletSwapUnavailableOff), findsNothing);
    // #382 (the copy MED): this flow is OutOfZec, so the killed-tracking
    // body must say the refund comes back to THIS wallet — the retired generic
    // body sent the user to "the provider's side", who would truthfully answer
    // "we already sent it back" while the ZEC sat undetected here.
    expect(
      find.text(l10n.walletSwapTrackingUnavailableBodyOutOfZec),
      findsOneWidget,
      reason:
          'the OutOfZec kill body names this wallet as where a refund lands',
    );
    expect(
      find.text(l10n.walletSwapTrackingUnavailableBody),
      findsNothing,
      reason: '#382 retired the provider-side misdirect for both directions',
    );
  });

  testWidgets('an empty destination surfaces an honest inline fault', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final fake = _funded();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _selectSell(tester);
    await _pickTarget(tester);
    final l10n = _l10n(tester);

    await tester.enterText(_outAmountField, '0.5');
    await tester.tap(find.text(l10n.walletSwapQuoteButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.walletSwapFaultDestinationRequired), findsOneWidget);
    expect(fake.swapQuoteCount, 0);
  });

  testWidgets('an empty / zero OutOfZec amount surfaces an inline fault at the '
      'WIDGET level (not just the controller)', (tester) async {
    // The symmetric counterpart of the empty-destination widget test: a user
    // picks a target, fills the destination, but leaves the amount blank (or 0)
    // → Get-quote shows the inline amount fault (`_FormFault`) and dispatches NO
    // quote. The controller logic is unit-tested; this pins the SCREEN render of
    // the SwapAmountFault for the Sell form (the e2e testing host-VM journey gap).
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final fake = _funded();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _selectSell(tester);
    await _pickTarget(tester);
    final l10n = _l10n(tester);

    // A picked target + a destination, but a ZERO amount → the notPositive fault.
    await tester.enterText(_outDestinationField, '0xdest');
    await tester.enterText(_outAmountField, '0');
    await tester.tap(find.text(l10n.walletSwapQuoteButton));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSendFaultAmountNotPositive), findsOneWidget);
    expect(fake.swapQuoteCount, 0);

    // And a BLANK amount → the empty fault (still no bridge call).
    await tester.enterText(_outAmountField, '');
    await tester.tap(find.text(l10n.walletSwapQuoteButton));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSendFaultAmountEmpty), findsOneWidget);
    expect(fake.swapQuoteCount, 0);
  });

  // --- real-world-edge round (S84 re-review) ---------------------------------

  testWidgets(
    'executing refreshes the wallet snapshot (money moved at execute)',
    (tester) async {
      final fake = _funded(); // 5 ZEC available at mount
      fake.swapQuoteResult = swapQuoteFixture();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      // The wallet the execute-time refresh will re-read: a lower balance (the
      // deposit is queued at execute). Set BEFORE executing — since #381 the
      // read provider reloads EAGERLY at the invalidate (the fenced view keeps
      // it hot), so the fresh figure must already be behind the port when the
      // execute transition fires.
      fake.setSnapshot(
        walletStateFixture(
          balance: balanceFixture(spendableZat: 100000000, totalZat: 100000000),
        ),
      );
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapStatusPendingTitle), findsOneWidget);

      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      // The snapshot was invalidated at the execute transition, so the surface
      // reports the FRESH balance — never the stale pre-swap figure.
      await tester.pumpAndSettle();
      final snap = container.read(walletSnapshotProvider).value;
      expect(snap?.balance.spendableZat, 100000000);
    },
  );

  testWidgets('the disclosure renders EVERY provider-sees item, incl. a '
      'forward-compat unknown (never silently dropped)', (tester) async {
    final fake = _funded();
    fake.swapQuoteResult = swapQuoteFixture(
      providerSees: const [
        DisclosureItem.amounts,
        DisclosureItem.sourceAddress,
        DisclosureItem.ipUnlessTor,
        DisclosureItem.crossAssetLink,
        DisclosureItem.unknown,
      ],
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);
    expect(find.text(l10n.walletSwapDiscloseAmounts), findsOneWidget);
    expect(find.text(l10n.walletSwapDiscloseSource), findsOneWidget);
    expect(find.text(l10n.walletSwapDiscloseIp), findsOneWidget);
    expect(find.text(l10n.walletSwapDiscloseCrossLink), findsOneWidget);
    // The forward-compat arm renders a generic line — never dropped.
    expect(find.text(l10n.walletSwapDiscloseGeneric), findsOneWidget);
  });

  testWidgets(
    'the provider-legs-public line shows iff providerLegsTransparent',
    (tester) async {
      final on = _funded()
        ..swapQuoteResult = swapQuoteFixture(providerLegsTransparent: true);
      await tester.pumpWidget(_harness(session: on));
      await tester.pumpAndSettle();
      await _toReview(tester);
      var l10n = _l10n(tester);
      expect(
        find.text(l10n.walletSwapDiscloseProviderLegsPublic),
        findsOneWidget,
      );

      // Clean slate before the second mount (pumpWidget reuses the container +
      // screen State at the same position otherwise).
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      final off = _funded()
        ..swapQuoteResult = swapQuoteFixture(providerLegsTransparent: false);
      await tester.pumpWidget(_harness(session: off));
      await tester.pumpAndSettle();
      await _toReview(tester);
      l10n = _l10n(tester);
      expect(
        find.text(l10n.walletSwapDiscloseProviderLegsPublic),
        findsNothing,
      );
    },
  );

  testWidgets('a non-de-shielding quote suppresses the de-shield warning', (
    tester,
  ) async {
    final fake = _funded()
      ..swapQuoteResult = swapQuoteFixture(deshields: false);
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);
    expect(find.text(l10n.walletSwapDeshieldTitle), findsNothing);
    // The rest of the review still renders (the disclosure card + numbers).
    expect(find.text(l10n.walletSwapDiscloseTitle), findsOneWidget);
    expect(find.text(l10n.walletSwapReviewTitle), findsOneWidget);
  });

  testWidgets('the provider min-out is rendered verbatim (no reformat/round)', (
    tester,
  ) async {
    const odd = '49.499999999999999999';
    final fake = _funded()
      ..swapQuoteResult = swapQuoteFixture(minAmountOut: odd);
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    expect(find.textContaining(odd), findsOneWidget);
  });

  testWidgets('a stale quote at execute shows the honest re-quote fault', (
    tester,
  ) async {
    final fake = _funded()..swapQuoteResult = swapQuoteFixture();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);
    await tester.tap(find.text(l10n.walletSwapAckLabel));
    await tester.pumpAndSettle();
    // The quote expired while the review sat open: execute throws QuoteExpired.
    fake.swapExecuteThrows = const SwapApiError(
      code: 'RW-SWAP-003',
      message: 'static',
      kind: SwapErrorKind.quoteExpired(),
    );
    await tester.tap(find.text(l10n.walletSwapConfirmButton));
    await tester.pumpAndSettle();
    // Back on the form with the honest "get a fresh quote" message.
    expect(find.text(l10n.walletSwapFaultExpired), findsOneWidget);
    expect(find.text(l10n.walletSwapQuoteButton), findsOneWidget);
  });

  testWidgets('a structurally untrackable swap shows an honest card, not a '
      'spinner forever', (tester) async {
    final fake = _funded()..swapQuoteResult = swapQuoteFixture();
    // The status stream errors on subscribe (an establish-time typed failure —
    // the core never routes a transient fault here).
    fake.failSwapOnSubscribe = true;
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _executeToTracking(tester);
    final l10n = _l10n(tester);
    expect(find.text(l10n.walletSwapTrackingError), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsNothing);
  });

  testWidgets('the acknowledgment does NOT persist across a re-quote', (
    tester,
  ) async {
    final fake = _funded()..swapQuoteResult = swapQuoteFixture(id: 'q-1');
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);
    await tester.tap(find.text(l10n.walletSwapAckLabel)); // acknowledge
    await tester.pumpAndSettle();
    // Back to the form, then quote again → a fresh review must re-require the ack
    // (a stale ack on fresh numbers would defeat the §2.6 blocking-UX). The
    // re-quote registers a NEW provider order (#364 F10: the ack reset is
    // keyed by quote OBJECT IDENTITY — never the provider-controlled id; a
    // same-INSTANCE restore after a host-prompt denial keeps the ack, any
    // freshly-quoted review resets. See the recycled-id attack pin below).
    await tester.tap(find.text(l10n.walletSwapBackButton));
    await tester.pumpAndSettle();
    fake.swapQuoteResult = swapQuoteFixture(id: 'q-2');
    await tester.tap(find.text(l10n.walletSwapQuoteButton));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
          )
          .onPressed,
      isNull,
    );
    expect(tester.widget<CheckboxListTile>(_privacyAck(tester)).value, isFalse);
    // The payout-address ack (ticked by `_toReview` on the first review) is
    // reset too: a fresh quote re-requires verifying where the asset goes (N03).
    expect(
      tester
          .widget<CheckboxListTile>(
            find.widgetWithText(
              CheckboxListTile,
              l10n.walletSwapPayoutVerifyAck,
            ),
          )
          .value,
      isFalse,
    );
  });

  testWidgets('each tracking status renders its own honest card', (
    tester,
  ) async {
    Future<void> check(
      SwapStatus status,
      String Function(WalletLocalizations) title,
    ) async {
      // Clean slate: pumpWidget reuses the ProviderScope container + screen State
      // at the same tree position across iterations, so tear the prior tree down
      // first to get a fresh controller (resetToForm) per status.
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = status;
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(find.text(title(l10n)), findsOneWidget);
    }

    await check(
      const SwapStatus.underDeposited(
        received: '0.4',
        missing: '0.1',
        deadline: 2000000000,
      ),
      (l) => l.walletSwapStatusUnderTitle,
    );
    await check(
      const SwapStatus.depositDetected(),
      (l) => l.walletSwapStatusDetectedTitle,
    );
    await check(
      const SwapStatus.processing(),
      (l) => l.walletSwapStatusProcessingTitle,
    );
    await check(
      const SwapStatus.success(),
      (l) => l.walletSwapStatusSuccessTitle,
    );
    await check(
      const SwapStatus.refunded(),
      (l) => l.walletSwapStatusRefundedTitle,
    );
    await check(
      const SwapStatus.failed(code: SwapFailureCode.providerFailure),
      (l) => l.walletSwapStatusFailedTitle,
    );
    await check(
      const SwapStatus.unknown(),
      (l) => l.walletSwapStatusUnknownTitle,
    );
  });

  testWidgets(
    'under-deposited renders the received/missing amounts and its deadline '
    '(#367 — the DTO fields existed and were never rendered)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.underDeposited(
          received: '0.4',
          missing: '0.1',
          deadline: 2000000000,
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      // The OutOfZec arm keeps the passive body (a wallet-sent deposit can't
      // be topped up by the user) and gains the amounts + deadline detail.
      expect(find.text(l10n.walletSwapStatusUnderBody), findsOneWidget);
      expect(find.textContaining('0.4'), findsOneWidget);
      expect(find.textContaining('0.1'), findsOneWidget);
    },
  );

  testWidgets(
    'an OutOfZec refund renders the direction-aware body (#368: the refund '
    'address is watched — the copy promises it lands in balance after a sync)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.refunded();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(
        find.text(l10n.walletSwapStatusRefundedBodyOutOfZec),
        findsOneWidget,
      );
      expect(find.text(l10n.walletSwapStatusRefundedBody), findsNothing);
    },
  );

  testWidgets(
    'the synthesized not-found terminal renders its own non-alarming card '
    '(#367 poll policy — a GC\'d order, not a "failure")',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.failed(code: SwapFailureCode.notFound);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapStatusNotFoundTitle), findsOneWidget);
      expect(find.text(l10n.walletSwapStatusNotFoundBody), findsOneWidget);
      expect(
        find.text(l10n.walletSwapStatusFailedTitle),
        findsNothing,
        reason: '"Swap failed" would over-claim for a provider-GC\'d order',
      );
    },
  );

  testWidgets('the not-found card\'s Done leaves WITHOUT dismissing the still-'
      'unresolved record (#385 MED-1ux — not-found is a heuristic, never '
      'pinned; a silent dismiss voided the promised watch)', (tester) async {
    final fake = _funded()
      ..swapQuoteResult = swapQuoteFixture()
      ..swapExecuteResult = 'swap-1'
      ..swapCurrent = const SwapStatus.failed(code: SwapFailureCode.notFound);
    await tester.pumpWidget(_routedHarness(session: fake));
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(SwapScreen)),
    );
    await _executeToTracking(tester);
    final l10n = _l10n(tester);
    expect(find.text(l10n.walletSwapStatusNotFoundTitle), findsOneWidget);
    // Not-found is UNRESOLVED — its button is "Back to wallet" (#364 F12).
    await tester.tap(find.text(l10n.walletSwapBackToWallet));
    await tester.pumpAndSettle();
    expect(find.text('wallet-root-placeholder'), findsOneWidget);
    expect(
      fake.dismissSwapRecordCount,
      0,
      reason:
          'the record under a synthesized not-found is UNRESOLVED — Done '
          'must not void the watch; removal stays with the home row\'s '
          'disclosure dialog',
    );
    expect(
      container.read(swapControllerProvider),
      isA<SwapFormState>(),
      reason: 'Done still clears the live tracking state (the escape half)',
    );
  });

  testWidgets(
    're-attach after a synthesized not-found RE-POLLS the provider — the '
    'latched heuristic must not outlive the overdue row\'s "open it to '
    'check" promise (#385, S196-c H-2)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.failed(code: SwapFailureCode.notFound);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapStatusNotFoundTitle), findsOneWidget);
      // Leave tracking (the Done shape) — the status element stays latched.
      container.read(swapControllerProvider.notifier).startNewSwap();
      await tester.pumpAndSettle();
      // The provider later KNOWS the real terminal; the user re-attaches from
      // the overdue home row. Without attachTo's not-found invalidation the
      // latched element re-renders the stale heuristic with zero provider
      // traffic for the rest of the process run.
      fake.swapCurrent = const SwapStatus.success();
      container
          .read(swapControllerProvider.notifier)
          .attachTo(swapId: 'swap-1', direction: SwapFlowDirection.outOfZec);
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletSwapStatusSuccessTitle),
        findsOneWidget,
        reason: 're-attach re-polled and rendered the REAL terminal',
      );
      expect(find.text(l10n.walletSwapStatusNotFoundTitle), findsNothing);
    },
  );

  testWidgets(
    'a SAME-ID re-attach after a synthesized not-found re-polls too — the '
    'back-nav exit keeps the live state, so the common home-row tap is a '
    'same-id attach and must not die on the idempotent-tap early return '
    '(#386, S197-b converged HIGH — guard order)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.failed(code: SwapFailureCode.notFound);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapStatusNotFoundTitle), findsOneWidget);
      // The user backs out of the card WITHOUT Done (PopScope only wraps
      // SwapExecuting) — the controller deliberately KEEPS the live
      // SwapExecuted('swap-1'). No startNewSwap here: that is the #385 test
      // above; this one pins the path it missed.
      fake.swapCurrent = const SwapStatus.success();
      container
          .read(swapControllerProvider.notifier)
          .attachTo(swapId: 'swap-1', direction: SwapFlowDirection.outOfZec);
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletSwapStatusSuccessTitle),
        findsOneWidget,
        reason:
            'the same-id re-attach must re-poll — pre-#386 the invalidate '
            'sat BELOW the same-id early return and never ran here',
      );
      expect(find.text(l10n.walletSwapStatusNotFoundTitle), findsNothing);
    },
  );

  testWidgets(
    'a PINNED record\'s re-attach does NOT re-poll — the pin outranks the '
    'heuristic and must keep rendering instantly, never displaced by a '
    '~75 s+ consecutive-404 re-earn busy (#386, S198 review MED)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.failed(code: SwapFailureCode.notFound)
        ..inFlightSwapsResult = [
          swapRecordFixture(
            id: 'swap-1',
            direction: SwapRecordDirection.outOfZec,
            outcome: SwapOutcome.refunded,
          ),
        ];
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapStatusRefundedTitle), findsOneWidget);
      final subscribesBefore = fake.swapSubscribeCount;
      // Back-nav exit (live state kept), then the home-row tap — the same-id
      // re-attach. The stale-NotFound re-poll must SKIP a pinned record.
      container
          .read(swapControllerProvider.notifier)
          .attachTo(swapId: 'swap-1', direction: SwapFlowDirection.outOfZec);
      await tester.pumpAndSettle();
      expect(
        fake.swapSubscribeCount,
        subscribesBefore,
        reason:
            'no re-subscribe: the chain-observed pin is already the truth — '
            'invalidating would re-earn the 404 terminal over money truth '
            'the wallet holds',
      );
      expect(find.text(l10n.walletSwapStatusRefundedTitle), findsOneWidget);
      // And the entry-path hook (resetToForm's live no-op) skips it too.
      container.read(swapControllerProvider.notifier).resetToForm();
      await tester.pumpAndSettle();
      expect(fake.swapSubscribeCount, subscribesBefore);
      expect(find.text(l10n.walletSwapStatusRefundedTitle), findsOneWidget);
    },
  );

  testWidgets(
    'DIRECT screen entry with a live not-found-latched swap re-polls — the '
    'wallet screen\'s Swap button never calls attachTo, so the entry-time '
    'resetToForm no-op is the re-attach on that path (#386, S197-b '
    'converged HIGH — the uncovered second path)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.failed(code: SwapFailureCode.notFound);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapStatusNotFoundTitle), findsOneWidget);
      // The user leaves via back-nav (live state kept), the provider later
      // knows the real terminal, and re-enters via the HOME SWAP BUTTON —
      // context.push(WalletRoutes.swap) → initState → post-frame
      // resetToForm(). That entry hook is what must kick the stale latch.
      fake.swapCurrent = const SwapStatus.success();
      container.read(swapControllerProvider.notifier).resetToForm();
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletSwapStatusSuccessTitle),
        findsOneWidget,
        reason:
            'the entry-time live no-op must re-poll a stale not-found — '
            'pre-#386 this path had NO invalidate at all (zero provider '
            'traffic for the process lifetime; desktop never self-heals)',
      );
      expect(find.text(l10n.walletSwapStatusNotFoundTitle), findsNothing);
      expect(
        container.read(swapControllerProvider),
        isA<SwapExecuted>(),
        reason: 'the entry hook must not wipe the live tracking state',
      );
    },
  );

  testWidgets(
    'a re-entry DURING an active re-poll neither restarts it nor flashes the '
    'busy card — the S199 retention keeps the honest NotFound rendered and '
    'the isLoading guard keeps the consecutive-404 run intact',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.failed(code: SwapFailureCode.notFound);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapStatusNotFoundTitle), findsOneWidget);
      // First re-entry kicks the re-poll; the fresh subscription is HELD
      // silent — the real-world GC'd order re-earning its ~75–135 s 404 run.
      fake.holdSwapSubscribe = true;
      container
          .read(swapControllerProvider.notifier)
          .attachTo(swapId: 'swap-1', direction: SwapFlowDirection.outOfZec);
      await tester.pumpAndSettle();
      final subscribesAfterKick = fake.swapSubscribeCount;
      expect(
        find.text(l10n.walletSwapStatusNotFoundTitle),
        findsOneWidget,
        reason:
            'retention: the honest latched card stays rendered through the '
            're-earn — never a "Swap started"-labeled busy over a swap the '
            'provider said does not exist',
      );
      // Re-entries WHILE the re-poll is in flight — BOTH paths (the home-row
      // same-id attach and the direct-entry resetToForm no-op). Pre-guard,
      // each would re-invalidate: the retained `.value` still reads
      // Failed(notFound), so only the isLoading guard stands between an
      // entry loop and an endlessly-reset 404 run (battery + provider
      // traffic on a dead order).
      container
          .read(swapControllerProvider.notifier)
          .attachTo(swapId: 'swap-1', direction: SwapFlowDirection.outOfZec);
      await tester.pumpAndSettle();
      container.read(swapControllerProvider.notifier).resetToForm();
      await tester.pumpAndSettle();
      expect(
        fake.swapSubscribeCount,
        subscribesAfterKick,
        reason: 're-entry during the re-earn must not restart the re-poll',
      );
    },
  );

  testWidgets(
    'a CONFIRMED terminal\'s Done does dismiss the record (the #367 belt — '
    'pinned here as the contrast to the not-found no-dismiss, #385)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.success();
      await tester.pumpWidget(_routedHarness(session: fake));
      await tester.pumpAndSettle();
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapStatusSuccessTitle), findsOneWidget);
      await tester.tap(find.text(l10n.walletSwapDone));
      await tester.pumpAndSettle();
      expect(
        fake.dismissSwapRecordCount,
        1,
        reason: 'a rendered (seen) terminal is finished business',
      );
      expect(fake.lastDismissedSwapId, 'swap-1');
    },
  );

  testWidgets(
    'a PINNED outcome outranks the synthesized not-found (#385 MED-2ux — '
    'the chain-observed Refunded pin lands without the card opening; the '
    'card must not contradict the home row)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.failed(code: SwapFailureCode.notFound)
        ..inFlightSwapsResult = [
          swapRecordFixture(
            id: 'swap-1',
            direction: SwapRecordDirection.outOfZec,
            outcome: SwapOutcome.refunded,
          ),
        ];
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _executeToTracking(tester);
      final l10n = _l10n(tester);
      expect(
        find.text(l10n.walletSwapStatusRefundedTitle),
        findsOneWidget,
        reason: 'the confirmed (pinned) terminal renders',
      );
      expect(
        find.text(l10n.walletSwapStatusRefundedBodyOutOfZec),
        findsOneWidget,
        reason: 'direction-honest body, same as a live Refunded observation',
      );
      expect(
        find.text(l10n.walletSwapStatusNotFoundTitle),
        findsNothing,
        reason:
            'rendering "not found — most likely expired" over a "Swap '
            'refunded." home row was the cross-surface contradiction',
      );
    },
  );

  testWidgets(
    'a RE-ATTACHED IntoZec pending swap gets the no-instructions copy — '
    'never "send them before the quote expires" (#367)',
    (tester) async {
      final fake = _funded()
        ..swapCurrent = const SwapStatus.pendingDeposit(expiresAt: 4100000000);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      container
          .read(swapControllerProvider.notifier)
          .attachTo(swapId: 'swap-1', direction: SwapFlowDirection.intoZec);
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(
        find.text(l10n.walletSwapStatusPendingBodyIntoZecReattached),
        findsOneWidget,
      );
      expect(
        find.text(l10n.walletSwapStatusPendingBodyIntoZec),
        findsNothing,
        reason: 'the issuing-run instructions are not followable post-attach',
      );
    },
  );

  testWidgets('the review disloses the network fee line for OutOfZec (#367 — '
      '"You send" is the deposit, not the whole debit)', (tester) async {
    final fake = _funded()..swapQuoteResult = swapQuoteFixture();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);
    expect(find.text(l10n.walletSwapNetworkFeeLabel), findsOneWidget);
    expect(find.text(l10n.walletSwapNetworkFeeValue), findsOneWidget);
  });

  testWidgets('the execute pop-guard blocks leaving mid-execute and says still '
      'working; the outcome screen is leavable (#367)', (tester) async {
    final fake = _funded()
      ..swapQuoteResult = swapQuoteFixture()
      ..swapExecuteGate = Completer<void>()
      ..swapCurrent = const SwapStatus.pendingDeposit(expiresAt: 4100000000);
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);
    await tester.tap(find.text(l10n.walletSwapAckLabel));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSwapConfirmButton));
    await tester.pump();
    expect(find.text(l10n.walletSwapExecuting), findsOneWidget);
    // A system back mid-execute: the PopScope refuses and explains.
    final navigator = tester.state<NavigatorState>(find.byType(Navigator));
    await navigator.maybePop();
    await tester.pump();
    expect(find.text(l10n.walletSwapExecuteStillWorking), findsOneWidget);
    expect(
      find.text(l10n.walletSwapExecuting),
      findsOneWidget,
      reason: 'still on the executing screen — the pop was blocked',
    );
    // The execute completes → tracking renders (pop-able again; the guard
    // was only for the bounded committing window).
    fake.swapExecuteGate!.complete();
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSwapStatusPendingTitle), findsWidgets);
  });

  testWidgets(
    'the busy banner is SUPPRESSED once the quote expired — the expired '
    'message (re-quote) wins over "try again" (review-fold pin)',
    (tester) async {
      // A quote already past its actionable deadline: the countdown flips
      // _quoteExpired on mount. The busy-restore is driven on the CONTROLLER
      // (the UI's Start is honestly disabled), modelling a busy that consumed
      // the tail of the quote window.
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture(expiresAt: 1700000000)
        ..swapExecuteThrows = SwapApiError(
          code: 'RW-SWAP-014',
          message: 'static',
          kind: const SwapErrorKind.swapStateBusy(),
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _toReview(tester);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapQuoteExpired), findsOneWidget);
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await container.read(swapControllerProvider.notifier).execute();
      await tester.pumpAndSettle();
      // The restored review re-detects expiry; the contradictory "busy — try
      // again" banner must not render next to it (Start is disabled; the true
      // instruction is the expired message's re-quote).
      expect(find.text(l10n.walletSwapQuoteExpired), findsOneWidget);
      expect(find.text(l10n.walletSwapFaultStoreBusyRetry), findsNothing);
    },
  );

  // --- IntoZec (buy ZEC) flow ------------------------------------------------

  SwapQuote intoZecQuote() => swapQuoteFixture(
    amountIn: '100',
    minAmountOut: '4.12248474',
    zecSideZat: 412248474,
    refundTo: 'bc1qrefundxyz',
    deshields: false,
    endsShielded: true,
    providerSees: const [
      DisclosureItem.amounts,
      DisclosureItem.crossAssetLink,
      DisclosureItem.sourceAddress,
      DisclosureItem.ipUnlessTor,
    ],
  );

  Future<void> pickSource(
    WidgetTester tester, {
    String label = 'USDC on Ethereum',
  }) async {
    await tester.tap(find.byKey(const Key('swap-source-asset-field')));
    await tester.pumpAndSettle();
    // The picker row now shows the symbol + a chain chip (not "SYMBOL on CHAIN"
    // inline), but keeps the full label as its a11y semantics — tap by that.
    await tester.tap(find.bySemanticsLabel(label));
    await tester.pumpAndSettle();
  }

  Future<void> intoZecToReview(WidgetTester tester) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await pickSource(tester);
    // IntoZec field order: asset → amount → refund. Find by key, not index.
    await tester.enterText(_intoAmountField, '100');
    await tester.enterText(_intoRefundField, 'bc1qrefundxyz');
    await tester.tap(find.text(_l10n(tester).walletSwapQuoteButton));
    await tester.pumpAndSettle();
  }

  testWidgets('picking a source asset enables Get-quote and shows the label', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();
    await pickSource(tester);
    final l10n = _l10n(tester);
    expect(find.text('USDC on Ethereum'), findsWidgets);
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, l10n.walletSwapQuoteButton),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('IntoZec field order is asset → amount → refund (S100 founder UX)', (
    tester,
  ) async {
    // A refund address belongs to the SOURCE chain, so the asset that determines
    // that chain comes FIRST, then the amount, then the refund LAST. Pin the
    // vertical reading order (by key + position, not index) so a future reshuffle
    // can't silently regress the maintainer-directed flow.
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();

    final assetY = tester
        .getTopLeft(find.byKey(const Key('swap-source-asset-field')))
        .dy;
    final amountY = tester.getTopLeft(_intoAmountField).dy;
    final refundY = tester.getTopLeft(_intoRefundField).dy;
    expect(assetY, lessThan(amountY), reason: 'asset picker is first');
    expect(
      amountY,
      lessThan(refundY),
      reason:
          'amount before refund (refund is filled last, once the chain is known)',
    );
  });

  testWidgets('IntoZec review shows the positive shielded line, NOT de-shield', (
    tester,
  ) async {
    final fake = _funded()..swapQuoteResult = intoZecQuote();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await intoZecToReview(tester);
    final l10n = _l10n(tester);
    // Positive end-state card, NOT the orange OutOfZec de-shield warning.
    expect(find.text(l10n.walletSwapIntoZecShieldTitle), findsOneWidget);
    expect(find.text(l10n.walletSwapDeshieldTitle), findsNothing);
    // The provider-sees disclosure includes the source (refund) address.
    expect(find.text(l10n.walletSwapDiscloseSource), findsOneWidget);
    // The refund verification step is present with the echoed address chunked.
    expect(find.text(l10n.walletSwapRefundVerifyTitle), findsOneWidget);
  });

  testWidgets('IntoZec Start needs BOTH the refund verification and the ack', (
    tester,
  ) async {
    final fake = _funded()..swapQuoteResult = intoZecQuote();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await intoZecToReview(tester);
    final l10n = _l10n(tester);

    FilledButton start() => tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
    );

    expect(start().onPressed, isNull); // both unchecked
    await tester.tap(find.text(l10n.walletSwapAckLabel)); // privacy only
    await tester.pumpAndSettle();
    expect(start().onPressed, isNull); // still blocked — refund not verified
    await tester.tap(find.text(l10n.walletSwapRefundVerifyAck));
    await tester.pumpAndSettle();
    expect(start().onPressed, isNotNull); // both checked → enabled
  });

  testWidgets('a review with no address to verify never enables Start, '
      'whatever is ticked', (tester) async {
    final quote = intoZecQuote();
    final fake = _funded()
      ..swapQuoteResult = swapQuoteFixture(
        amountIn: quote.amountIn,
        minAmountOut: quote.minAmountOut,
        zecSideZat: quote.zecSideZat,
        deshields: false,
        endsShielded: true,
      );
    expect(fake.swapQuoteResult!.refundTo, isNull);
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await intoZecToReview(tester);
    final l10n = _l10n(tester);
    FilledButton start() => tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
    );
    await tester.tap(find.text(l10n.walletSwapAckLabel));
    await tester.tap(find.text(l10n.walletSwapRefundVerifyAck));
    await tester.pumpAndSettle();
    expect(start().onPressed, isNull, reason: 'an ack against a dash');
  });

  testWidgets(
    'a host kill mid-AwaitingDeposit KEEPS the deposit screen (#366-d) — '
    'never the generic "off" that strands the user mid-deposit',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = intoZecQuote()
        ..swapExecuteResult = 'swap-iz-kill';
      await tester.pumpWidget(_harness(session: fake, flippable: true));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      await intoZecToReview(tester);
      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapRefundVerifyAck));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapConfirmButton));
      // The deposit screen runs a 1s countdown — pump frames, never settle.
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));
      expect(find.text(l10n.walletSwapDepositTitle), findsOneWidget);

      // The kill flips mid-deposit: pre-#366 this arm dropped the whole screen
      // to the generic "swap off", losing the address/amount/memo mid-money.
      container.read(_killHolder.notifier).state = false;
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));
      expect(
        find.text(l10n.walletSwapDepositTitle),
        findsOneWidget,
        reason: 'the committed deposit screen is spared from the kill gate',
      );
      expect(find.text(l10n.walletSwapUnavailableOff), findsNothing);

      // Dispose the tree so the countdown timer is cancelled.
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('IntoZec execute reaches the D7 deposit screen', (tester) async {
    final fake = _funded()
      ..swapQuoteResult = intoZecQuote()
      ..swapExecuteResult = 'swap-iz-1';
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await intoZecToReview(tester);
    final l10n = _l10n(tester);
    await tester.tap(find.text(l10n.walletSwapAckLabel));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSwapRefundVerifyAck));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.walletSwapConfirmButton));
    // The deposit screen runs a 1s countdown timer — pump frames, never settle.
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(fake.swapExecuteCount, 1);
    // The D7 "send your payment" screen with the deposit address + send affordance.
    expect(find.text(l10n.walletSwapDepositTitle), findsOneWidget);
    expect(find.textContaining('tdeposit'), findsWidgets);
    expect(find.text(l10n.walletSwapDepositSent), findsOneWidget);

    // Dispose the tree so the countdown timer is cancelled (no pending-timer fail).
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('refund_address_paste_is_always_available', (tester) async {
    // Paste/type is the ALWAYS-available refund entry path (§3.3b D6/L8) — the QR
    // scan is IZ-4, ADDITIVE (a blind user can't aim a camera; desktop has no
    // camera). The refund field is a plain editable TextField (which always
    // supports paste), and a typed value reaches the quote verbatim.
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final fake = _funded()..swapQuoteResult = intoZecQuote();
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await pickSource(tester);
    // IntoZec field order: asset → amount → refund (paste/type the refund).
    await tester.enterText(_intoAmountField, '100');
    await tester.enterText(_intoRefundField, 'bc1qtypedrefund');
    await tester.tap(find.text(_l10n(tester).walletSwapQuoteButton));
    await tester.pumpAndSettle();
    expect(fake.lastSwapQuoteRequest, isNotNull);
    expect(fake.lastSwapQuoteRequest!.refundAddress, 'bc1qtypedrefund');
  });

  group('IZ-4 refund QR scan', () {
    // The scan affordance is gated on `addressScannerSupportedProvider` (mobile
    // only) and invokes the injected `addressScannerProvider`. Tests drive the
    // flow without a camera via `_harness(scannerSupported:, scanner:)` — the
    // real ReaderWidget never mounts. The test host is desktop, so the gate is
    // false unless overridden.

    testWidgets('refund_address_qr_scan_populates_the_field', (tester) async {
      // A scanned standard BIP-21 URI is URI-unwrapped to a bare address, fills
      // the field, and reaches the quote request verbatim (money-correctness).
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final fake = _funded()..swapQuoteResult = intoZecQuote();
      await tester.pumpWidget(
        _harness(
          session: fake,
          scannerSupported: true,
          scanner: (_) async => 'bitcoin:bc1qscanme?amount=0.25&label=wallet',
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.tap(find.byTooltip(l10n.walletSwapRefundScanTooltip));
      await tester.pumpAndSettle();

      // The field shows the bare address (scheme + query stripped).
      expect(
        tester.widget<TextField>(_intoRefundField).controller!.text,
        'bc1qscanme',
      );

      // …and it flows through quote → request unchanged.
      await pickSource(tester);
      await tester.enterText(_intoAmountField, '100');
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      expect(fake.lastSwapQuoteRequest!.refundAddress, 'bc1qscanme');
    });

    testWidgets('scan offered when the platform supports a camera', (
      tester,
    ) async {
      await tester.pumpWidget(
        _harness(
          session: _funded(),
          scannerSupported: true,
          scanner: (_) async => null,
        ),
      );
      await tester.pumpAndSettle();
      expect(
        find.byTooltip(_l10n(tester).walletSwapRefundScanTooltip),
        findsOneWidget,
      );
    });

    testWidgets('scan hidden on a camera-less platform (desktop paste-only)', (
      tester,
    ) async {
      // No override → the real `addressScannerSupported` (false on the desktop
      // test host). The refund field is still present and type/paste-able.
      await tester.pumpWidget(_harness(session: _funded()));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(find.byTooltip(l10n.walletSwapRefundScanTooltip), findsNothing);
      expect(find.text(l10n.walletSwapRefundLabel), findsWidgets);
    });

    testWidgets('cancelling the scan leaves the field untouched', (
      tester,
    ) async {
      await tester.pumpWidget(
        _harness(
          session: _funded(),
          scannerSupported: true,
          scanner: (_) async => null, // user backs out / no result
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await tester.enterText(_intoRefundField, 'bc1qpretyped');
      await tester.tap(find.byTooltip(l10n.walletSwapRefundScanTooltip));
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(_intoRefundField).controller!.text,
        'bc1qpretyped',
      );
    });

    // --- real-world-edge round (S84) -----------------------------------------

    testWidgets('a successful scan OVERWRITES an already-typed refund value '
        '(intended — the scan is the user\'s latest deliberate action)', (
      tester,
    ) async {
      // A user types a guess, then scans the real QR: the scan wins (a stale
      // typed prefix must not corrupt the scanned address). Documents the
      // overwrite as the intended behavior, not append/ignore.
      await tester.pumpWidget(
        _harness(
          session: _funded(),
          scannerSupported: true,
          scanner: (_) async => 'bitcoin:bc1qscanned?amount=0.1',
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      await tester.enterText(_intoRefundField, 'bc1qoldtyped');
      await tester.tap(find.byTooltip(l10n.walletSwapRefundScanTooltip));
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(_intoRefundField).controller!.text,
        'bc1qscanned',
      );
    });

    testWidgets('a scan yielding an EMPTY address (bitcoin:) leaves the field '
        'empty and the quote is blocked by the non-empty guard', (
      tester,
    ) async {
      // `bitcoin:` normalizes to '' → the field is cleared → the IntoZec quote
      // guard rejects an empty refund with refundAddressRequired and NEVER calls
      // swapQuote. The form must not become quotable on an empty refund.
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final fake = _funded()..swapQuoteResult = intoZecQuote();
      await tester.pumpWidget(
        _harness(
          session: fake,
          scannerSupported: true,
          scanner: (_) async => 'bitcoin:', // scheme-only → empty after unwrap
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      // Pre-fill a value, then scan a scheme-only QR: the field is cleared.
      await tester.enterText(_intoRefundField, 'bc1qstale');
      await tester.tap(find.byTooltip(l10n.walletSwapRefundScanTooltip));
      await tester.pumpAndSettle();
      expect(tester.widget<TextField>(_intoRefundField).controller!.text, '');

      // With a source asset + amount but an empty refund, Get-quote surfaces the
      // honest "refund required" fault and dispatches NO quote (the SDK is never
      // handed an empty refund).
      await pickSource(tester);
      await tester.enterText(_intoAmountField, '100');
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      expect(
        find.text(l10n.walletSwapFaultRefundAddressRequired),
        findsOneWidget,
      );
      expect(fake.swapQuoteCount, 0);
    });

    testWidgets('a scanned address flows UNCHANGED to the quote request AND is '
        'rendered at the review refund-verification echo (the char-by-char '
        'step)', (tester) async {
      // The whole point of the refund verification (§3.3b D6): the user checks
      // the SAME address the SDK got. (1) The scanned-then-unwrapped value must
      // reach the request verbatim — the money-correctness assertion. (2) The
      // provider echoes that address back as `refundTo`; the review renders it
      // chunked for char-by-char checking. The fake's `refundTo` is the echo, so
      // we pin the fixture's echo to the scanned bare address and assert both.
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final fake = _funded()
        // Same shape as intoZecQuote() but with the provider echoing the
        // SCANNED bare address back (the refund-verification render source).
        ..swapQuoteResult = swapQuoteFixture(
          amountIn: '100',
          minAmountOut: '4.12248474',
          zecSideZat: 412248474,
          refundTo: 'bc1qscanecho',
          deshields: false,
          endsShielded: true,
          providerSees: const [
            DisclosureItem.amounts,
            DisclosureItem.crossAssetLink,
            DisclosureItem.sourceAddress,
            DisclosureItem.ipUnlessTor,
          ],
        );
      await tester.pumpWidget(
        _harness(
          session: fake,
          scannerSupported: true,
          scanner: (_) async => 'bitcoin:bc1qscanecho?amount=0.1&label=wallet',
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      await tester.tap(find.byTooltip(l10n.walletSwapRefundScanTooltip));
      await tester.pumpAndSettle();
      await pickSource(tester);
      await tester.enterText(_intoAmountField, '100');
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();

      // (1) The scanned value reached the request verbatim (bare address).
      expect(fake.lastSwapQuoteRequest!.refundAddress, 'bc1qscanecho');
      // (2) The review's refund-verification echo renders it (chunked 4-char):
      // 'bc1qscanecho' (11 chars) -> 'bc1q scan echo'.
      expect(find.text(l10n.walletSwapRefundVerifyTitle), findsOneWidget);
      expect(find.text('bc1q scan echo'), findsOneWidget);
    });
  });

  group('OutOfZec destination QR scan (shared scanner brick)', () {
    // The destination scan reuses the SAME address-scanner brick as the IntoZec
    // refund scan (the honest rename), but fills the destination field and flows
    // to the quote request's `destination` — a DIFFERENT money field than refund.
    // ADDITIVE: paste/type is always available; desktop/web is paste-only (the
    // support gate is false off-camera). The unwrap itself is exhaustively tested
    // in `swap_scanned_address_test.dart` (the shared normalizer); these pin the
    // DESTINATION wiring + money-correctness.

    testWidgets('destination_qr_scan_populates_the_field_and_reaches_the_quote', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final fake = _funded()..swapQuoteResult = swapQuoteFixture();
      await tester.pumpWidget(
        _harness(
          session: fake,
          scannerSupported: true,
          // A standard EIP-681 wallet QR — URI-unwrapped to the bare 0x address.
          scanner: (_) async => 'ethereum:0xDEST123abc?value=1e18',
        ),
      );
      await tester.pumpAndSettle();
      await _selectSell(tester);
      await _pickTarget(tester);
      final l10n = _l10n(tester);

      await tester.tap(find.byTooltip(l10n.walletSwapDestinationScanTooltip));
      await tester.pumpAndSettle();
      // The field shows the bare address (scheme + query stripped).
      expect(
        tester.widget<TextField>(_outDestinationField).controller!.text,
        '0xDEST123abc',
      );

      // …and the SAME value reaches the quote request's destination verbatim
      // (money-correctness: the SDK gets exactly what the user saw).
      await tester.enterText(_outAmountField, '0.5');
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      expect(fake.lastSwapQuoteRequest!.destination, '0xDEST123abc');
    });

    testWidgets(
      'destination scan offered when the platform supports a camera',
      (tester) async {
        await tester.binding.setSurfaceSize(const Size(1200, 2600));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        await tester.pumpWidget(
          _harness(
            session: _funded(),
            scannerSupported: true,
            scanner: (_) async => null,
          ),
        );
        await tester.pumpAndSettle();
        await _selectSell(tester);
        expect(
          find.byTooltip(_l10n(tester).walletSwapDestinationScanTooltip),
          findsOneWidget,
        );
      },
    );

    testWidgets(
      'destination scan hidden on a camera-less platform (paste-only)',
      (tester) async {
        // No override → the real `addressScannerSupported` (false on the desktop
        // test host). The destination field is still present + type/paste-able.
        await tester.pumpWidget(_harness(session: _funded()));
        await tester.pumpAndSettle();
        await _selectSell(tester);
        expect(
          find.byTooltip(_l10n(tester).walletSwapDestinationScanTooltip),
          findsNothing,
        );
        expect(_outDestinationField, findsOneWidget);
      },
    );

    testWidgets(
      'a successful destination scan OVERWRITES an already-typed value '
      '(the scan is the user\'s latest deliberate action)',
      (tester) async {
        await tester.binding.setSurfaceSize(const Size(1200, 2600));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        await tester.pumpWidget(
          _harness(
            session: _funded(),
            scannerSupported: true,
            scanner: (_) async => 'ethereum:0xSCANNED',
          ),
        );
        await tester.pumpAndSettle();
        await _selectSell(tester);
        await _pickTarget(tester);
        final l10n = _l10n(tester);
        await tester.enterText(_outDestinationField, '0xoldtyped');
        await tester.tap(find.byTooltip(l10n.walletSwapDestinationScanTooltip));
        await tester.pumpAndSettle();
        expect(
          tester.widget<TextField>(_outDestinationField).controller!.text,
          '0xSCANNED',
        );
      },
    );

    testWidgets('cancelling the destination scan leaves the field untouched', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        _harness(
          session: _funded(),
          scannerSupported: true,
          scanner: (_) async => null, // user backs out / no result
        ),
      );
      await tester.pumpAndSettle();
      await _selectSell(tester);
      await _pickTarget(tester);
      final l10n = _l10n(tester);
      await tester.enterText(_outDestinationField, '0xpretyped');
      await tester.tap(find.byTooltip(l10n.walletSwapDestinationScanTooltip));
      await tester.pumpAndSettle();
      // The null-result guard in `_scanAddressInto` leaves the typed value intact.
      expect(
        tester.widget<TextField>(_outDestinationField).controller!.text,
        '0xpretyped',
      );
    });

    testWidgets(
      'a destination scan yielding an EMPTY address (ethereum:) clears '
      'the field and the quote is blocked by the non-empty guard',
      (tester) async {
        // `ethereum:` normalizes to '' → the destination field is cleared → the
        // OutOfZec quote guard rejects an empty destination with destinationRequired
        // and NEVER calls swapQuote. The form must not become quotable on an empty
        // destination (the SDK is never handed one).
        await tester.binding.setSurfaceSize(const Size(1200, 2600));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        final fake = _funded()..swapQuoteResult = swapQuoteFixture();
        await tester.pumpWidget(
          _harness(
            session: fake,
            scannerSupported: true,
            scanner: (_) async =>
                'ethereum:', // scheme-only → empty after unwrap
          ),
        );
        await tester.pumpAndSettle();
        await _selectSell(tester);
        await _pickTarget(tester);
        final l10n = _l10n(tester);

        // Pre-fill, then scan a scheme-only QR: the field is cleared.
        await tester.enterText(_outDestinationField, '0xstale');
        await tester.tap(find.byTooltip(l10n.walletSwapDestinationScanTooltip));
        await tester.pumpAndSettle();
        expect(
          tester.widget<TextField>(_outDestinationField).controller!.text,
          '',
        );

        // With a target + amount but an empty destination, Get-quote surfaces the
        // honest "destination required" fault and dispatches NO quote.
        await tester.enterText(_outAmountField, '0.5');
        await tester.tap(find.text(l10n.walletSwapQuoteButton));
        await tester.pumpAndSettle();
        expect(
          find.text(l10n.walletSwapFaultDestinationRequired),
          findsOneWidget,
        );
        expect(fake.swapQuoteCount, 0);
      },
    );
  });

  group('IntoZec token picker states', () {
    Future<void> openPicker(WidgetTester tester, FakeWalletSession fake) async {
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('swap-source-asset-field')));
      await tester.pumpAndSettle();
    }

    testWidgets('a fresh list renders the tokens', (tester) async {
      await openPicker(tester, _funded());
      // Rows show the symbol + a chain chip now; assert by the row's a11y label
      // (still the full "SYMBOL on CHAIN").
      expect(find.bySemanticsLabel('USDC on Ethereum'), findsOneWidget);
      expect(find.bySemanticsLabel('BTC on Bitcoin'), findsOneWidget);
    });

    testWidgets('a stale list shows the "couldn\'t refresh" banner (L6)', (
      tester,
    ) async {
      final fake = _funded()
        ..swapListTokensResult = swapTokenListFixture(fresh: false);
      await openPicker(tester, fake);
      expect(find.text(_l10n(tester).walletSwapPickerStale), findsOneWidget);
      // The cached tokens are still usable.
      expect(find.bySemanticsLabel('USDC on Ethereum'), findsOneWidget);
    });

    testWidgets('an empty fresh list shows the honest no-assets state', (
      tester,
    ) async {
      final fake = _funded()
        ..swapListTokensResult = swapTokenListFixture(
          tokens: const [],
          fresh: true,
        );
      await openPicker(tester, fake);
      expect(find.text(_l10n(tester).walletSwapPickerEmpty), findsOneWidget);
    });

    testWidgets('a no-cache fetch error shows an error + retry', (
      tester,
    ) async {
      final fake = _funded()
        ..swapListTokensThrows = const SwapApiError(
          code: 'RW-SWAP-009',
          message: 'static',
          kind: SwapErrorKind.providerUnavailable(),
        );
      await openPicker(tester, fake);
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletSwapPickerError), findsOneWidget);
      expect(find.text(l10n.walletSwapPickerRetry), findsOneWidget);
    });

    testWidgets(
      'search filters the list, and a non-match shows the no-match state',
      (tester) async {
        await tester.binding.setSurfaceSize(const Size(1200, 2600));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        await openPicker(tester, _funded());
        // Both rows present before searching.
        expect(find.bySemanticsLabel('USDC on Ethereum'), findsOneWidget);
        expect(find.bySemanticsLabel('BTC on Bitcoin'), findsOneWidget);
        // Filter by symbol — only USDC survives.
        await tester.enterText(
          find.byKey(const Key('swap-token-search')),
          'usdc',
        );
        await tester.pumpAndSettle();
        expect(find.bySemanticsLabel('USDC on Ethereum'), findsOneWidget);
        expect(find.bySemanticsLabel('BTC on Bitcoin'), findsNothing);
        // A non-matching query → the honest no-match state (echoes the query).
        await tester.enterText(
          find.byKey(const Key('swap-token-search')),
          'zzzz',
        );
        await tester.pumpAndSettle();
        expect(find.bySemanticsLabel('USDC on Ethereum'), findsNothing);
        expect(find.textContaining('zzzz'), findsWidgets);
      },
    );
  });

  testWidgets('an expired quote disables Start swap + shows the expiry message', (
    tester,
  ) async {
    // A quote whose display deadline is already in the past (year 2001): the
    // review countdown flips to expired, Start stays disabled even after the ack,
    // and the honest re-quote message shows (the SDK ALSO re-gates execute).
    final fake = _funded()
      ..swapQuoteResult = swapQuoteFixture(expiresAt: 1000000000);
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester); // OutOfZec review
    await tester.pumpAndSettle(); // let the post-frame onExpired fire
    final l10n = _l10n(tester);
    expect(find.text(l10n.walletSwapQuoteExpired), findsOneWidget);
    // Even after acknowledging, the dead quote can't be confirmed.
    await tester.tap(find.text(l10n.walletSwapAckLabel));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
          )
          .onPressed,
      isNull,
    );
  });

  testWidgets(
    'a quote that never returns times out to an honest fault (no infinite spinner)',
    (tester) async {
      // The maintainer's edge case: a stalled dial must not hang "Getting a quote…"
      // forever — the host-side timeout surfaces an honest, connectivity-focused
      // fault (the request never reached 1Click), never a stuck spinner.
      final fake = _funded()..swapQuoteNeverCompletes = true;
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _selectSell(tester);
      await _pickTarget(tester); // OutOfZec requires a target pick now
      await tester.enterText(_outAmountField, '0.5');
      await tester.enterText(_outDestinationField, '0xdest');
      await tester.tap(find.text(_l10n(tester).walletSwapQuoteButton));
      await tester.pump(); // enter SwapQuoting (the spinner)
      // The quote hangs; advance past the host-side timeout.
      await tester.pump(kSwapNetworkTimeout + const Duration(seconds: 1));
      await tester.pumpAndSettle();
      // Back on the form with the honest connection fault — never a stuck spinner.
      expect(find.text(_l10n(tester).walletSwapFaultConnection), findsWidgets);
    },
  );

  testWidgets('the Available line is QUALIFIED while the wallet is still '
      'catching up (#380 (d)) — a low/zero spendable must not read as '
      'final', (tester) async {
    // A never-synced wallet below tip (lastSynced null + scanning): the
    // spendable figure is the PARTIAL repopulating balance — post-rescan,
    // post-relaunch, or a restore's first sync.
    const midCatchUp = SyncStatus.scanning(
      from: 2_000_000,
      to: 2_500_000,
      percent: 0.4, // determinate (a 0 fraction animates → pumpAndSettle hangs)
      spendableReady: false,
      rewound: false,
    );
    final fake = FakeWalletSession(
      current: midCatchUp,
      snapshotValue: walletStateFixture(syncStatus: midCatchUp),
    );
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _selectSell(tester);
    final l10n = _l10n(tester);

    expect(
      find.text(l10n.walletSwapAvailableCatchingUp(formatZec(0))),
      findsOneWidget,
    );
    expect(
      find.text(l10n.walletSwapAvailable(formatZec(0))),
      findsNothing,
      reason: 'the unqualified figure would read as a final 0',
    );
  });

  testWidgets('S205-c: the Available qualifier drops under the host\'s '
      'sync-off policy — the plain figure renders (no active-progress claim '
      'while nothing runs; the badge story carries the caveat)', (
    tester,
  ) async {
    // The exact mid-catch-up wallet the #380 (d) test above qualifies,
    // differing only in the sync-policy seam.
    const midCatchUp = SyncStatus.scanning(
      from: 2_000_000,
      to: 2_500_000,
      percent: 0.4, // determinate (a 0 fraction animates → pumpAndSettle hangs)
      spendableReady: false,
      rewound: false,
    );
    final fake = FakeWalletSession(
      current: midCatchUp,
      snapshotValue: walletStateFixture(syncStatus: midCatchUp),
    );
    await tester.pumpWidget(
      _harness(
        session: fake,
        extraOverrides: [walletSyncPolicyProvider.overrideWithValue(false)],
      ),
    );
    await tester.pumpAndSettle();
    await _selectSell(tester);
    final l10n = _l10n(tester);

    expect(find.text(l10n.walletSwapAvailable(formatZec(0))), findsOneWidget);
    expect(
      find.text(l10n.walletSwapAvailableCatchingUp(formatZec(0))),
      findsNothing,
      reason: '"still catching up" would claim progress the host turned off',
    );
  });

  // ── #364 swap UX batch ────────────────────────────────────────────────────

  testWidgets(
    'a host-prompt DENIAL restores the same quote\'s review with the ack '
    'INTACT — a re-approve executes without re-acking (#364 F10)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1';
      final auth = FakeSendAuthorizer(denyAll: true);
      await tester.pumpWidget(
        _harness(
          session: fake,
          extraOverrides: [
            walletSendAuthorizerProvider.overrideWithValue(auth),
          ],
        ),
      );
      await tester.pumpAndSettle();
      await _toReview(tester);
      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapConfirmButton));
      await tester.pumpAndSettle();

      // Denied at the prompt → back on the SAME quote's review; the ack the
      // user already gave survives (there is no new fact to re-acknowledge —
      // the pre-#364 state-edge reset wiped it here).
      expect(fake.swapExecuteCount, 0);
      expect(
        tester.widget<CheckboxListTile>(_privacyAck(tester)).value,
        isTrue,
        reason: 'a same-quote restore must not wipe the §2.6 ack (#364 F10)',
      );
      // …and Start is still live: approve → the SAME quote executes.
      auth.denyAll = false;
      await tester.tap(find.text(l10n.walletSwapConfirmButton));
      await tester.pumpAndSettle();
      expect(fake.swapExecuteCount, 1);
    },
  );

  // The counterpart — a NEW quote id still resets the ack — is pinned by the
  // (updated) 'acknowledgment does NOT persist across a re-quote' test above.

  testWidgets(
    'a provider RECYCLING a quote id cannot carry the ack onto changed facts '
    '— the F10 key is quote INSTANCE identity, never the provider-controlled '
    'id (S219 security review HIGH)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture(id: 'q-1', zecSideZat: 50000000);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _toReview(tester);
      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      expect(
        tester.widget<CheckboxListTile>(_privacyAck(tester)).value,
        isTrue,
      );

      // Re-quote: the (adversarial) provider returns DIFFERENT facts under
      // the SAME order id. The review is a fresh instance ⇒ acks reset.
      await tester.tap(find.text(l10n.walletSwapBackButton));
      await tester.pumpAndSettle();
      fake.swapQuoteResult = swapQuoteFixture(id: 'q-1', zecSideZat: 90000000);
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();

      expect(
        tester.widget<CheckboxListTile>(_privacyAck(tester)).value,
        isFalse,
        reason:
            'an id-keyed reset would arrive PRE-CHECKED over facts the user '
            'never acknowledged — the §2.6 gate must not rest on a provider '
            'uniqueness contract',
      );
      expect(
        tester
            .widget<FilledButton>(
              find.widgetWithText(FilledButton, l10n.walletSwapConfirmButton),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets(
    'NON-terminal tracking says "Back to wallet", never "Done" (#364 F12; '
    'the terminal contrast — success DOES say "Done" — is pinned by the '
    '#385 confirmed-terminal test above)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.pendingDeposit(expiresAt: 4100000000);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _executeToTracking(tester);
      final l10n = _l10n(tester);

      // Pending (money in motion): the primary action is honest navigation.
      expect(find.text(l10n.walletSwapStatusPendingTitle), findsOneWidget);
      expect(find.text(l10n.walletSwapBackToWallet), findsOneWidget);
      expect(
        find.text(l10n.walletSwapDone),
        findsNothing,
        reason: '"Done" on a live swap reads as "the swap is done"',
      );
    },
  );

  testWidgets('the swap-unavailable screen says "Back to wallet" (#364 F12)', (
    tester,
  ) async {
    await tester.pumpWidget(
      _harness(session: _funded(), nullSession: true, swapEnabled: false),
    );
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);
    expect(find.text(l10n.walletSwapUnavailableWallet), findsOneWidget);
    expect(find.text(l10n.walletSwapBackToWallet), findsOneWidget);
    expect(find.text(l10n.walletSwapDone), findsNothing);
  });

  testWidgets(
    'tracking status content is ONE live region — transitions are announced '
    'to screen readers (#364 F8)',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteResult = 'swap-1'
        ..swapCurrent = const SwapStatus.pendingDeposit(expiresAt: 4100000000);
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _executeToTracking(tester);
      final l10n = _l10n(tester);

      final node = tester.getSemantics(
        find.text(l10n.walletSwapStatusPendingTitle),
      );
      expect(
        node.flagsCollection.isLiveRegion,
        isTrue,
        reason:
            'the status card swaps silently without a live region — '
            'pending→detected→processing→terminal must be heard',
      );
      // The title AND body ride one merged node, so a transition announces
      // the full story, not a bare title.
      expect(node.label, contains(l10n.walletSwapStatusPendingTitle));
      expect(node.label, contains(l10n.walletSwapStatusPendingBodyOutOfZec));
      semantics.dispose();
    },
  );

  testWidgets(
    'a form fault is a live region and the over-ceiling copy is the SWAP '
    'key, not the send form\'s "limits sends" (#364 S5 + S6)',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture(zecSideZat: 50000000);
      await tester.pumpWidget(
        _harness(
          session: fake,
          extraOverrides: [
            // 0.1 ZEC cap — the quoted 0.5 ZEC deposit exceeds it.
            walletSendCeilingZatProvider.overrideWithValue(10000000),
          ],
        ),
      );
      await tester.pumpAndSettle();
      await _toReview(tester); // the ceiling refusal bounces back to the form
      final l10n = _l10n(tester);

      final overCeiling = l10n.walletSwapFaultOverCeiling(formatZec(10000000));
      expect(find.text(overCeiling), findsOneWidget);
      expect(
        find.text(l10n.walletSendFaultOverCeiling(formatZec(10000000))),
        findsNothing,
        reason: 'the send form\'s "limits sends" copy misreads on a swap form',
      );
      expect(
        tester
            .getSemantics(find.text(overCeiling))
            .flagsCollection
            .isLiveRegion,
        isTrue,
        reason: 'swap faults were silent to screen readers pre-#364 (S5)',
      );
      semantics.dispose();
    },
  );

  testWidgets('the review countdown\'s a11y label is minute-granular — no 1 Hz '
      'announcement storm (#364 F9)', (tester) async {
    final semantics = tester.ensureSemantics();
    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    // ~12.5 min out: the label floors to "12 min" for the whole first
    // half-minute of the test, immune to the seconds ticking underneath.
    final fake = _funded()
      ..swapQuoteResult = swapQuoteFixture(expiresAt: now + 754);
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await _toReview(tester);
    final l10n = _l10n(tester);

    expect(
      find.bySemanticsLabel(l10n.walletSwapQuoteExpiresIn('12 min')),
      findsOneWidget,
      reason:
          'the live region must carry a minute-granular label, not the '
          'per-second countdown',
    );
    semantics.dispose();
  });

  testWidgets(
    'a store-BUSY execute restore keeps the ack — the second F10 motivating '
    'path (S219-b U2; only the denial path was pinned)',
    (tester) async {
      final fake = _funded()
        ..swapQuoteResult = swapQuoteFixture()
        ..swapExecuteThrows = SwapApiError(
          code: 'RW-SWAP-014',
          message: 'static',
          kind: const SwapErrorKind.swapStateBusy(),
        );
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await _toReview(tester);
      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapConfirmButton));
      await tester.pumpAndSettle();

      // Busy consumed nothing: same quote instance restored with the retry
      // banner — and the ack the user already gave survives, so "Start swap"
      // IS the retry (re-acking would contradict the banner's instruction).
      expect(find.text(l10n.walletSwapFaultStoreBusyRetry), findsOneWidget);
      expect(
        tester.widget<CheckboxListTile>(_privacyAck(tester)).value,
        isTrue,
        reason: 'a same-quote busy restore must not wipe the §2.6 ack',
      );
      fake.swapExecuteThrows = null;
      fake.swapExecuteResult = 'swap-1';
      await tester.tap(find.text(l10n.walletSwapConfirmButton));
      await tester.pumpAndSettle();
      expect(fake.swapExecuteCount, 2); // busy attempt + the retry
    },
  );

  testWidgets(
    'a SESSION FLIP clears the ack triple — even a same-INSTANCE quote '
    'arriving after the flip re-requires the acks (S219-b U3, the security '
    'review MED: identity hygiene by construction, not by key strength)',
    (tester) async {
      // One quote INSTANCE shared by both identities' fakes — the strongest
      // adversarial premise: if the flip failed to clear _ackedQuote, the
      // post-flip review would arrive with identical(quote) == true and the
      // OLD identity's ack pre-checked.
      final sharedQuote = swapQuoteFixture(id: 'q-shared');
      final fakeA = _funded()..swapQuoteResult = sharedQuote;
      final fakeB = _funded()..swapQuoteResult = sharedQuote;
      final holder = StateProvider<FakeWalletSession>((_) => fakeA);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWith((ref) => ref.watch(holder)),
            swapEnabledProvider.overrideWithValue(true),
            screenSecurityProvider.overrideWithValue(FakeScreenSecurity()),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const SwapScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await _toReview(tester);
      final l10n = _l10n(tester);
      await tester.tap(find.text(l10n.walletSwapAckLabel));
      await tester.pumpAndSettle();

      // Flip identities mid-review: the controller drops to the form.
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SwapScreen)),
      );
      container.read(holder.notifier).state = fakeB;
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletSwapQuoteButton), findsWidgets);

      // Re-quote under identity B — the SAME quote instance comes back. The
      // ack must be re-required (flip cleared the triple).
      await tester.tap(find.text(l10n.walletSwapDirectionSell));
      await tester.pumpAndSettle();
      await _pickTarget(tester);
      await tester.enterText(_outAmountField, '0.5');
      await tester.enterText(_outDestinationField, '0xdest');
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      expect(
        tester.widget<CheckboxListTile>(_privacyAck(tester)).value,
        isFalse,
        reason:
            'the OLD identity\'s ack must never pre-satisfy the NEW '
            'identity\'s §2.6 gate',
      );
    },
  );

  testWidgets(
    'a same-reason fault REPEAT remounts the fault node — the live region '
    're-announces for screen readers, not only the sighted re-scroll '
    '(S219-b M3; the S5 invariant, both channels)',
    (tester) async {
      final fake = _funded();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await _selectSell(tester);
      final l10n = _l10n(tester);
      await _pickTarget(tester);
      await tester.enterText(_outAmountField, '0.5');
      // Destination left empty → validation fault, twice.
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      final msg = l10n.walletSwapFaultDestinationRequired;
      final firstElement = tester.element(find.text(msg));
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();
      final secondElement = tester.element(find.text(msg));
      expect(
        identical(firstElement, secondElement),
        isFalse,
        reason:
            'without the per-instance remount an identical-text repeat '
            'never re-announces (live regions announce on CHANGE) — the '
            'a11y half of the S5 invariant',
      );
    },
  );

  testWidgets(
    'the form keeps every child inflated and scrolls a validation fault '
    'into view (S219-b F1/U1 — a lazily-uninflated fault card silently '
    'voided BOTH the scroll and the announcement)',
    (tester) async {
      final fake = _funded();
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      // A surface short enough that the form genuinely scrolls.
      await tester.binding.setSurfaceSize(const Size(600, 500));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);

      expect(
        tester.widget<ListView>(find.byType(ListView)).cacheExtent,
        1e5,
        reason:
            'the async-fault path rebuilds the list at offset 0 — without '
            'a covering cache extent a below-fold fault card is never '
            'inflated and the GlobalKey scroll silently no-ops',
      );

      // IntoZec: pick a source, leave the amount empty, reach Get quote.
      await tester.tap(find.byKey(const Key('swap-source-asset-field')));
      await tester.pumpAndSettle();
      await tester.tap(find.bySemanticsLabel('USDC on Ethereum'));
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.text(l10n.walletSwapQuoteButton),
        200,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.tap(find.text(l10n.walletSwapQuoteButton));
      await tester.pumpAndSettle();

      // The fault rendered AND the listener scrolled it into view.
      expect(
        find.text(l10n.walletSwapFaultForeignAmountRequired),
        findsOneWidget,
      );
      final position = tester
          .state<ScrollableState>(find.byType(Scrollable).first)
          .position;
      expect(
        position.pixels,
        greaterThan(0),
        reason: 'ensureVisible must have moved the viewport to the fault',
      );
    },
  );

  testWidgets('S13 §1.1: the refund field\'s (i) opens its explainer whole, '
      'named "More about <the field>"', (tester) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(_harness(session: _funded()));
    await tester.pumpAndSettle();
    final l10n = _l10n(tester);
    final info = find.byKey(const Key('swap-refund-info'));
    expect(info, findsOneWidget);
    expect(
      find.bySemanticsLabel(
        l10n.walletInfoButtonLabel(l10n.walletSwapRefundLabel),
      ),
      findsOneWidget,
    );
    await tester.ensureVisible(info);
    await tester.tap(info);
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSwapRefundInfoTitle), findsOneWidget);
    expect(find.text(l10n.walletSwapRefundInfoBody), findsOneWidget);
    handle.dispose();
  });
}
