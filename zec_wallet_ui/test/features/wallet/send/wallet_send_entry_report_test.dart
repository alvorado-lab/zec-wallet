import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/legacy.dart' show StateProvider;
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/router/wallet_router.dart';
import 'package:zec_wallet_ui/core/router/wallet_routes.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/send/send_controller.dart'
    show sendControllerProvider, sendFlowOutcomeProvider;
import 'package:zec_wallet_ui/features/wallet/send/send_screen.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_entry.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_report.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_request.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// FR-26 — the prefilled entry REPORTS the outcome of the flow it opened.
///
/// **The load-bearing row is the negative one.** A build in which paying and
/// backing out yield the same value fails this file, and that is the whole
/// point: before this seam existed, `WalletSendEntry.push` returned the
/// navigation future, which completes on POP — a signal true of the user who
/// paid and of the user who abandoned alike. Every other row here exists to
/// stop the fix from being "return something, anything".
///
/// Driven through the REAL `walletRoutes()` builder, because the route builder
/// is half the seam: it has to unwrap the entry's envelope and still accept a
/// bare request (a host navigating by path itself gets no report — the
/// pre-FR-26 behaviour, not a crash).
void main() {
  final upToDate = walletStateFixture(
    syncStatus: const SyncStatus.upToDate(tip: 1),
    balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
  );

  FakeWalletSession newSession() => FakeWalletSession(
    current: const SyncStatus.upToDate(tip: 1),
    snapshotValue: upToDate,
  );

  /// The router the last [harness] built — for the rows that have to drive an
  /// IN-PLACE route update (`replace`/`go` onto the send path), which no button
  /// in the shell can do.
  late GoRouter lastRouter;

  /// A host shell whose one button pushes the send flow through the public
  /// entry and keeps whatever report comes back. Successive taps push
  /// successive [requests] (the last one repeats), so one tree can drive
  /// several payments the way a real host session does.
  Widget harness({
    required List<WalletSendRequest> requests,
    required List<WalletSendReport> reports,
    FakeWalletSession? session,
    WalletSendAuthorizer? authorizer,
    StateProvider<WalletSession?>? sessionSwitch,
  }) {
    final fake = session ?? newSession();
    var pushes = 0;
    final router = GoRouter(
      initialLocation: WalletRoutes.wallet,
      routes: [
        GoRoute(
          path: WalletRoutes.wallet,
          builder: (context, state) => Scaffold(
            body: Center(
              child: TextButton(
                onPressed: () async {
                  final index = pushes.clamp(0, requests.length - 1);
                  pushes += 1;
                  final request = requests[index];
                  reports.add(await WalletSendEntry.push(context, request));
                },
                child: const Text('pay'),
              ),
            ),
          ),
        ),
        ...walletRoutes().where(
          (r) => r is GoRoute && r.path != WalletRoutes.wallet,
        ),
      ],
    );
    lastRouter = router;
    return ProviderScope(
      overrides: [
        if (sessionSwitch != null)
          walletSessionProvider.overrideWith((ref) => ref.watch(sessionSwitch))
        else
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

  Future<void> openSend(WidgetTester tester) async {
    await tester.tap(find.text('pay'));
    await tester.pumpAndSettle();
    expect(find.byType(SendScreen), findsOneWidget);
  }

  /// Drive form → review → confirm on an already-seeded form.
  Future<void> payThrough(WidgetTester tester) async {
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendConfirmButton),
    );
    await tester.pumpAndSettle();
  }

  testWidgets(
    'THE LOAD-BEARING ROW — paying and backing out are DISTINGUISHABLE. A '
    'build where both report the same value fails here, which is the exact '
    'shape of the pop-future this seam replaces.',
    (tester) async {
      // The SAME request twice: paid once, abandoned once. Identical inputs, so
      // nothing but the user's behaviour can account for a difference.
      final reports = <WalletSendReport>[];
      const request = WalletSendRequest(address: 'u1alice', amountZat: 150000);
      await tester.pumpWidget(
        harness(requests: const [request, request], reports: reports),
      );

      await openSend(tester);
      await payThrough(tester);
      await tester.pageBack(); // leave the result screen
      await tester.pumpAndSettle();

      await openSend(tester);
      await tester.pageBack(); // straight out, nothing signed
      await tester.pumpAndSettle();

      expect(reports, hasLength(2));
      final paid = reports[0];
      final abandoned = reports[1];
      expect(paid, isA<WalletSendTransactionCreated>());
      expect(abandoned, isA<WalletSendNoTransaction>());
      expect(paid.runtimeType, isNot(abandoned.runtimeType));
    },
  );

  testWidgets('a paid flow carries the txid — the host can cite the payment', (
    tester,
  ) async {
    final reports = <WalletSendReport>[];
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
      ),
    );
    await openSend(tester);
    await payThrough(tester);
    await tester.pageBack();
    await tester.pumpAndSettle();

    final report = reports.single as WalletSendTransactionCreated;
    expect(report.txids, isNotEmpty);
    expect(report.broadcastCount, report.txids.length);
    expect(report.motion, WalletSendMotion.notInMotion);
  });

  testWidgets('the correlation token travels WITH the request and comes back '
      'verbatim — no join on recipient+amount', (tester) async {
    final reports = <WalletSendReport>[];
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(
            address: 'u1alice',
            amountZat: 150000,
            correlationId: 'host-record-7',
          ),
        ],
        reports: reports,
      ),
    );
    await openSend(tester);
    await tester.pageBack();
    await tester.pumpAndSettle();

    expect(reports.single.correlationId, 'host-record-7');
  });

  testWidgets('two entries pushed in one session each correlate to their OWN '
      'request', (tester) async {
    final reports = <WalletSendReport>[];
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(
            address: 'u1alice',
            amountZat: 150000,
            correlationId: 'first',
          ),
          WalletSendRequest(
            address: 'u1bob',
            amountZat: 250000,
            correlationId: 'second',
          ),
        ],
        reports: reports,
      ),
    );
    // The first is PAID, the second abandoned — so a build that simply echoed
    // "the last request pushed" onto every report would still have to get the
    // pairing right.
    await openSend(tester);
    await payThrough(tester);
    await tester.pageBack();
    await tester.pumpAndSettle();

    await openSend(tester);
    await tester.pageBack();
    await tester.pumpAndSettle();

    expect(reports.map((r) => r.correlationId), ['first', 'second']);
    expect(reports[0], isA<WalletSendTransactionCreated>());
    expect(reports[1], isA<WalletSendNoTransaction>());
  });

  testWidgets('a declined host authorization reports NO transaction — the '
      'user was never asked to pay twice', (tester) async {
    final reports = <WalletSendReport>[];
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        authorizer: const _DenyingAuthorizer(),
      ),
    );
    await openSend(tester);
    await payThrough(tester); // lands back on Review, denied
    await tester.pageBack();
    await tester.pumpAndSettle();

    expect(reports.single, isA<WalletSendNoTransaction>());
  });

  testWidgets('a QUEUED send is not a broadcast one — no transaction, no txid, '
      'and a host that renders it as sent is lying', (tester) async {
    final reports = <WalletSendReport>[];
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        // Offline is what OFFERS the queue: the wallet cannot anchor a
        // proposal, so the user commits the intent instead.
        session:
            FakeWalletSession(
                current: const SyncStatus.offline(),
                snapshotValue: walletStateFixture(
                  syncStatus: const SyncStatus.offline(),
                  balance: balanceFixture(
                    spendableZat: 500000000,
                    totalZat: 500000000,
                  ),
                ),
              )
              // A DISTINCTIVE parked id, not the fake's default: the row below
              // asserts the core's own value travelled, and a default would
              // also match a hardcoded constant on the way out.
              ..queuedId = 'parked-7f3c',
      ),
    );
    await openSend(tester);
    await tester.tap(
      find.widgetWithText(OutlinedButton, l10n(tester).walletSendQueueButton),
    );
    await tester.pumpAndSettle();
    expect(find.text(l10n(tester).walletSendQueuedTitle), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    expect(reports.single, isA<WalletSendQueuedOffline>());
    expect(reports.single, isNot(isA<WalletSendTransactionCreated>()));
    // carried debt: the variant alone was the whole assertion, so "the
    // parked id rides on the report" was a claim with no pin. A queued send has
    // NO txid — this id is the only handle the host has onto a payment that is
    // already committed, and a null here silently strands it.
    expect(
      (reports.single as WalletSendQueuedOffline).queuedSendId,
      'parked-7f3c',
      reason:
          'the core ParkedSend id must reach the host, end to end — it is the '
          'only join key onto a committed send with no transaction yet',
    );
  });

  testWidgets('a DELIVERED terminal stops holding the txids — the root channel '
      'is not a place the previous identity\'s payments stay readable', (
    tester,
  ) async {
    // duress-hygiene debt. `sendFlowOutcomeProvider` deliberately watches
    // nothing so an identity flip cannot erase a report the host is owed — and
    // that same retention left the PREVIOUS identity's txids resident in a root
    // provider after the report had already been delivered. The send screen
    // clears its draft on a flip precisely so a coercer cannot read it.
    //
    // The fix is scoped to DELIVERY, not to the flip: once reported, drop it.
    final reports = <WalletSendReport>[];
    late ProviderContainer container;
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
      ),
    );
    container = ProviderScope.containerOf(
      tester.element(find.text('pay')),
      listen: false,
    );

    await openSend(tester);
    await payThrough(tester);
    await tester.pageBack();
    await tester.pumpAndSettle();

    // The host was told — that half must still hold.
    expect(reports.single, isA<WalletSendTransactionCreated>());
    expect(
      (reports.single as WalletSendTransactionCreated).txids,
      isNotEmpty,
      reason: 'the report itself still carries what the host is owed',
    );
    // …and the channel no longer holds them.
    expect(
      container.read(sendFlowOutcomeProvider),
      isNull,
      reason:
          'a DELIVERED terminal is dropped: the txids have done their job and '
          'must not outlive the screen in a root provider',
    );
  });

  testWidgets('leaving MID-FLIGHT is unclassified, never "nothing happened" — '
      'the sign+broadcast outlives the screen', (tester) async {
    final reports = <WalletSendReport>[];
    final hanging = Completer<void>();
    final session = newSession()..sendGate = hanging;
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        session: session,
      ),
    );
    await openSend(tester);
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendConfirmButton),
    );
    await tester.pump(); // into Submitting, and stay there
    // S13 §1a H2: Back is held while sending and asks; Leave is the exit.
    await tester.pageBack();
    // The busy spinner never settles; pump the dialog in.
    await tester.pump(const Duration(milliseconds: 500));
    expect(find.byType(SendScreen), findsOneWidget);
    expect(reports, isEmpty);
    await tester.tap(find.byKey(const ValueKey('send-leave-confirm')));
    await tester.pumpAndSettle();

    expect(reports.single, isA<WalletSendUnclassified>());
    expect(reports.single, isNot(isA<WalletSendNoTransaction>()));

    hanging.complete();
    await tester.pumpAndSettle();
    // The late landing does not rewrite an answer the host already has.
    expect(reports, hasLength(1));
  });

  testWidgets('a push stacked over an IN-FLIGHT send never attributes that '
      "send's outcome to the new request", (tester) async {
    final reports = <WalletSendReport>[];
    final hanging = Completer<void>();
    final session = newSession()..sendGate = hanging;
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(
            address: 'u1alice',
            amountZat: 150000,
            correlationId: 'first',
          ),
          WalletSendRequest(
            address: 'u1bob',
            amountZat: 250000,
            correlationId: 'second',
          ),
        ],
        reports: reports,
        session: session,
      ),
    );
    await openSend(tester);
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendConfirmButton),
    );
    await tester.pump(); // request "first" is submitting

    // The host pushes a SECOND payment over it (the wallet route is still
    // mounted underneath, so its context is live).
    unawaited(
      WalletSendEntry.push(
        tester.element(find.text('pay', skipOffstage: false)),
        const WalletSendRequest(
          address: 'u1bob',
          amountZat: 250000,
          correlationId: 'second',
        ),
      ).then(reports.add),
    );
    // Bounded pumps, never pumpAndSettle: the first send is still submitting
    // and its spinner would keep the frame scheduler busy forever.
    await tester.pump();
    await tester.pump(const Duration(seconds: 1)); // the route transition

    // The first send lands UNDER the second entry: its outcome belongs to
    // "first", and the deferred entry reset then clears to "second"'s form.
    hanging.complete();
    await tester.pump();
    await tester.pump();
    expect(
      reports.where((r) => r.correlationId == 'second'),
      isEmpty,
      reason: 'the second request has not finished — nothing to report yet',
    );

    await tester.pageBack(); // leave the second entry without paying
    await tester.pumpAndSettle();
    final second = reports.singleWhere((r) => r.correlationId == 'second');
    expect(
      second,
      isA<WalletSendNoTransaction>(),
      reason:
          'the first send\'s transaction must not be reported as the second '
          "request's payment",
    );
  });

  testWidgets('the payload carries ids and counts, never what the user typed', (
    tester,
  ) async {
    final reports = <WalletSendReport>[];
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(
            address: 'u1alice',
            amountZat: 150000,
            memo: 'rent for March',
            correlationId: 'row-9',
          ),
        ],
        reports: reports,
      ),
    );
    await openSend(tester);
    await payThrough(tester);
    await tester.pageBack();
    await tester.pumpAndSettle();

    final report = reports.single as WalletSendTransactionCreated;
    final payload = [...report.txids, report.correlationId].join('|');
    expect(payload, isNot(contains('u1alice')));
    expect(payload, isNot(contains('rent for March')));
    expect(payload, isNot(contains('150000')));
  });

  // ---------------------------------------------------------------------
  // The four rows the four-angle review earned. Every one of them was
  // reachable, and every one made the report LIE in the direction that costs
  // money. They are grouped so the shape stays visible: a report is a claim
  // about ONE flow, and the screen used to infer it from a GLOBAL controller.
  // ---------------------------------------------------------------------

  testWidgets('TWO IDLE STACKED ENTRIES: paying on the top one produces ONE '
      'report, to the top one. The covered screen must not book the payment '
      'it merely watched.', (tester) async {
    final reports = <WalletSendReport>[];
    final session = newSession();
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(
            address: 'u1alice',
            amountZat: 150000,
            correlationId: 'invoice-A',
          ),
          WalletSendRequest(
            address: 'u1bob',
            amountZat: 250000,
            correlationId: 'invoice-B',
          ),
        ],
        reports: reports,
        session: session,
      ),
    );
    // A opens and is left sitting on its form — no step started.
    await openSend(tester);
    // B stacks over it, from the (still mounted) wallet route below.
    unawaited(
      WalletSendEntry.push(
        tester.element(find.text('pay', skipOffstage: false)),
        const WalletSendRequest(
          address: 'u1bob',
          amountZat: 250000,
          correlationId: 'invoice-B',
        ),
      ).then(reports.add),
    );
    await tester.pumpAndSettle();
    // The user pays on B.
    await payThrough(tester);
    await tester.pageBack();
    await tester.pumpAndSettle();

    expect(session.sendCount, 1, reason: 'exactly one payment was made');
    final paid = reports.whereType<WalletSendTransactionCreated>().toList();
    expect(
      paid,
      hasLength(1),
      reason:
          'one transaction must yield one transaction-exists report — a second '
          'one lets the host book two payments for one txid',
    );
    expect(paid.single.correlationId, 'invoice-B');
  });

  testWidgets('A SESSION FLIP MID-SEND never reports "nothing was created" '
      'over money that moved — the controller drops the landing on purpose, '
      'so the report must not read that silence as an absence', (tester) async {
    final reports = <WalletSendReport>[];
    final hanging = Completer<void>();
    final fakeA = newSession()..sendGate = hanging;
    final fakeB = newSession();
    final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        sessionSwitch: sessionSwitch,
      ),
    );
    await openSend(tester);
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendConfirmButton),
    );
    await tester.pump(); // submitting

    // The host switches identity while the send is signing/broadcasting.
    final element = tester.element(find.byType(SendScreen));
    ProviderScope.containerOf(element).read(sessionSwitch.notifier).state =
        fakeB;
    await tester.pump();
    // …and the send lands afterwards. confirm()'s identity guard correctly
    // swallows the STATE write; the money still moved.
    hanging.complete();
    await tester.pump();
    await tester.pump();
    await tester.pageBack();
    await tester.pumpAndSettle();

    // A SEPARATE, PRE-EXISTING defect this row walks through, consumed here so
    // it does not mask the assertion below. `WalletDisplaySyncStatusNotifier`
    // adopts a raw status into its own state while the send screen is first
    // watching it during a BUILD, which trips riverpod's
    // "modified a provider while the widget tree was building". PROBED, not
    // assumed: a bare `SendScreen` with no entry, no reporter and no FR-26 path
    // reproduces it on the same flip-mid-send sequence. It is owed its own fix
    // in `wallet_display_sync_status.dart`; swallowing it silently here would
    // be how it stays unfixed, so it is named.
    expect(
      tester.takeException().toString(),
      contains('Tried to modify a provider while the widget tree was building'),
      reason:
          'if this stops firing the pre-existing defect was fixed — delete '
          'this expectation, do not weaken the row below',
    );

    expect(fakeA.sendCount, 1, reason: 'the send really ran');
    expect(
      reports.single,
      isNot(isA<WalletSendNoTransaction>()),
      reason:
          '"nothing was created" over a signed, broadcast transaction tells '
          'the user to pay again',
    );
  });

  testWidgets('A LATE THROW after the send succeeded is not "nothing was '
      'created" — the host authorizer wraps the action and can throw after '
      'awaiting it', (tester) async {
    final reports = <WalletSendReport>[];
    final session = newSession();
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        session: session,
        authorizer: const _ThrowsAfterActionAuthorizer(),
      ),
    );
    await openSend(tester);
    await payThrough(tester);
    await tester.pageBack();
    await tester.pumpAndSettle();

    expect(session.sendCount, 1, reason: 'the send really ran');
    expect(
      reports.single,
      isNot(isA<WalletSendNoTransaction>()),
      reason:
          'the transaction was signed, persisted and broadcast before the '
          "host's own bookkeeping threw",
    );
  });

  testWidgets('AN IN-PLACE ROUTE UPDATE WITH THE SAME const REQUEST still '
      'completes the first future — two identical const literals ARE the same '
      'object, so identity on the request is not a liveness signal', (
    tester,
  ) async {
    const request = WalletSendRequest(
      address: 'u1alice',
      amountZat: 150000,
      correlationId: 'first',
    );
    final reports = <WalletSendReport>[];
    await tester.pumpWidget(
      harness(requests: const [request], reports: reports),
    );
    await openSend(tester);
    // A host replaces the send route in place, with the SAME request value.
    lastRouter.replace(WalletRoutes.send, extra: request);
    await tester.pumpAndSettle();
    await tester.pageBack();
    await tester.pumpAndSettle();

    expect(
      reports,
      hasLength(1),
      reason: 'the first push must not await forever on a money surface',
    );
    expect(reports.single.correlationId, 'first');
  });

  testWidgets(
    'A DENIED AUTHORIZATION LEAVES THE FLOW SPENDABLE, so the channel '
    'must NOT close — the next Confirm pays, and a host holding "nothing was '
    'created" would double-pay on the re-request',
    (tester) async {
      final reports = <WalletSendReport>[];
      final session = newSession();
      final authorizer = _DeniesOnceAuthorizer();
      await tester.pumpWidget(
        harness(
          requests: const [
            WalletSendRequest(address: 'u1alice', amountZat: 150000),
          ],
          reports: reports,
          session: session,
          authorizer: authorizer,
        ),
      );
      await openSend(tester);
      await payThrough(tester); // denied — back to a LIVE review
      expect(session.sendCount, 0);
      expect(
        reports,
        isEmpty,
        reason:
            'the flow is not over: publishing a terminal here consumed the '
            'one-shot channel and made the next payment unreportable',
      );

      // The user taps Confirm again. This time it goes through.
      await tester.tap(
        find.widgetWithText(FilledButton, l10n(tester).walletSendConfirmButton),
      );
      await tester.pumpAndSettle();
      await tester.pageBack();
      await tester.pumpAndSettle();

      expect(session.sendCount, 1, reason: 'the retry really paid');
      expect(
        reports.single,
        isA<WalletSendTransactionCreated>(),
        reason: 'the payment the user actually made is the one reported',
      );
    },
  );

  testWidgets('A REPORTED entry offers no way to spend again — "Try again" is '
      'gated like "Send another", because a retry after a delivered report is '
      'a payment the host can never learn about', (tester) async {
    final reports = <WalletSendReport>[];
    final session = newSession()
      ..sendThrows = const WalletApiError(
        code: 'RW-SEND-001',
        message: 'sign failed',
        kind: WalletErrorKind.proposeFailed(),
      );
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        session: session,
      ),
    );
    await openSend(tester);
    await payThrough(tester); // sign failed → the result screen

    expect(find.text(l10n(tester).walletSendTryAgain), findsNothing);
  });

  testWidgets('a fabricated authorizer return cannot invent a payment — the '
      'controller reads the spend it ran, not what the host handed back', (
    tester,
  ) async {
    final reports = <WalletSendReport>[];
    final session = newSession();
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        session: session,
        authorizer: const _FabricatingAuthorizer(),
      ),
    );
    await openSend(tester);
    await payThrough(tester);
    await tester.pageBack();
    await tester.pumpAndSettle();

    expect(session.sendCount, 0, reason: 'the closure was never run');
    expect(
      reports.single,
      isNot(isA<WalletSendTransactionCreated>()),
      reason:
          'a host returning invented TxSubmitResults must not make the wallet '
          'report txids for a payment that never happened',
    );
  });

  testWidgets('the route builder still accepts a bare request (a host that '
      'navigates by path gets no report, and no crash)', (tester) async {
    final router = GoRouter(
      initialLocation: WalletRoutes.wallet,
      routes: [
        GoRoute(
          path: WalletRoutes.wallet,
          builder: (context, state) => Scaffold(
            body: Center(
              child: TextButton(
                onPressed: () => context.push(
                  WalletRoutes.send,
                  extra: const WalletSendRequest(address: 'u1alice'),
                ),
                child: const Text('pay'),
              ),
            ),
          ),
        ),
        ...walletRoutes().where(
          (r) => r is GoRoute && r.path != WalletRoutes.wallet,
        ),
      ],
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [walletSessionProvider.overrideWithValue(newSession())],
        child: MaterialApp.router(
          theme: lightTheme,
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          routerConfig: router,
        ),
      ),
    );
    await tester.tap(find.text('pay'));
    await tester.pumpAndSettle();
    expect(find.byType(SendScreen), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  // The 2026-09-20 review's R05 probe (audit-2026-09-20-remediation.md §2a/b;
  // body from docs/reviews/2026-09-20/probes/send_entry.dart), RED BY RULING
  // GREEN since stage S8 `deadline` (ADR-0557): the entry's
  // mount grace REVOKES the request before it reports NoTransaction, so the
  // route that mounts late renders the expired state and cannot pay. The last
  // assertions once asserted the base tree's witness (a payment under the
  // negative report: sendCount 1, a TransactionCreated report); they now
  // assert contract §3.3 row 1 — the pay path reaches nothing, sendCount 0,
  // the expired title, the report stays NoTransaction — the claim met by
  // making the request unable to spend. Appended at the end of main().
  testWidgets(
    'R05 — a send route that mounts after the timeout cannot pay under a "no transaction" report',
    (tester) async {
      final releaseRedirect = Completer<String?>();
      final reports = <WalletSendReport>[];
      final session = newSession();
      final router = GoRouter(
        initialLocation: WalletRoutes.wallet,
        redirect: (context, state) {
          if (state.uri.path == WalletRoutes.send &&
              !releaseRedirect.isCompleted) {
            return releaseRedirect.future;
          }
          return null;
        },
        routes: [
          GoRoute(
            path: WalletRoutes.wallet,
            builder: (context, state) => Scaffold(
              body: Center(
                child: TextButton(
                  onPressed: () async {
                    reports.add(
                      await WalletSendEntry.push(
                        context,
                        const WalletSendRequest(
                          address: 'u1alice',
                          amountZat: 150000,
                        ),
                      ),
                    );
                  },
                  child: const Text('review delayed pay'),
                ),
              ),
            ),
          ),
          ...walletRoutes().where(
            (route) => route is GoRoute && route.path != WalletRoutes.wallet,
          ),
        ],
      );
      addTearDown(router.dispose);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(session)],
          child: MaterialApp.router(
            theme: lightTheme,
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            routerConfig: router,
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('review delayed pay'));
      await tester.pump();
      await tester.pump(const Duration(seconds: 6));
      await tester.pump();
      expect(find.byType(SendScreen), findsNothing);
      final earlyReport = reports.isEmpty
          ? 'pending'
          : reports.single.runtimeType.toString();
      releaseRedirect.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);
      // Stage S8 `deadline`, arm (a) — plan §3.3 row 1, the probe's honest
      // form: the mount grace REVOKES, so the late screen says the request
      // expired and offers no pay path, the money never moves, and the
      // negative the host already holds stays the true and only answer.
      // The pay path is ATTEMPTED (whatever the screen offers is tapped) and
      // then asserted on the MONEY, the SCREEN and the REPORT — never the
      // screen alone — so a tree that still pays reds here on the R05 witness
      // itself, `sendCount` 1 under a "no transaction" report. (The
      // orchestrator's edit, delegated to the adjudicator at the join; the
      // lines it replaced asserted that witness as the expectation, which no
      // conforming build can satisfy.)
      final review = find.widgetWithText(
        FilledButton,
        l10n(tester).walletSendReviewButton,
      );
      if (review.evaluate().isNotEmpty) await payThrough(tester);
      expect(
        session.sendCount,
        0,
        reason:
            'the host holds "no transaction" for this request — a payment now '
            'is money on chain with no host record (R05)',
      );
      expect(
        find.text(l10n(tester).walletSendExpiredTitle),
        findsOneWidget,
        reason: 'the late screen states that the request expired',
      );
      expect(
        review,
        findsNothing,
        reason: 'an expired request offers no Review — the pay path is gone',
      );
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(reports, hasLength(1));
      expect(
        reports.single,
        isA<WalletSendNoTransaction>(),
        reason:
            'The negative report is final: nothing this request signed was '
            'broadcast. Before mount=$earlyReport; sendCount=${session.sendCount}',
      );
    },
  );

  // ── S13 §1.2 + revision 2 (B1, H1, H2): no leaving while sending ──────────

  Future<void> toSubmittingFromForm(WidgetTester tester) async {
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendReviewButton),
    );
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.text(l10n(tester).walletSendConfirmButton),
      200,
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.widgetWithText(FilledButton, l10n(tester).walletSendConfirmButton),
    );
    await tester.pump(); // into Submitting
  }

  Future<void> toSubmitting(WidgetTester tester) async {
    await openSend(tester);
    await toSubmittingFromForm(tester);
  }

  testWidgets('S13 H1: a host pop during the authorizer prompt, then the '
      'approval — nothing is sent, and the host was told "no transaction"', (
    tester,
  ) async {
    final reports = <WalletSendReport>[];
    final session = newSession();
    final prompt = _GatedAuthorizer();
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        session: session,
        authorizer: prompt,
      ),
    );
    await toSubmitting(tester);
    expect(prompt.asked, 1, reason: 'the prompt is up, nothing entered yet');

    // The host pops programmatically — the exit the Back hold cannot stop.
    Navigator.of(tester.element(find.byType(SendScreen))).pop();
    await tester.pumpAndSettle();
    expect(find.byType(SendScreen), findsNothing);
    expect(reports.single, isA<WalletSendNoTransaction>());

    // The user approves at the host's prompt AFTER the screen is gone.
    prompt.approve();
    await tester.pumpAndSettle();
    expect(
      session.sendCount,
      0,
      reason:
          '"no transaction" must be structurally true: the abandoned '
          "flow's spend refuses before it enters",
    );
    expect(reports, hasLength(1));
  });

  testWidgets('S13 B1 and H2: Back and maybePop during SendSubmitting do NOT '
      'leave and do not report; Stay stays; Leave leaves and reports '
      'honestly', (tester) async {
    final reports = <WalletSendReport>[];
    final hanging = Completer<void>();
    final session = newSession()..sendGate = hanging;
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        session: session,
      ),
    );
    await toSubmitting(tester);
    expect(session.sendCount, 1, reason: 'the send is in flight');

    // maybePop is what system Back runs.
    final screen = tester.element(find.byType(SendScreen));
    unawaited(Navigator.of(screen).maybePop());
    await tester.pump(const Duration(milliseconds: 500));
    expect(find.byType(SendScreen), findsOneWidget);
    expect(reports, isEmpty);
    expect(find.text(l10n(tester).walletSendLeaveTitle), findsOneWidget);
    expect(find.text(l10n(tester).walletSendLeaveBody), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('send-leave-stay')));
    await tester.pump(const Duration(milliseconds: 500));
    expect(find.byType(SendScreen), findsOneWidget);
    expect(find.text(l10n(tester).walletSendLeaveTitle), findsNothing);
    expect(reports, isEmpty);

    await tester.pageBack();
    await tester.pump(const Duration(milliseconds: 500));
    await tester.tap(find.byKey(const ValueKey('send-leave-confirm')));
    // Pumped in steps: if Leave did NOT leave, the busy spinner never settles,
    // and the row must fail on the assertion below, not on a settle timeout.
    for (var i = 0; i < 6; i++) {
      await tester.pump(const Duration(milliseconds: 300));
    }
    expect(find.byType(SendScreen), findsNothing, reason: 'Leave leaves');
    expect(
      reports.single,
      isA<WalletSendUnclassified>(),
      reason: 'the spend entered: its outcome outlives the screen',
    );
    hanging.complete();
    await tester.pumpAndSettle();
  });

  testWidgets('S13: after SendSent a Back pop works, with no confirm', (
    tester,
  ) async {
    final reports = <WalletSendReport>[];
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
      ),
    );
    await toSubmitting(tester);
    await tester.pumpAndSettle();
    expect(find.text(l10n(tester).walletSendSentTitle), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.byType(SendScreen), findsNothing);
    expect(reports.single, isA<WalletSendTransactionCreated>());
  });

  testWidgets('S13 H1, the in-place door: a host replace during the prompt '
      'answers "no transaction", and the late approval sends nothing (the '
      'S303 diff review HIGH)', (tester) async {
    final reports = <WalletSendReport>[];
    final session = newSession();
    final prompt = _GatedAuthorizer();
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        session: session,
        authorizer: prompt,
      ),
    );
    await toSubmitting(tester);
    expect(prompt.asked, 1);

    // A deep link replaces the send route in place (the channel goes).
    lastRouter.replace(
      WalletRoutes.send,
      extra: const WalletSendRequest(address: 'u1bob', amountZat: 1),
    );
    await tester.pump();
    await tester.pump();
    expect(reports.single, isA<WalletSendNoTransaction>());

    prompt.approve();
    await tester.pumpAndSettle();
    expect(session.sendCount, 0, reason: 'the told-"none" flow cannot pay');
    expect(reports, hasLength(1));
  });

  testWidgets('S13 H1 after an identity switch: the flow the NEW identity '
      'drives is abandoned with the screen too (the S303 diff review '
      'MEDIUM)', (tester) async {
    final reports = <WalletSendReport>[];
    final fakeA = newSession();
    final fakeB = newSession();
    final sessionSwitch = StateProvider<WalletSession?>((ref) => fakeA);
    final prompt = _GatedAuthorizer();
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        sessionSwitch: sessionSwitch,
        authorizer: prompt,
      ),
    );
    await openSend(tester);
    ProviderScope.containerOf(
      tester.element(find.byType(SendScreen)),
    ).read(sessionSwitch.notifier).state = fakeB;
    await tester.pumpAndSettle();
    // The flip cleared the draft; the user types a payment on identity B.
    await tester.enterText(find.byType(TextField).at(0), 'u1carol');
    await tester.enterText(find.byType(TextField).at(1), '1');
    await tester.pump();
    await toSubmittingFromForm(tester);
    expect(prompt.asked, 1);

    Navigator.of(tester.element(find.byType(SendScreen))).pop();
    await tester.pumpAndSettle();
    prompt.approve();
    await tester.pumpAndSettle();
    expect(fakeB.sendCount, 0, reason: 'a gone screen cannot show it');
    expect(fakeA.sendCount, 0);
  });

  testWidgets('S13: after its flow is abandoned elsewhere, a host entry that '
      'reviews again and pays tells its host the payment was made — the '
      'claim moves to the flow actually paid through', (tester) async {
    final reports = <WalletSendReport>[];
    final session = newSession();
    await tester.pumpWidget(
      harness(
        requests: const [
          WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ],
        reports: reports,
        session: session,
      ),
    );
    await openSend(tester);
    // Another screen that shared the controller went away with this flow
    // (the revealed-under-a-stacked-screen shape).
    final c = ProviderScope.containerOf(
      tester.element(find.byType(SendScreen)),
    );
    final notifier = c.read(sendControllerProvider.notifier);
    notifier.abandonFlow(notifier.flowId);

    // Review → Confirm: refused before any prompt, back to the form.
    await toSubmittingFromForm(tester);
    await tester.pumpAndSettle();
    expect(session.sendCount, 0);
    expect(
      find.text(l10n(tester).walletSendFaultAmountsExpired),
      findsOneWidget,
    );

    // The user reviews again and pays.
    await toSubmittingFromForm(tester);
    await tester.pumpAndSettle();
    expect(session.sendCount, 1);
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(
      reports.single,
      isA<WalletSendTransactionCreated>(),
      reason: 'never "no transaction" for a request this screen paid',
    );
  });
}

/// Holds the host's prompt open until [approve], then runs the spend — the
/// window in which a host can pop the screen (S13 H1).
class _GatedAuthorizer implements WalletSendAuthorizer {
  final Completer<void> _approved = Completer<void>();
  int asked = 0;

  void approve() => _approved.complete();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    asked += 1;
    await _approved.future;
    return action();
  }
}

class _DenyingAuthorizer implements WalletSendAuthorizer {
  const _DenyingAuthorizer();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) => Future<T>.error(const WalletSpendAuthorizationDenied());
}

/// Declines the FIRST prompt only — the commonest real interaction (a
/// cancelled biometric), after which the review is live again.
class _DeniesOnceAuthorizer implements WalletSendAuthorizer {
  int calls = 0;

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) {
    calls += 1;
    if (calls == 1) {
      return Future<T>.error(const WalletSpendAuthorizationDenied());
    }
    return action();
  }
}

/// Never runs the spend, and returns an invented result as if it had.
class _FabricatingAuthorizer implements WalletSendAuthorizer {
  const _FabricatingAuthorizer();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async =>
      const <TxSubmitResult>[TxSubmitResult.success(txidHex: 'facade')] as T;
}

/// A host authorizer that runs the spend and THEN fails — its own bookkeeping,
/// a rethrow, anything. The money moved; the throw arrives after it.
class _ThrowsAfterActionAuthorizer implements WalletSendAuthorizer {
  const _ThrowsAfterActionAuthorizer();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    await action();
    throw StateError('host bookkeeping failed after the spend');
  }
}
