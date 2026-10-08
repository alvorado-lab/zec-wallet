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
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/src/send/wallet_send_channel.dart';
import 'package:zec_wallet_ui/testing.dart';

/// Stage S8 `deadline` (R05) — the rows that name what the contract DESCRIBES
/// but the base tree does not have (plan §3.3 "The seam": a revoke verb on the
/// channel, the revoked-request bit the controller reads with no screen in the
/// tree, the "request expired" state's copy). They cannot compile at the base,
/// so they live apart from `wallet_send_deadline_test.dart`, which runs red
/// there; the adjudicator joins the three names below to the implementer's.
///
/// The names are the contract's words, and each is used in exactly ONE place:
/// - `WalletSendReporter.isRevoked` / `revoke()` — the channel's revoke bit;
/// - [revokeOn] — how a revoked request reaches the controller directly;
/// - `WalletLocalizations.walletSendExpiredBody` — the expired state's copy
///   (joined from the blind name `walletSendRequestExpired`: the implementer
///   split the state into a title and a body, and the body is the sentence
///   that states the limit).
void main() {
  final upToDate = walletStateFixture(
    syncStatus: const SyncStatus.upToDate(tip: 1),
    balance: balanceFixture(spendableZat: 500000000, totalZat: 500000000),
  );

  FakeWalletSession newSession() => FakeWalletSession(
    current: const SyncStatus.upToDate(tip: 1),
    snapshotValue: upToDate,
  );

  const request = WalletSendRequest(
    address: 'u1alice',
    amountZat: 150000,
    correlationId: 'inv-1',
  );

  /// THE SEAM THIS FILE CANNOT SEE. The contract names the bit (a revoked
  /// request, as the screen holds it) and where the controller reads it
  /// (`confirm()` and `queueOffline()`, before the authorizer), but not the
  /// verb that sets it when no screen exists to carry it. This is the ONE line
  /// the adjudicator joins to the implementer's mechanism; every assertion in
  /// the row that calls it stands as written.
  ///
  /// Joined (S8 deadline adjudication): the implementer's verb is the entry
  /// reset itself — the screen hands the request's bit to the controller at
  /// every `resetToForm`, so with no screen in the tree the row binds it the
  /// same way. The request is what the SCREEN compares (`sameSendRequest`);
  /// the controller holds only the bit.
  void revokeOn(SendController controller, WalletSendRequest request) =>
      controller.resetToForm(requestRevoked: true);

  // ---------------------------------------------------------------------
  // The channel: "reported" and "revoked" are two bits.
  // ---------------------------------------------------------------------

  test('the channel carries the revoke bit apart from the reported bit', () {
    final reported = WalletSendReporter('inv-1')
      ..report(const WalletSendNoTransaction(correlationId: 'inv-1'));
    expect(reported.isReported, isTrue);
    expect(
      reported.isRevoked,
      isFalse,
      reason:
          'a retired channel (_closeIdleClaims) and a teardown report both '
          'answer "no transaction" WITHOUT revoking — the flow they graded '
          'was never spendable through them, and a revoke here would gate '
          'row 687\'s denied-then-paid flow',
    );

    final revoked = WalletSendReporter('inv-1')..revoke();
    expect(revoked.isRevoked, isTrue);
    expect(
      revoked.isReported,
      isFalse,
      reason: 'revoking is not reporting: the grace reports separately, after',
    );
  });

  // ---------------------------------------------------------------------
  // Row 2 — THROUGH THE CONTROLLER, NO WIDGET IN THE TREE (the contract's
  // literal shape; the harness precedent is shield_controller_test.dart).
  // ---------------------------------------------------------------------

  test(
    'a revoked request is refused by the controller before the authorizer is '
    'asked — confirm and queueOffline (no widget in the tree)',
    () async {
      final fake = newSession();
      final auth = FakeSendAuthorizer();
      final container = ProviderContainer(
        overrides: [
          walletSessionProvider.overrideWithValue(fake),
          walletSendAuthorizerProvider.overrideWithValue(auth),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(sendControllerProvider.notifier);
      final seen = <SendState>[];
      final watch = container.listen(
        sendControllerProvider,
        (_, next) => seen.add(next),
        fireImmediately: true,
      );
      addTearDown(watch.close);

      revokeOn(controller, request);
      final flowId = controller.flowId;

      // The online door.
      await controller.prepare(address: 'u1alice', amountText: '0.0015');
      await controller.confirm();
      // The offline door (the Queue button skips Review).
      await controller.queueOffline(address: 'u1alice', amountText: '0.0015');

      expect(
        auth.intents,
        isEmpty,
        reason:
            "the refusal precedes the host's authorizer — its bracket must "
            'not learn about the entry (Relim D4)',
      );
      expect(controller.hasEnteredSpend(flowId), isFalse);
      expect(fake.sendCount, 0);
      expect(fake.queueCount, 0);
      expect(seen.whereType<SendSubmitting>(), isEmpty);
      expect(seen.whereType<SendQueuing>(), isEmpty);
    },
  );

  // ---------------------------------------------------------------------
  // Rows 1 and 4, the clauses the blind file could not state: the expired
  // state's COPY, and the channel-level revoke under a redirect that never
  // releases.
  // ---------------------------------------------------------------------

  /// The FR-26 harness with a held redirect that also CAPTURES the entry's
  /// route argument as the redirect sees it — the one handle on the channel a
  /// test can hold when no screen ever mounts.
  Widget harness({
    required List<WalletSendReport> reports,
    required Completer<String?> hold,
    required List<WalletSendEntryArgs> seenArgs,
    required FakeWalletSession session,
  }) {
    final router = GoRouter(
      initialLocation: WalletRoutes.wallet,
      redirect: (context, state) {
        if (state.uri.path == WalletRoutes.send) {
          final extra = state.extra;
          if (extra is WalletSendEntryArgs && !seenArgs.contains(extra)) {
            seenArgs.add(extra);
          }
          if (!hold.isCompleted) return hold.future;
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
    addTearDown(router.dispose);
    return ProviderScope(
      overrides: [walletSessionProvider.overrideWithValue(session)],
      child: MaterialApp.router(
        theme: lightTheme,
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        routerConfig: router,
      ),
    );
  }

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
    expect(reports.single, isA<WalletSendNoTransaction>());
  }

  testWidgets(
    'a redirect held past the grace then released shows the request expired '
    "in the screen's own copy, and the copy states the five-second limit",
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final seenArgs = <WalletSendEntryArgs>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(
          reports: reports,
          hold: hold,
          seenArgs: seenArgs,
          session: session,
        ),
      );
      await pushPastTheGrace(tester, reports);
      hold.complete(null);
      await tester.pumpAndSettle();
      expect(find.byType(SendScreen), findsOneWidget);

      final l10n = WalletLocalizations.of(
        tester.element(find.byType(SendScreen)),
      );
      final copy = l10n.walletSendExpiredBody;
      expect(find.text(copy), findsOneWidget);
      // Plan §3.3: "the copy states the limit: a phone that took more than
      // five seconds to open the send screen" — the number, in English.
      expect(
        copy,
        matches(RegExp(r'\b(five|5)\b.*\bseconds?\b', caseSensitive: false)),
        reason:
            'the user is told what the limit was, so the restart makes sense',
      );
      expect(session.sendCount, 0);
    },
  );

  testWidgets(
    'a redirect that never releases revokes the request on its channel — '
    'not merely reports it',
    (tester) async {
      final hold = Completer<String?>();
      final reports = <WalletSendReport>[];
      final seenArgs = <WalletSendEntryArgs>[];
      final session = newSession();
      await tester.pumpWidget(
        harness(
          reports: reports,
          hold: hold,
          seenArgs: seenArgs,
          session: session,
        ),
      );
      await pushPastTheGrace(tester, reports);

      final args = seenArgs.single;
      expect(identical(args.request, request), isTrue);
      expect(args.reporter.isReported, isTrue);
      expect(
        args.reporter.isRevoked,
        isTrue,
        reason:
            'the grace REVOKES before it reports: a screen that mounts for '
            'this channel later must find the request unable to spend',
      );
      expect(find.byType(SendScreen), findsNothing);
      expect(session.sendCount, 0);
    },
  );
}
