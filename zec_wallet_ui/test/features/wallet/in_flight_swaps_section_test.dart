import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet_ui/core/router/wallet_routes.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/in_flight_swaps_section.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_controller.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_state.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// The durable SWAP HOME surface (W-swap-5, #366): rows from the wallet's own
/// encrypted store, direction-honest copy, "View swap" re-attach, honest error
/// line, self-hide when empty. Host-VM only — the fake session stands in for
/// the FFI; the provider fence itself is pinned in
/// wallet_identity_fence_test.dart.

WalletLocalizations _l10nAt(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(InFlightSwapsSection)));

/// A minimal router harness: the section under test at '/', a placeholder at
/// the swap route so the row tap's push has a live target (the section calls
/// `context.push(WalletRoutes.swap)` — a plain MaterialApp would throw).
Widget _harness(
  FakeWalletSession session, {
  ProviderContainer? container,
  double? textScale,
}) {
  final router = GoRouter(
    routes: [
      GoRoute(
        path: '/',
        // The production shape: the section lives inside the wallet screen's
        // scrollable — a bare body would report the SCAFFOLD's vertical
        // overflow at large text scales, masking what the row test measures.
        builder: (_, _) =>
            Scaffold(body: ListView(children: const [InFlightSwapsSection()])),
      ),
      GoRoute(
        path: WalletRoutes.swap,
        builder: (_, _) =>
            const Scaffold(body: Text('swap-screen-placeholder')),
      ),
    ],
  );
  final app = MaterialApp.router(
    localizationsDelegates: WalletLocalizations.localizationsDelegates,
    supportedLocales: WalletLocalizations.supportedLocales,
    theme: lightTheme,
    routerConfig: router,
    // Force a fixed text scale INSIDE the app (the wallet_screen_test idiom —
    // MaterialApp rebuilds MediaQuery, so an outer clamp would be lost).
    builder: textScale == null
        ? null
        : (context, child) => MediaQuery.withClampedTextScaling(
            minScaleFactor: textScale,
            maxScaleFactor: textScale,
            child: child!,
          ),
  );
  return container == null
      ? ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(session)],
          child: app,
        )
      : UncontrolledProviderScope(container: container, child: app);
}

void main() {
  testWidgets('self-hides when nothing is in flight', (tester) async {
    final fake = FakeWalletSession();
    await tester.pumpWidget(_harness(fake));
    await tester.pumpAndSettle();
    expect(find.byType(TextButton), findsNothing);
    expect(find.byType(Icon), findsNothing);
  });

  testWidgets(
    'renders direction-honest rows with the started stamp; the forward-compat '
    'unknown arm gets NO re-attach action',
    (tester) async {
      final fake = FakeWalletSession()
        ..inFlightSwapsResult = [
          swapRecordFixture(
            id: 'out-1',
            direction: SwapRecordDirection.outOfZec,
          ),
          swapRecordFixture(id: 'in-1', direction: SwapRecordDirection.intoZec),
          swapRecordFixture(
            id: 'mystery-1',
            direction: SwapRecordDirection.unknown,
          ),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(find.text(l10n.walletSwapsInFlightTitle(3)), findsOneWidget);
      expect(find.text(l10n.walletSwapInFlightRowOutOfZec), findsOneWidget);
      expect(find.text(l10n.walletSwapInFlightRowIntoZec), findsOneWidget);
      expect(find.text(l10n.walletSwapInFlightRowGeneric), findsOneWidget);
      // Exactly TWO view actions — the unknown arm renders without one (an
      // attach would have to guess the direction-split money copy).
      expect(find.text(l10n.walletSwapViewSwap), findsNWidgets(2));
      // The record id itself never renders (§5.4 render-hygiene: for the
      // shipped provider it EQUALS the deposit address).
      expect(find.textContaining('out-1'), findsNothing);
    },
  );

  testWidgets('rows survive 3.0x text scale on a 320dp phone with no overflow '
      '(arch M-A1) — the action stacks under the copy at large scales', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(320, 800));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final fake = FakeWalletSession()
      ..inFlightSwapsResult = [
        swapRecordFixture(id: 'ax-1', direction: SwapRecordDirection.outOfZec),
      ];
    await tester.pumpWidget(_harness(fake, textScale: 3.0));
    await tester.pumpAndSettle();
    expect(
      tester.takeException(),
      isNull,
      reason: 'pre-fix the shared row overflowed 144-346px at 3.0x/320dp',
    );
    final l10n = _l10nAt(tester);
    expect(find.text(l10n.walletSwapViewSwap), findsOneWidget);
  });

  testWidgets('at 320dp/1.3x the copy STILL keeps the row — the half of M-A1 '
      'the scale-only rule left open (#409 R3: pre-fix the direction line kept '
      '67dp and this ONE row ran 336dp tall, throwing nothing)', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(320, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final fake = FakeWalletSession()
      ..inFlightSwapsResult = [
        swapRecordFixture(id: 'nx-1', direction: SwapRecordDirection.outOfZec),
      ];
    await tester.pumpWidget(_harness(fake, textScale: 1.3));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester);
    final thrown = tester.takeException();
    final line = tester.getSize(find.text(l10n.walletSwapInFlightRowOutOfZec));
    expect(thrown, isNull, reason: 'this starvation was always silent');
    expect(
      line.width,
      greaterThan(200),
      reason: 'the direction line keeps the width once the actions stack',
    );
  });

  testWidgets('the Remove button announces its own label on its OWN node '
      '(#409 R3 review: the Semantics wrapper merged UPWARD inside the row\'s '
      'container node and left the button announcing "")', (tester) async {
    final handle = tester.ensureSemantics();
    final fake = FakeWalletSession()
      ..inFlightSwapsResult = [
        swapRecordFixture(id: 'ax-2', direction: SwapRecordDirection.outOfZec),
      ];
    await tester.pumpWidget(_harness(fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester);

    final node = tester.getSemantics(find.byType(IconButton));
    expect(
      node.label,
      '${l10n.walletSwapRemove}: ${l10n.walletSwapInFlightRowOutOfZec}',
      reason: 'the label must be ON the button node, not merged into the row',
    );
    expect(node.getSemanticsData().flagsCollection.isButton, isTrue);

    // 44dp floor (flutter-patterns): `VisualDensity.compact` used to subtract
    // 4dp per axis AFTER the constraints and shipped a 40x40 target.
    final box = tester.getSize(find.byType(IconButton));
    expect(box.width, greaterThanOrEqualTo(44));
    expect(box.height, greaterThanOrEqualTo(44));
    // Disposed in the body, not a tearDown — the framework verifies handles
    // BEFORE tearDowns run.
    handle.dispose();
  });

  testWidgets('a failed read renders the honest error line, never a silent '
      'hide', (tester) async {
    final fake = FakeWalletSession()
      ..listInFlightSwapsThrows = Exception('store busy');
    await tester.pumpWidget(_harness(fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester);
    expect(find.text(l10n.walletSwapsInFlightError), findsOneWidget);
  });

  testWidgets(
    'the read-error line carries an inline retry that re-pulls in place — the '
    'home has no pull-to-refresh, and this list is a swap\'s only wallet-side '
    'witness (S199-c UX+reliability MED)',
    (tester) async {
      final fake = FakeWalletSession()
        ..listInFlightSwapsThrows = Exception('store busy');
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      expect(find.text(l10n.walletSwapsInFlightError), findsOneWidget);
      final retry = find.byKey(const Key('swaps-in-flight-error-retry'));
      expect(retry, findsOneWidget);
      expect(
        find.descendant(
          of: retry,
          matching: find.text(l10n.walletSwapsInFlightRetry),
        ),
        findsOneWidget,
      );
      final readsBefore = fake.listInFlightSwapsCount;

      // The transient fault clears; tapping retry re-pulls the identity-scoped
      // reader and the section recovers to data in place (error gone, row shown).
      fake
        ..listInFlightSwapsThrows = null
        ..inFlightSwapsResult = [
          swapRecordFixture(
            id: 'recovered-1',
            direction: SwapRecordDirection.outOfZec,
          ),
        ];
      await tester.tap(retry);
      await tester.pumpAndSettle();

      expect(
        fake.listInFlightSwapsCount,
        greaterThan(readsBefore),
        reason:
            'the retry re-pulled the identity-scoped in-flight-swaps reader',
      );
      expect(find.text(l10n.walletSwapsInFlightError), findsNothing);
      expect(find.text(l10n.walletSwapsInFlightTitle(1)), findsOneWidget);
    },
  );

  testWidgets(
    'retry on a PERSISTENT fault is never a dead end — error line + button '
    're-render and each tap re-pulls (S200 review pin)',
    (tester) async {
      final fake = FakeWalletSession()
        ..listInFlightSwapsThrows = Exception('store busy');
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final retry = find.byKey(const Key('swaps-in-flight-error-retry'));
      final readsBefore = fake.listInFlightSwapsCount;

      // The fault does NOT clear; tap anyway.
      await tester.tap(retry);
      await tester.pumpAndSettle();

      expect(
        fake.listInFlightSwapsCount,
        greaterThan(readsBefore),
        reason: 'the tap re-pulled even though the fault persists',
      );
      expect(
        find.text(l10n.walletSwapsInFlightError),
        findsOneWidget,
        reason: 'the honest error line re-renders — never a silent hide',
      );
      expect(retry, findsOneWidget, reason: 'no dead end after a failed retry');
      expect(
        tester.widget<TextButton>(retry).onPressed,
        isNotNull,
        reason: 'the settled error arm re-enables the button for another try',
      );
    },
  );

  testWidgets(
    'View swap ATTACHES the controller to the record and opens the swap '
    'surface (the #366 re-attach)',
    (tester) async {
      final fake = FakeWalletSession()
        ..inFlightSwapsResult = [
          swapRecordFixture(
            id: 'reattach-1',
            direction: SwapRecordDirection.intoZec,
          ),
        ];
      final container = ProviderContainer(
        overrides: [walletSessionProvider.overrideWithValue(fake)],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(_harness(fake, container: container));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await tester.tap(find.text(l10n.walletSwapViewSwap));
      await tester.pumpAndSettle();
      final state = container.read(swapControllerProvider);
      expect(state, isA<SwapExecuted>());
      expect((state as SwapExecuted).swapId, 'reattach-1');
      expect(state.direction, SwapFlowDirection.intoZec);
      expect(
        state.reattached,
        isTrue,
        reason:
            '#367: a row attach is a RE-ATTACH — the tracking copy must '
            'not instruct actions only the issuing run could follow',
      );
      expect(find.text('swap-screen-placeholder'), findsOneWidget);
    },
  );

  testWidgets('a record whose deposit window has PASSED renders the neutral '
      'check-status line, not present-tense motion (#367, S192-b M-UX-2)', (
    tester,
  ) async {
    final fake = FakeWalletSession()
      ..inFlightSwapsResult = [
        swapRecordFixture(
          id: 'stale-1',
          direction: SwapRecordDirection.outOfZec,
          depositDeadline: 1700000900, // long past vs the real clock
        ),
      ];
    await tester.pumpWidget(_harness(fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester);
    expect(find.text(l10n.walletSwapInFlightRowPastWindow), findsOneWidget);
    expect(
      find.text(l10n.walletSwapInFlightRowOutOfZec),
      findsNothing,
      reason: '"on its way" is false once the window lapsed',
    );
    expect(
      find.text(l10n.walletSwapViewSwap),
      findsOneWidget,
      reason: 'the re-attach stays — tracking shows the live truth',
    );
  });

  testWidgets(
    'an UNRESOLVED record past its settlement bound renders the overdue line '
    'with the balance promise (#382 — such rows list instead of vanishing)',
    (tester) async {
      final fake = FakeWalletSession()
        ..inFlightSwapsResult = [
          swapRecordFixture(
            id: 'overdue-1',
            direction: SwapRecordDirection.outOfZec,
            depositDeadline: 1700000900, // long past vs the real clock
            expiresAt: 1700172800, // settlement bound long past too
          ),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(find.text(l10n.walletSwapInFlightRowOverdue), findsOneWidget);
      expect(
        find.text(l10n.walletSwapInFlightRowPastWindow),
        findsNothing,
        reason:
            'past-settlement outranks past-deposit-window — the overdue line '
            'carries the money promise ("any ZEC coming back shows up in '
            'your balance")',
      );
      expect(
        find.text(l10n.walletSwapViewSwap),
        findsOneWidget,
        reason: 'the re-attach handle survives any absence length (#382)',
      );
    },
  );

  testWidgets(
    'an UNRESOLVED IntoZec record past its bound renders the delivery-shaped '
    'overdue line (#385 — "coming back" was refund-shaped for a delivery)',
    (tester) async {
      final fake = FakeWalletSession()
        ..inFlightSwapsResult = [
          swapRecordFixture(
            id: 'overdue-iz',
            direction: SwapRecordDirection.intoZec,
            depositDeadline: 1700000900,
            expiresAt: 1700172800,
          ),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(
        find.text(l10n.walletSwapInFlightRowOverdueIntoZec),
        findsOneWidget,
        reason: 'IntoZec gets "any ZEC it delivers", not "coming back"',
      );
      expect(find.text(l10n.walletSwapInFlightRowOverdue), findsNothing);
    },
  );

  testWidgets('the Remove dialog body for an unresolved OutOfZec row keeps the '
      'refund-watch disclosure (#385 — the direction split must not lose it)', (
    tester,
  ) async {
    final fake = FakeWalletSession()
      ..inFlightSwapsResult = [
        swapRecordFixture(
          id: 'remove-oz',
          direction: SwapRecordDirection.outOfZec,
        ),
      ];
    await tester.pumpWidget(_harness(fake));
    await tester.pumpAndSettle();
    final l10n = _l10nAt(tester);
    await tester.tap(find.byTooltip(l10n.walletSwapRemove));
    await tester.pumpAndSettle();
    expect(find.text(l10n.walletSwapRemoveBodyInFlight), findsOneWidget);
    expect(find.text(l10n.walletSwapRemoveBodyInFlightIntoZec), findsNothing);
  });

  testWidgets(
    'a PINNED terminal outcome renders its honest row line (#367 — the '
    'away-observation is never erased unseen)',
    (tester) async {
      final fake = FakeWalletSession()
        ..inFlightSwapsResult = [
          swapRecordFixture(
            id: 'done-1',
            direction: SwapRecordDirection.outOfZec,
            outcome: SwapOutcome.refunded,
            // PAST settlement bound (#382): a pinned row must render its
            // outcome, never the unresolved-overdue line — outcome is the
            // most-certain tier.
            expiresAt: 1700172800,
          ),
          swapRecordFixture(
            id: 'done-2',
            direction: SwapRecordDirection.intoZec,
            outcome: SwapOutcome.success,
          ),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(find.text(l10n.walletSwapRowOutcomeRefunded), findsOneWidget);
      expect(find.text(l10n.walletSwapRowOutcomeSuccess), findsOneWidget);
      expect(
        find.text(l10n.walletSwapInFlightRowOutOfZec),
        findsNothing,
        reason: 'a pinned terminal replaces the motion line',
      );
      expect(
        find.text(l10n.walletSwapInFlightRowOverdue),
        findsNothing,
        reason: 'a pinned row past its bound shows its OUTCOME, never overdue',
      );
    },
  );

  testWidgets(
    'Remove asks for confirmation, dismisses the record, and refreshes the '
    'list (#367 — the confirm names the dropped re-attach handle in flight)',
    (tester) async {
      final fake = FakeWalletSession()
        ..inFlightSwapsResult = [
          swapRecordFixture(
            id: 'remove-1',
            direction: SwapRecordDirection.intoZec,
          ),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await tester.tap(find.byTooltip(l10n.walletSwapRemove));
      await tester.pumpAndSettle();
      // The in-flight body (no pinned outcome), IntoZec ARM (#385 HIGH-1):
      // explicit that the swap is NOT cancelled and the re-attach handle goes
      // — and direction-honest (the shared body's refund/rescan claims were
      // false for IntoZec).
      expect(
        find.text(l10n.walletSwapRemoveBodyInFlightIntoZec),
        findsOneWidget,
      );
      expect(
        find.text(l10n.walletSwapRemoveBodyInFlight),
        findsNothing,
        reason:
            'the OutOfZec refund-watch body must not render for an IntoZec '
            'row — its three claims (refund watch, ZEC refunded later, '
            'rescan-findable) are all false there (#385 HIGH-1)',
      );
      // Cancel first: nothing dismissed.
      await tester.tap(find.text(l10n.walletSwapRemoveCancel));
      await tester.pumpAndSettle();
      expect(fake.dismissSwapRecordCount, 0);
      // Confirm: the record is dismissed and the list re-read.
      final readsBefore = fake.listInFlightSwapsCount;
      await tester.tap(find.byTooltip(l10n.walletSwapRemove));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletSwapRemoveConfirm));
      await tester.pumpAndSettle();
      expect(fake.dismissSwapRecordCount, 1);
      expect(fake.lastDismissedSwapId, 'remove-1');
      expect(
        fake.listInFlightSwapsCount,
        greaterThan(readsBefore),
        reason: 'the home list refreshes after a confirmed remove',
      );
    },
  );
}
