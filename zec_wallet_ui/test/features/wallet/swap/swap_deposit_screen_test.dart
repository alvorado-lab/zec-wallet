import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/colors.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/core/theme/typography.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_assets.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_deposit_screen.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations_en.dart';
import 'package:zec_wallet_ui/shared/address_text.dart';
import 'package:zec_wallet_ui/shared/wallet_notice.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

import '../../../support/a11y_activation.dart';

/// IntoZec deposit screen (§3.3b D7 / L7) tests. The money-correctness assertions
/// are the deposit address reaching the QR + the send instruction, and the
/// expired-window guard disabling the "I've sent" hand-off. The screen runs a 1s
/// countdown timer, so tests pump frames (never settle) and tear the tree down to
/// cancel the timer.
void main() {
  const counter = SwapAsset(chain: 'btc', symbol: 'btc', label: 'BTC on BTC');

  Widget harness({
    required SwapQuote quote,
    VoidCallback? onConfirmSent,
    VoidCallback? onLeave,
    VoidCallback? onStartNew,
  }) => MaterialApp(
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    theme: lightTheme,
    home: Scaffold(
      body: SwapDepositView(
        quote: quote,
        counter: counter,
        onConfirmSent: onConfirmSent ?? () {},
        onLeave: onLeave ?? () {},
        onStartNew: onStartNew ?? () {},
      ),
    ),
  );

  group('localizedCountdown', () {
    // Minute magnitudes use unit words, never mm:ss (#364 N3): "15:00" reads
    // as a CLOCK TIME in 24h-clock locales — a duration on a money deadline
    // must not be mistakable for a time-of-day. Minutes are floored (never
    // overstate the remaining window). Unit forms come from the ARB (
    // F2 — the first cut hardcoded Latin units into every locale's spoken
    // labels); these pins run against the EN template.
    final en = WalletLocalizationsEn();
    test('minute words under an hour (floored)', () {
      expect(localizedCountdown(en, 15 * 60), '15 min');
      expect(localizedCountdown(en, 754), '12 min');
      expect(localizedCountdown(en, 65), '1 min');
      expect(localizedCountdown(en, 60), '1 min');
    });
    test('second words under a minute', () {
      expect(localizedCountdown(en, 59), '59 s');
      expect(localizedCountdown(en, 42), '42 s');
      expect(localizedCountdown(en, 1), '1 s');
    });
    test('h m past an hour (minutes zero-padded)', () {
      expect(localizedCountdown(en, 3725), '1h 02m');
      // The hour boundary both ways (U7).
      expect(localizedCountdown(en, 3599), '59 min');
      expect(localizedCountdown(en, 3600), '1h 00m');
    });
    test('zero at or under the deadline', () {
      expect(localizedCountdown(en, 0), '0 s');
      expect(localizedCountdown(en, -5), '0 s');
    });
  });

  testWidgets(
    'the EXPIRED window offers the "start another swap" escape; the live '
    'window does not (S219-b review HIGH — the latched SwapAwaitingDeposit '
    'made the expired screen a process-lifetime dead end)',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));

      // Live window: no escape (leaving is the deliberate dialog-guarded path).
      await tester.pumpWidget(
        harness(quote: swapQuoteFixture(expiresAt: 2000000000)),
      );
      await tester.pump();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SwapDepositView)),
      );
      expect(find.text(l10n.walletSwapStartAnother), findsNothing);
      await tester.pumpWidget(const SizedBox()); // fresh State for the next leg

      // Expired: the notice says "start a new swap" — the button must exist
      // and fire (it clears the latched live state back to the form).
      var startNew = 0;
      await tester.pumpWidget(
        harness(
          quote: swapQuoteFixture(expiresAt: 1000000000), // past
          onStartNew: () => startNew++,
        ),
      );
      await tester.pump();
      await tester.scrollUntilVisible(
        find.text(l10n.walletSwapStartAnother),
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.tap(find.text(l10n.walletSwapStartAnother));
      await tester.pump();
      expect(startNew, 1);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('renders the address QR, send instruction, and copy + sent CTAs', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final quote = swapQuoteFixture(
      depositAddress: 'tdepositADDRxyz',
      amountIn: '100',
      expiresAt: 2000000000, // far future — live window
    );
    await tester.pumpWidget(harness(quote: quote));
    await tester.pump();

    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SwapDepositView)),
    );
    expect(find.text(l10n.walletSwapDepositTitle), findsOneWidget);
    // The deposit address is shown via AddressText (bold ends) — assert the exact
    // full address it carries (its spans mean find.text won't match).
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      'tdepositADDRxyz',
    );
    expect(find.byKey(const Key('swap-deposit-qr-tile')), findsOneWidget);
    // The send instruction carries the exact foreign amount + the source asset.
    expect(find.textContaining('100'), findsWidgets);
    // The hand-off affordance is enabled while the window is open.
    final sent = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.walletSwapDepositSent),
    );
    expect(sent.onPressed, isNotNull);

    await tester.pumpWidget(const SizedBox()); // cancel the countdown timer
  });

  testWidgets('tapping "I\'ve sent the funds" invokes the hand-off', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    var sentTaps = 0;
    final quote = swapQuoteFixture(expiresAt: 2000000000);
    await tester.pumpWidget(
      harness(quote: quote, onConfirmSent: () => sentTaps++),
    );
    await tester.pump();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SwapDepositView)),
    );
    await tester.tap(find.text(l10n.walletSwapDepositSent));
    await tester.pump();
    expect(sentTaps, 1);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'an expired window shows the closed message + disables the sent CTA',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final quote = swapQuoteFixture(expiresAt: 1000000000); // year 2001 — past
      await tester.pumpWidget(harness(quote: quote));
      await tester.pump();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SwapDepositView)),
      );
      expect(find.text(l10n.walletSwapDepositExpired), findsOneWidget);
      final sent = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.walletSwapDepositSent),
      );
      expect(sent.onPressed, isNull);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'the leave dialog\'s LIVE-window body invites copying the address; the '
    'EXPIRED variant warns off sending instead (#364 F11)',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));

      // Live window → the "copy it first" body.
      await tester.pumpWidget(
        harness(quote: swapQuoteFixture(expiresAt: 2000000000)),
      );
      await tester.pump();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SwapDepositView)),
      );
      final navigator = tester.state<NavigatorState>(find.byType(Navigator));
      navigator.maybePop();
      await tester.pump();
      // The dialog route animates in; a plain settle would hang on the 1s
      // countdown ticker, so pump past the transition explicitly.
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text(l10n.walletSwapDepositBackBody), findsOneWidget);
      expect(find.text(l10n.walletSwapDepositBackBodyExpired), findsNothing);
      await tester.tap(find.text(l10n.walletSwapDepositBackStay));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      // Expired window → the "don't send now" body; leaving still works.
      // Tear the live tree down first: the same widget position would reuse
      // the State, whose initState-computed remaining belongs to the LIVE
      // quote.
      await tester.pumpWidget(const SizedBox());
      var left = 0;
      await tester.pumpWidget(
        harness(
          quote: swapQuoteFixture(expiresAt: 1000000000), // past
          onLeave: () => left++,
        ),
      );
      await tester.pump();
      tester.state<NavigatorState>(find.byType(Navigator)).maybePop();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(
        find.text(l10n.walletSwapDepositBackBodyExpired),
        findsOneWidget,
        reason:
            'post-expiry the live body\'s "you\'ll need the address to pay" '
            'invites exactly the send the user must not make',
      );
      expect(find.text(l10n.walletSwapDepositBackBody), findsNothing);
      await tester.tap(find.text(l10n.walletSwapDepositBackLeave));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(left, 1);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'the deposit countdown\'s a11y label is coarse — "less than a minute" '
    'under 60s while the visual text keeps the seconds (#364 F9)',
    (tester) async {
      final semantics = tester.ensureSemantics();
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
      // ~40s left: comfortably inside the sub-minute band for the whole test.
      await tester.pumpWidget(
        harness(quote: swapQuoteFixture(expiresAt: now + 40)),
      );
      await tester.pump();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SwapDepositView)),
      );

      expect(
        find.bySemanticsLabel(
          l10n.walletSwapDepositExpiresIn(l10n.walletCountdownUnderMinute),
        ),
        findsOneWidget,
        reason:
            'a per-second label inside the live region was a 1 Hz '
            'screen-reader announcement storm',
      );
      // The VISUAL text still carries the ticking seconds ("NN s").
      expect(
        find.textContaining(RegExp(r'\d+ s')),
        findsOneWidget,
        reason: 'the sighted countdown keeps its urgency cadence',
      );
      await tester.pumpWidget(const SizedBox());
      semantics.dispose();
    },
  );

  testWidgets('a back-press guard is wired (canPop is false)', (tester) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      harness(quote: swapQuoteFixture(expiresAt: 2000000000)),
    );
    await tester.pump();
    // A PopScope with canPop == false guards a system back (the generic type arg
    // makes byType brittle, so match + read canPop in the predicate).
    expect(
      find.byWidgetPredicate((w) => w is PopScope && w.canPop == false),
      findsOneWidget,
    );
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    'a required deposit memo is shown with the funds-loss warning + the EXACT value',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      // an XRP-destination-tag-style memo (the IntoZec memo-chain case)
      final quote = swapQuoteFixture(
        depositMemo: '100345677',
        expiresAt: 2000000000,
      );
      await tester.pumpWidget(harness(quote: quote));
      await tester.pump();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SwapDepositView)),
      );

      // the required-memo heading + the funds-loss warning are present
      expect(find.text(l10n.walletSwapDepositMemoRequired), findsOneWidget);
      expect(find.text(l10n.walletSwapDepositMemoWarning), findsOneWidget);
      // money-correctness: the EXACT memo is rendered (what the user copies)
      final memo = tester.widget<SelectableText>(
        find.byKey(const Key('swap-deposit-memo')),
      );
      expect(memo.data, '100345677');
      // the memo-copy affordance is offered (distinct from the address copy)
      expect(
        find.widgetWithText(OutlinedButton, l10n.walletSwapDepositMemoCopy),
        findsOneWidget,
      );

      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('S11 C2: the deposit memo is a DANGER card-notice, and its value '
      'stays a SelectableText in the host\'s mono face (never the notice '
      'message)', (tester) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    const hostMono = TextStyle(fontFamily: 'HostMono');
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        theme: buildTheme(
          WalletColors.light,
          extensions: const [WalletTypography(mono: hostMono)],
        ),
        home: Scaffold(
          body: SwapDepositView(
            quote: swapQuoteFixture(
              depositMemo: '100345677',
              expiresAt: 2000000000,
            ),
            counter: counter,
            onConfirmSent: () {},
            onLeave: () {},
            onStartNew: () {},
          ),
        ),
      ),
    );
    await tester.pump();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SwapDepositView)),
    );

    // The card-notice: only `WalletNotice.card` carries a title, and the tone
    // is DANGER (a missing memo loses the funds).
    final memoField = find.byKey(const Key('swap-deposit-memo'));
    final notice = tester.widget<WalletNotice>(
      find.ancestor(of: memoField, matching: find.byType(WalletNotice)),
    );
    expect(notice.title, l10n.walletSwapDepositMemoRequired);
    expect(notice.tone, WalletNoticeTone.danger);
    expect(notice.message, l10n.walletSwapDepositMemoWarning);
    expect(notice.message, isNot(contains('100345677')));
    // The title says so in the danger colour, not only the glyph.
    final title = tester.widget<Text>(
      find.text(l10n.walletSwapDepositMemoRequired),
    );
    expect(title.style?.color, WalletColors.light.red);

    // The value: selectable, and in the HOST's mono face (the style is
    // pinned, not only the key) — its label and Copy button beside it.
    final memo = tester.widget<SelectableText>(memoField);
    expect(memo.data, '100345677');
    expect(memo.style?.fontFamily, hostMono.fontFamily);
    expect(
      find.descendant(
        of: find.byWidget(notice),
        matching: find.text(l10n.walletSwapDepositMemoLabel),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byWidget(notice),
        matching: find.widgetWithText(
          OutlinedButton,
          l10n.walletSwapDepositMemoCopy,
        ),
      ),
      findsOneWidget,
    );

    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('NO memo section when the quote carries no deposit memo', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    // depositMemo defaults to null — the common case (every ZEC deposit, most chains)
    await tester.pumpWidget(
      harness(quote: swapQuoteFixture(expiresAt: 2000000000)),
    );
    await tester.pump();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SwapDepositView)),
    );
    expect(find.text(l10n.walletSwapDepositMemoRequired), findsNothing);
    expect(find.byKey(const Key('swap-deposit-memo')), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('an empty deposit memo renders no memo section', (tester) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    // a defensive host-side guard: the SDK normalizes empty→absent, but the UI
    // also treats "" as no memo (never an empty "required" field).
    await tester.pumpWidget(
      harness(quote: swapQuoteFixture(depositMemo: '', expiresAt: 2000000000)),
    );
    await tester.pump();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SwapDepositView)),
    );
    expect(find.text(l10n.walletSwapDepositMemoRequired), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('tapping "Copy memo" puts the EXACT memo on the clipboard', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    // intercept the platform clipboard channel
    String? copied;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          copied = (call.arguments as Map)['text'] as String?;
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );

    await tester.pumpWidget(
      harness(
        quote: swapQuoteFixture(
          depositMemo: 'memo-xyz-42',
          expiresAt: 2000000000,
        ),
      ),
    );
    await tester.pump();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SwapDepositView)),
    );
    await tester.tap(
      find.widgetWithText(OutlinedButton, l10n.walletSwapDepositMemoCopy),
    );
    await tester.pump();
    expect(copied, 'memo-xyz-42');
    await tester.pumpWidget(const SizedBox());
  });

  // BOTH deposit copies driven the way a screen reader drives them. The rows
  // above use `tester.tap`, a POINTER event that hits the live OutlinedButton
  // and never consults the semantics tree — green throughout the life of the
  // defect, where `excludeSemantics: true` dropped each button's own tap
  // action. These are the two strings that must reach the payer: the deposit
  // address, and (on the chains that require it) the memo without which the
  // deposit is lost. Contract: `test/support/a11y_activation.dart`.
  testWidgets(
    'both deposit copies are ACTIVATABLE by a screen reader, and each semantics '
    'action puts its own EXACT string on the clipboard',
    (tester) async {
      final handle = tester.ensureSemantics();
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String?;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );

      await tester.pumpWidget(
        harness(
          quote: swapQuoteFixture(
            depositAddress: 'bc1qdepositaddressforthea11yrow',
            depositMemo: 'memo-xyz-42',
            expiresAt: 2000000000,
          ),
        ),
      );
      await tester.pump();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SwapDepositView)),
      );

      expectActivatable(
        tester,
        find.semantics.byLabel(l10n.walletSwapDepositCopy),
        reason: 'the deposit address is the one string the payer needs',
      );
      await tester.pump();
      expect(copied, 'bc1qdepositaddressforthea11yrow');

      expectActivatable(
        tester,
        find.semantics.byLabel(l10n.walletSwapDepositMemoCopy),
        reason: 'a required memo omitted from the deposit is lost funds',
      );
      await tester.pump();
      expect(copied, 'memo-xyz-42');
      await tester.pumpWidget(const SizedBox());
      handle.dispose();
    },
  );

  testWidgets(
    'a max-length (256) memo renders fully with no overflow and the sent CTA stays present',
    (tester) async {
      // a REALISTIC phone surface (not the oversize used elsewhere) so the layout
      // is actually exercised — a long required memo must not push the hand-off
      // CTA off-screen or overflow (the body is a scrollable ListView).
      await tester.binding.setSurfaceSize(const Size(390, 844));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final longMemo = 'A' * 256;
      await tester.pumpWidget(
        harness(
          quote: swapQuoteFixture(depositMemo: longMemo, expiresAt: 2000000000),
        ),
      );
      await tester.pump();
      expect(
        tester.takeException(),
        isNull,
        reason: 'a 256-char no-break memo wraps, never overflows',
      );
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SwapDepositView)),
      );
      // the FULL memo is rendered (not elided/truncated) — money-correctness.
      // The body is a lazy list: scroll the memo into the built range first.
      await tester.scrollUntilVisible(
        find.byKey(const Key('swap-deposit-memo')),
        300,
        scrollable: find.byType(Scrollable).first,
      );
      final memo = tester.widget<SelectableText>(
        find.byKey(const Key('swap-deposit-memo')),
      );
      expect(memo.data, longMemo);
      // the hand-off CTA stays REACHABLE — the body is a scrollable ListView, so
      // scrolling it brings the (below-the-fold) "I've sent" button into view.
      // (Target the ListView's Scrollable explicitly — the SelectableText nests
      // its own, so the default single-Scrollable lookup is ambiguous.)
      await tester.scrollUntilVisible(
        find.widgetWithText(FilledButton, l10n.walletSwapDepositSent),
        300,
        scrollable: find.byType(Scrollable).first,
      );
      expect(
        find.widgetWithText(FilledButton, l10n.walletSwapDepositSent),
        findsOneWidget,
      );
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'copies a memo with internal spaces byte-exact (no trim/collapse)',
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1200, 2600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String?;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );

      // a Stellar MEMO_TEXT-style free-text memo with legitimate internal spaces
      const spaced = 'order 12 ab';
      await tester.pumpWidget(
        harness(
          quote: swapQuoteFixture(depositMemo: spaced, expiresAt: 2000000000),
        ),
      );
      await tester.pump();
      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SwapDepositView)),
      );
      await tester.tap(
        find.widgetWithText(OutlinedButton, l10n.walletSwapDepositMemoCopy),
      );
      await tester.pump();
      expect(copied, spaced); // spaces preserved exactly
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets('S13 §1.4: Copy amount writes EXACTLY the quote amount-in, '
      'and says so', (tester) async {
    await tester.binding.setSurfaceSize(const Size(1200, 2600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    String? copied;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          copied = (call.arguments as Map)['text'] as String?;
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    // The provider's own decimal string — trailing zeros and all, never
    // re-formatted (a deposit must match it to the digit).
    const amountIn = '12.340000';
    await tester.pumpWidget(
      harness(
        quote: swapQuoteFixture(amountIn: amountIn, expiresAt: 2000000000),
      ),
    );
    await tester.pump();
    final l10n = WalletLocalizations.of(
      tester.element(find.byType(SwapDepositView)),
    );
    await tester.tap(find.byKey(const Key('swap-deposit-copy-amount')));
    await tester.pump();
    expect(copied, amountIn);
    await tester.pump();
    expect(find.text(l10n.walletSwapDepositAmountCopied), findsOneWidget);
    // A cost stays on screen, not behind an (i).
    expect(find.text(l10n.walletSwapDepositExactNote), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });
}
