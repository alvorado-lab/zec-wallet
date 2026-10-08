import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/router/wallet_router.dart';
import 'package:zec_wallet_ui/core/router/wallet_routes.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_controller.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_screen.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_state.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_entry.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_report.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_request.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// Stage S8 `deadline` (R05) — a send request the entry has reported as "no
/// transaction" cannot pay afterwards, through any door.
///
/// **The load-bearing premise (plan §3.3):** "reported" and "revoked" are two
/// bits. The mount grace REVOKES the request before it reports the negative;
/// the revocation is a property of the REQUEST as the screen holds it — not of
/// the reporter (a bare re-navigation drops it) and not of the flow id (an
/// in-place update re-mints it) — and the CONTROLLER reads it at both places
/// a spend is asked for, before the host's authorizer is invoked.
///
/// Every refusal here is asserted on the MONEY (`sendCount`, `queueCount`, the
/// authorizer's intents) and the REPORT, never on the screen alone; every
/// pinned row proves the refusal keys on the revoke bit and not on
/// `isReported` — a request whose channel was merely reported (a retired
/// channel, a denied prompt) still pays.
///
/// Driven through the REAL `walletRoutes()` builder and a host redirect that
/// HOLDS the send path (an app-lock or auth gate, ordinary on a wallet host):
/// go_router applies redirects before the push's completer attaches, so a
/// held redirect is exactly the mount the entry's grace answers for. The
/// FR-26 file (`wallet_send_entry_report_test.dart`) is cited by LINE by the
/// contract, so nothing here touches it; its helpers are re-stated, not
/// re-used.
void main() {
  final upToDate = walletStateFixture(
    syncStatus: const SyncStatus.upToDate(tip: 1),
    balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
  );

  FakeWalletSession newSession() => FakeWalletSession(
    current: const SyncStatus.upToDate(tip: 1),
    snapshotValue: upToDate,
  );

  /// The one request every row pushes unless it says otherwise. `const`, as a
  /// host writes it — two identical literals ARE the same object, which is why
  /// a request can never identify a flow, and why a revoke keyed on the
  /// request has to survive a re-navigation that hands the screen this very
  /// object again.
  const request = WalletSendRequest(
    address: 'u1alice',
    amountZat: 150000,
    correlationId: 'inv-1',
  );

  /// The router the last [harness] built — for the rows that drive an IN-PLACE
  /// route update (`replace` onto the send path), which no button can do.
  late GoRouter lastRouter;

  /// A host shell whose one button pushes [requests] in turn through the public
  /// entry (the last one repeats) and keeps every report. When [hold] is given,
  /// a host redirect holds the send path until it completes — the redirect
  /// chain the entry's grace was written for. [nested] mounts every wallet
  /// route under a nested navigator (a `ShellRoute`), the way a host with tabs
  /// does.
  Widget harness({
    required List<WalletSendReport> reports,
    List<WalletSendRequest> requests = const [request],
    Completer<String?>? hold,
    bool nested = false,
    FakeWalletSession? session,
    WalletSendAuthorizer? authorizer,
  }) {
    final fake = session ?? newSession();
    var pushes = 0;
    final walletRoute = GoRoute(
      path: WalletRoutes.wallet,
      builder: (context, state) => Scaffold(
        body: Center(
          child: TextButton(
            onPressed: () async {
              final index = pushes.clamp(0, requests.length - 1);
              pushes += 1;
              reports.add(await WalletSendEntry.push(context, requests[index]));
            },
            child: const Text('pay'),
          ),
        ),
      ),
    );
    final routes = <RouteBase>[
      walletRoute,
      ...walletRoutes().where(
        (r) => r is GoRoute && r.path != WalletRoutes.wallet,
      ),
    ];
    final router = GoRouter(
      initialLocation: WalletRoutes.wallet,
      redirect: hold == null
          ? null
          : (context, state) {
              if (state.uri.path == WalletRoutes.send && !hold.isCompleted) {
                return hold.future;
              }
              return null;
            },
      routes: nested
          ? [
              ShellRoute(
                builder: (context, state, child) => Scaffold(body: child),
                routes: routes,
              ),
            ]
          : routes,
    );
    addTearDown(router.dispose);
    lastRouter = router;
    return ProviderScope(
      overrides: [
        walletSessionProvider.overrideWithValue(fake),
        if (authorizer != null)
          walletSendAuthorizerProvider.overrideWithValue(authorizer),
      ],
      child: MaterialApp.router(
        theme: lightTheme,
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        routerConfig: router,
      ),
    );
  }

  WalletLocalizations l10n(WidgetTester tester) =>
      WalletLocalizations.of(tester.element(find.byType(SendScreen)));

  Finder reviewButton(WidgetTester tester) =>
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton);
  Finder confirmButton(WidgetTester tester) =>
      find.widgetWithText(FilledButton, l10n(tester).walletSendConfirmButton);
  Finder queueButton(WidgetTester tester) =>
      find.widgetWithText(OutlinedButton, l10n(tester).walletSendQueueButton);

  /// Tap "pay", let the route mount, and check it did.
  Future<void> openSend(WidgetTester tester) async {
    await tester.tap(find.text('pay'));
    await tester.pumpAndSettle();
    expect(find.byType(SendScreen), findsOneWidget);
  }

  /// Tap "pay" with the send path HELD, let the entry's grace run out (5 s,
  /// private to the entry; 6 s clears it), and check that nothing mounted and
  /// the host was answered "no transaction" — the state every late-mount row
  /// starts from.
  Future<void> pushPastTheGrace(
    WidgetTester tester,
    List<WalletSendReport> reports,
  ) async {
    await tester.pumpAndSettle();
    await tester.tap(find.text('pay'));
    await tester.pump();
    await tester.pump(const Duration(seconds: 6));
    await tester.pump();
    expect(find.byType(SendScreen), findsNothing);
    expect(
      reports.single,
      isA<WalletSendNoTransaction>(),
      reason: 'the grace answered the host before any screen existed',
    );
  }

  /// Drive form → review → confirm on a seeded form that OFFERS the pay path.
  Future<void> payThrough(WidgetTester tester) async {
    await tester.tap(reviewButton(tester));
    await tester.pumpAndSettle();
    await tester.tap(confirmButton(tester));
    await tester.pumpAndSettle();
  }

  /// Attempt the pay path the way a user would, tapping whatever the screen
  /// offers: Review if it is there, Confirm if it is there. On a screen that
  /// offers neither this is a no-op — so the rows that call it assert the
  /// refusal on the MONEY and the REPORT afterwards, never on the buttons.
  /// On the base tree this pays (the R05 witness); under the fix it must not.
  Future<void> tryPayThrough(WidgetTester tester) async {
    if (reviewButton(tester).evaluate().isNotEmpty) {
      await tester.tap(reviewButton(tester));
      await tester.pumpAndSettle();
    }
    if (confirmButton(tester).evaluate().isNotEmpty) {
      await tester.tap(confirmButton(tester));
      await tester.pumpAndSettle();
    }
  }

  /// The screen's "request expired" state, as this file can see it blind: the
  /// send screen is MOUNTED and offers no way to pay — no Review, no Queue, no
  /// Confirm. The expired state's own copy is the implementer's (plan §3.3
  /// "Host-visible surface"); the adjudicator joins its finder here.
  void expectRequestExpired(WidgetTester tester) {
    expect(find.byType(SendScreen), findsOneWidget);
    expect(
      reviewButton(tester),
      findsNothing,
      reason: 'an expired request offers no Review — the pay path is gone',
    );
    expect(
      queueButton(tester),
      findsNothing,
      reason: 'the offline queue is a spend too; it is not offered either',
    );
    expect(confirmButton(tester), findsNothing);
  }

  // ---------------------------------------------------------------------
  // Row 1 — THE PROBE's honest form under arm (a). The standing red at
  // wallet_send_entry_report_test.dart:840-919 asserts the base tree's witness
  // (a payment under the negative report); this is the same scenario asserted
  // on what a conforming build makes true.
  // ---------------------------------------------------------------------

  testWidgets(
    'a redirect held past the grace then released mounts an expired request '
    'that cannot pay',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, hold: hold, session: session),
      );
      await pushPastTheGrace(tester, reports);

      // The gate lets the route through AFTER the host has its answer.
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      // Row 1's three assertions: the MONEY, the SCREEN, the REPORT. Money
      // first, so the red this row prints on a tree that still pays is the
      // R05 witness itself (`sendCount` 1 under a "no transaction" report),
      // not a button.
      await tryPayThrough(tester);
      expect(
        session.sendCount,
        0,
        reason:
            'the host holds "no transaction" for this request — a payment now '
            'is money on chain with no host record (R05)',
      );
      expectRequestExpired(tester);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports, hasLength(1));
      expect(
        reports.single,
        isA<WalletSendNoTransaction>(),
        reason:
            'the negative report stays the true and only answer: nothing this '
            'request signed was broadcast',
      );
    },
  );

  // ---------------------------------------------------------------------
  // Row 2 — THROUGH THE CONTROLLER. `payThrough` never reaches the controller:
  // a screen with no buttons passes a widget-only row whether or not the
  // controller refuses, so a widget-only row cannot see the controller's fence
  // break. This one calls the controller DIRECTLY, the way a host, a stale
  // callback or a future screen could, and asserts the refusal precedes the
  // host's authorizer.
  // ---------------------------------------------------------------------

  testWidgets(
    'a revoked request is refused by the controller before the authorizer is '
    'asked — confirm and queueOffline',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      final authorizer = FakeSendAuthorizer();
      await tester.pumpWidget(
        harness(
          reports: reports,
          hold: hold,
          session: session,
          authorizer: authorizer,
        ),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      // The controller the mounted (expired) screen is bound to, driven with
      // no button in between.
      final container = ProviderScope.containerOf(
        tester.element(find.byType(SendScreen)),
        listen: false,
      );
      final controller = container.read(sendControllerProvider.notifier);
      final flowId = controller.flowId;
      final seen = <SendState>[];
      final watch = container.listen(
        sendControllerProvider,
        (_, next) => seen.add(next),
        fireImmediately: true,
      );
      addTearDown(watch.close);

      // The online door: propose, then confirm.
      await controller.prepare(address: 'u1alice', amountText: '0.0015');
      await tester.pump();
      await controller.confirm();
      await tester.pump();
      // The offline door: the Queue button skips Review and goes here.
      await controller.queueOffline(address: 'u1alice', amountText: '0.0015');
      await tester.pump();

      expect(
        authorizer.intents,
        isEmpty,
        reason:
            "the refusal precedes the host's authorizer: its bracket is opaque "
            'and must not learn about the entry (Relim D4)',
      );
      expect(
        controller.hasEnteredSpend(flowId),
        isFalse,
        reason: 'no spend closure was ever entered for the revoked flow',
      );
      expect(session.sendCount, 0, reason: 'nothing signed');
      expect(session.queueCount, 0, reason: 'nothing committed to the queue');
      expect(
        seen.whereType<SendSubmitting>(),
        isEmpty,
        reason: 'the machine never reached the sign transient',
      );
      expect(
        seen.whereType<SendQueuing>(),
        isEmpty,
        reason: 'nor the queue transient',
      );
      // And the host's answer did not move.
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports.single, isA<WalletSendNoTransaction>());
    },
  );

  // ---------------------------------------------------------------------
  // Row 3 — A BARE-REQUEST RE-NAVIGATION AFTER A REVOKE. The row-658 shape:
  // `replace` onto the send path hands the SAME State a widget whose reporter
  // is null (wallet_router.dart:41) and whose prefill is the very same const
  // object; `didUpdateWidget` re-seeds and `resetToForm` re-mints the flow id.
  // A revoke keyed on the reporter or the flow id is dropped exactly here.
  // ---------------------------------------------------------------------

  testWidgets(
    'a bare-request re-navigation with the same request after a revoke cannot '
    'pay',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, hold: hold, session: session),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      // The host navigates to the send path again, in place, with the same
      // request it already holds an irreversible negative for.
      lastRouter.replace(WalletRoutes.send, extra: request);
      await tester.pumpAndSettle();

      await tryPayThrough(tester);
      expect(
        session.sendCount,
        0,
        reason:
            'the host holds "no transaction" for inv-1; a bare re-navigation '
            'with the same request is not a new host act',
      );
      expectRequestExpired(tester);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports, hasLength(1));
      expect(reports.single, isA<WalletSendNoTransaction>());
    },
  );

  testWidgets(
    'a bare-request re-navigation with the same correlationId but different '
    'fields after a revoke cannot pay',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, hold: hold, session: session),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      // Same host record, corrected amount: the host's record for inv-1 still
      // says "no transaction", so a payment here is R05 under another name.
      lastRouter.replace(
        WalletRoutes.send,
        extra: const WalletSendRequest(
          address: 'u1alice',
          amountZat: 250000,
          correlationId: 'inv-1',
        ),
      );
      await tester.pumpAndSettle();

      await tryPayThrough(tester);
      expect(
        session.sendCount,
        0,
        reason: 'the same correlationId is the same host record',
      );
      expectRequestExpired(tester);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports.single, isA<WalletSendNoTransaction>());
    },
  );

  testWidgets(
    'a bare-request re-navigation with a different correlationId after a '
    'revoke is a new host act and pays (row 789 stands)',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, hold: hold, session: session),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      // (Whether the late mount is expired is row 1's claim, not a
      // precondition here: this row guards against a revoke that reaches
      // FURTHER than its request, and it holds on a tree with no revoke at all.)
      expect(find.byType(SendScreen), findsOneWidget);

      // Same payee, same amount, a DIFFERENT host record: the negative the
      // host holds is for inv-1, and this is inv-2.
      lastRouter.replace(
        WalletRoutes.send,
        extra: const WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          correlationId: 'inv-2',
        ),
      );
      await tester.pumpAndSettle();

      expect(
        reviewButton(tester),
        findsOneWidget,
        reason: 'a new host act gets the live form',
      );
      await payThrough(tester);
      expect(session.sendCount, 1, reason: 'inv-2 was never revoked');
      await tester.pageBack();
      await tester.pumpAndSettle();
      // A bare navigation reports nothing (row 789): inv-1's negative is the
      // only report, and it is still true — inv-1 signed nothing.
      expect(reports, hasLength(1));
      expect(reports.single.correlationId, 'inv-1');
      expect(reports.single, isA<WalletSendNoTransaction>());
    },
  );

  testWidgets(
    'a bare-request re-navigation with no correlationId and different fields '
    'after a revoke is a new host act and pays',
    (tester) async {
      const uncorrelated = WalletSendRequest(
        address: 'u1alice',
        amountZat: 150000,
      );
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(
          reports: reports,
          requests: const [uncorrelated],
          hold: hold,
          session: session,
        ),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      // No host record to hold a negative against; a different amount is a
      // different request.
      lastRouter.replace(
        WalletRoutes.send,
        extra: const WalletSendRequest(address: 'u1alice', amountZat: 250000),
      );
      await tester.pumpAndSettle();

      expect(reviewButton(tester), findsOneWidget);
      await payThrough(tester);
      expect(session.sendCount, 1);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports, hasLength(1));
      expect(reports.single, isA<WalletSendNoTransaction>());
    },
  );

  // ---------------------------------------------------------------------
  // Row 3, the cells the DIFF REVIEW found empty (ADR-0558). Both were
  // red on the each PAID.
  //  (i) a `correlationId` on ONE side only: `sameSendRequest` let a tag the
  //      other request never carried make a "different request" of identical
  //      payment fields — the paying direction on a money predicate;
  // (ii) a FRESH MOUNT: the revoke bit lived in one SendScreen State and was
  //      threaded through `didUpdateWidget` only, so the user popping the
  //      expired screen and the host `go`ing back with the same bare request
  //      met a State with no previous request and paid.
  // ---------------------------------------------------------------------

  testWidgets(
    'a bare-request re-navigation with the same fields and a correlationId '
    'the revoked request never carried cannot pay',
    (tester) async {
      const untagged = WalletSendRequest(address: 'u1alice', amountZat: 150000);
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(
          reports: reports,
          requests: const [untagged],
          hold: hold,
          session: session,
        ),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      // The host's retry wrapper tags what it re-dispatches. The fields are
      // the very payment the host holds "no transaction" for; a tag the first
      // request never carried is not the host saying these are two payments.
      lastRouter.replace(
        WalletRoutes.send,
        extra: const WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          correlationId: 'retry-1',
        ),
      );
      await tester.pumpAndSettle();

      await tryPayThrough(tester);
      expect(
        session.sendCount,
        0,
        reason:
            'the same payment fields under a one-sided tag are the revoked '
            'request; the fields decide unless BOTH sides are tagged',
      );
      expectRequestExpired(tester);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports, hasLength(1));
      expect(reports.single, isA<WalletSendNoTransaction>());
    },
  );

  testWidgets(
    'a bare-request re-navigation with the same fields and the correlationId '
    'dropped after a revoke cannot pay',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, hold: hold, session: session),
      );
      // The tagged fixture (inv-1) is revoked.
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      // The mirror cell: the host re-navigates with the payment alone, no tag.
      lastRouter.replace(
        WalletRoutes.send,
        extra: const WalletSendRequest(address: 'u1alice', amountZat: 150000),
      );
      await tester.pumpAndSettle();

      await tryPayThrough(tester);
      expect(
        session.sendCount,
        0,
        reason: 'dropping the tag does not make inv-1\'s payment a new act',
      );
      expectRequestExpired(tester);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports, hasLength(1));
      expect(reports.single, isA<WalletSendNoTransaction>());
    },
  );

  testWidgets(
    'a fresh mount of the send route with the same bare request after a '
    'revoke cannot pay',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, hold: hold, session: session),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      // The user leaves the expired screen: the State that held the bit is
      // disposed with it.
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsNothing);

      // The host navigates to the send path afresh with the request it holds
      // a final negative for — a NEW SendScreen, a NEW State, `initState`
      // with no previous request to compare against.
      lastRouter.go(WalletRoutes.send, extra: request);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      await tryPayThrough(tester);
      expect(
        session.sendCount,
        0,
        reason:
            'the revocation is a property of the request, not of the screen '
            'State that first learned it',
      );
      expectRequestExpired(tester);
      // A bare navigation reports nothing (row 789): the grace's negative is
      // the only report and it is still true.
      expect(reports, hasLength(1));
      expect(reports.single, isA<WalletSendNoTransaction>());
    },
  );

  testWidgets(
    'a fresh push of a request the grace revoked is a new grant and pays '
    '(pinned: the live channel outranks the revoked memory)',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, hold: hold, session: session),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expectRequestExpired(tester);
      await tester.pageBack();
      await tester.pumpAndSettle();

      // "To pay, start again from the app": the same request, pushed again on
      // a fresh channel, with the redirect no longer holding the route.
      await openSend(tester);
      expect(
        reviewButton(tester),
        findsOneWidget,
        reason: 'a fresh push is the restart the copy promises',
      );
      await payThrough(tester);
      expect(session.sendCount, 1, reason: 'the new grant pays');
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports, hasLength(2));
      expect(reports.first, isA<WalletSendNoTransaction>());
      expect(reports.last, isA<WalletSendTransactionCreated>());
    },
  );

  // ---------------------------------------------------------------------
  // Row 4 — CANCELLED REDIRECT: the gate never lets the route through.
  // ---------------------------------------------------------------------

  testWidgets('a redirect that never releases revokes and reports honestly', (
    tester,
  ) async {
    final hold = Completer<String?>();
    final reports = <WalletSendReport>[];
    final session = newSession();
    await tester.pumpWidget(
      harness(reports: reports, hold: hold, session: session),
    );
    await pushPastTheGrace(tester, reports);

    // Long after the grace, still nothing — no screen, no spend, and the
    // one answer the host has is the honest one. Bounded pumps: the held
    // redirect is a pending navigation, and pumpAndSettle would wait on it.
    await tester.pump(const Duration(seconds: 30));
    await tester.pump();
    expect(find.byType(SendScreen), findsNothing);
    expect(session.sendCount, 0);
    expect(reports, hasLength(1));
    expect(reports.single, isA<WalletSendNoTransaction>());
    expect(reports.single.correlationId, 'inv-1');
    // (That the REQUEST is revoked — not just reported — is what the released
    // form of this redirect proves in row 1; the channel-level bit is asserted
    // in the seam file.)
  });

  // ---------------------------------------------------------------------
  // Row 5 — PINNED: mounted before the grace, paid after it. The timer reads
  // `attached`, as today; a revoke that fired on the clock alone would break
  // this. Green on the base tree BY DESIGN.
  // ---------------------------------------------------------------------

  testWidgets(
    'a route mounted within the grace and paid after it reports the payment '
    '(pinned)',
    (tester) async {
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(harness(reports: reports, session: session));
      await openSend(tester);
      // The user thinks about it for longer than the grace.
      await tester.pump(const Duration(seconds: 6));
      await tester.pump();
      expect(reports, isEmpty, reason: 'a mounted screen owns the answer');
      expect(reviewButton(tester), findsOneWidget);

      await payThrough(tester);
      expect(session.sendCount, 1);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports.single, isA<WalletSendTransactionCreated>());
      expect(reports.single, isNot(isA<WalletSendNoTransaction>()));
    },
  );

  // ---------------------------------------------------------------------
  // Row 6 — NESTED NAVIGATOR: the same late mount under a ShellRoute. The
  // pinned twin first, so the nested harness is proven to mount, pay, pop and
  // report on its own before the refusal row leans on it.
  // ---------------------------------------------------------------------

  testWidgets(
    'a route mounted within the grace under a nested navigator pays and '
    'reports (pinned)',
    (tester) async {
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, session: session, nested: true),
      );
      await openSend(tester);
      await payThrough(tester);
      expect(session.sendCount, 1);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsNothing);
      expect(reports.single, isA<WalletSendTransactionCreated>());
    },
  );

  testWidgets(
    'a late mount under a nested navigator cannot pay under a negative report',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, hold: hold, session: session, nested: true),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      await tryPayThrough(tester);
      expect(session.sendCount, 0);
      expectRequestExpired(tester);
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports, hasLength(1));
      expect(reports.single, isA<WalletSendNoTransaction>());
    },
  );

  // ---------------------------------------------------------------------
  // The two-bits premise, pinned from the other side: a channel that was
  // REPORTED without a revoke leaves its flow spendable. Rows 658 and 687 of
  // the FR-26 file are the contract's rows 7 and 8 and stand as they are; this
  // is the retired-channel shape row 8 names — `_closeIdleClaims` reports the
  // old channel "no transaction" when a host replaces the route in place, and
  // the bare form it leaves behind is a live one. A refusal keyed on
  // `isReported` would fire here; the revoke bit never was set.
  // Green on the base tree BY DESIGN.
  // ---------------------------------------------------------------------

  testWidgets(
    'an in-place update that retired a reported channel leaves the bare form '
    'spendable — the revoke bit is not the reported bit (rows 658 and 687)',
    (tester) async {
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(harness(reports: reports, session: session));
      await openSend(tester);

      // The host replaces the route in place with the SAME const request: the
      // channel is retired and answered (row 658), the request is not revoked.
      lastRouter.replace(WalletRoutes.send, extra: request);
      await tester.pumpAndSettle();
      expect(
        reports.single,
        isA<WalletSendNoTransaction>(),
        reason: 'the retired channel was answered — reported, not revoked',
      );

      expect(reviewButton(tester), findsOneWidget);
      await payThrough(tester);
      expect(
        session.sendCount,
        1,
        reason:
            'a reported channel is not a revoked request: the bare form the '
            'host navigated to is still a live one',
      );
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(
        reports,
        hasLength(1),
        reason: 'a bare navigation reports nothing',
      );
    },
  );

  // ---------------------------------------------------------------------
  // UNLISTED (IT-1 +A) — the restart the copy promises. "The user starts again
  // from the app (one tap)": on a real host that tap re-pushes the SAME
  // request — same fields, same correlationId — through a NEW push, with a
  // new channel and its own report. A revoke that outlived its grant (a
  // request-keyed mark nothing clears) would strand that invoice for good, and
  // the copy's promise would be false. Green on the base tree BY DESIGN; its
  // value is post-fix.
  // ---------------------------------------------------------------------

  testWidgets(
    'a fresh push of the same request after a revoke is a new grant and pays '
    '— the restart the copy promises',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(reports: reports, hold: hold, session: session),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      // The late screen is left WITHOUT paying (row 1 says it cannot; this
      // row does not depend on that): the user goes back to the app.
      expect(find.byType(SendScreen), findsOneWidget);
      await tester.pageBack();
      await tester.pumpAndSettle();

      // The user starts again from the app. The gate is open this time; the
      // same const request rides a fresh push.
      await openSend(tester);
      expect(
        reviewButton(tester),
        findsOneWidget,
        reason:
            'a new push is a new grant — the revoke was per push, not per '
            'invoice',
      );
      await payThrough(tester);
      expect(session.sendCount, 1);
      await tester.pageBack();
      await tester.pumpAndSettle();

      expect(reports, hasLength(2));
      expect(reports[0], isA<WalletSendNoTransaction>());
      expect(reports[1], isA<WalletSendTransactionCreated>());
      expect(reports.map((r) => r.correlationId), ['inv-1', 'inv-1']);
    },
  );
}
