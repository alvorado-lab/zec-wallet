import 'dart:async';
// `intl` exports its OWN TextDirection, so the framework's is aliased here.
import 'dart:ui' as ui show TextDirection;
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/intl.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/in_flight_sends_section.dart';
import 'package:zec_wallet_ui/features/wallet/send_authorization.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/features/wallet/zat_format.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// Resolve l10n from a live element so finders couple to KEYS, not literals.
WalletLocalizations _l10nAt(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(WalletScreen)));

/// The row's save-time component, computed through the SAME format + local-TZ
/// path the widget uses (deterministic within a run regardless of machine TZ).
String _timeOf(int unixSecs) => DateFormat.MMMd().add_jm().format(
  DateTime.fromMillisecondsSinceEpoch(unixSecs * 1000).toLocal(),
);

Widget _harness(WalletSession session) {
  return ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session)],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: const WalletScreen(),
    ),
  );
}

/// A writable stand-in for the host's session, so a test can FLIP the wallet
/// identity mid-bracket (#400 R8). `walletSessionProvider` is a plain `Provider`
/// with no setter; overriding it to watch this slot is the only way to model the
/// thing the fence exists for.
final _sessionSlot = NotifierProvider<_SessionSlot, WalletSession?>(
  _SessionSlot.new,
);

/// Seeds [_SessionSlot.build] — set by [_flipHarness] just before the pump, so the
/// first frame already has the right identity (a null-session first frame would
/// render onboarding instead of the wallet).
WalletSession? _initialFlipSession;

class _SessionSlot extends Notifier<WalletSession?> {
  @override
  WalletSession? build() => _initialFlipSession;

  void swap(WalletSession? session) => state = session;
}

/// [_harness] whose session comes from [_sessionSlot] and whose authorizer can
/// flip it — before the action (the pre-action fence) or after it (the delivery
/// re-check, #400 R6).
Widget _flipHarness(WalletSession initial, WalletSendAuthorizer authorizer) {
  _initialFlipSession = initial;
  return ProviderScope(
    overrides: [
      walletSessionProvider.overrideWith((ref) => ref.watch(_sessionSlot)),
      walletSendAuthorizerProvider.overrideWithValue(authorizer),
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: const WalletScreen(),
    ),
  );
}

/// [_harness] pinned at a fixed text scale — the a11y edge case (a low-vision
/// user at 2–3×). Overrides the scaler for the whole tree via the app builder.
Widget _harnessScaled(WalletSession session, double scale) {
  return ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session)],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(
          context,
        ).copyWith(textScaler: TextScaler.linear(scale)),
        child: child!,
      ),
      home: const WalletScreen(),
    ),
  );
}

/// A fake in the ACTIVE (up-to-date) wallet state, so the parked surface + the
/// recover affordance render. Override the parked / recoverable knobs per case.
FakeWalletSession _activeFake() {
  return FakeWalletSession(
    current: const SyncStatus.upToDate(tip: 100),
    snapshotValue: walletStateFixture(
      syncStatus: const SyncStatus.upToDate(tip: 100),
      balance: balanceFixture(
        spendableZat: 1000000,
        totalZat: 1000000,
        transparentZat: 1000000,
      ),
    ),
  );
}

void main() {
  // A phone-tall viewport (FR-49 S10). The home body is a lazy ListView, and
  // the refreshed type scale (bodyLarge 16, bodyMedium 14) pushed the parked
  // and recover sections past the default 800×600 surface's build range.
  // These rows assert CONTENT and behaviour, not the fold. Only the HEIGHT
  // grows: the width stays 800, so the 2–3× text-scale cases still overflow a
  // row that cannot fit, which is what they pin.
  setUp(() {
    final view =
        TestWidgetsFlutterBinding.instance.platformDispatcher.views.first;
    view.physicalSize = const Size(800, 1200) * view.devicePixelRatio;
  });
  tearDown(() {
    TestWidgetsFlutterBinding.instance.platformDispatcher.views.first
        .resetPhysicalSize();
  });
  // #401 R6 — registered first so the new pins run alongside the existing groups.
  _main401();
  group('parked "saved & pending" surface (2e-2b-v-4b)', () {
    testWidgets('renders a row with the committed amount (no recipient)', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(amountZat: 70000)];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      expect(find.text(l10n.walletParkedTitle), findsOneWidget);
      expect(
        find.text(
          l10n.walletParkedRowTimed(
            l10n.walletAmount(formatZec(70000)),
            _timeOf(1700000000),
          ),
        ),
        findsOneWidget,
      );
      // with the sync policy at its default ON, the sync-off money
      // note does NOT render (the counterfactual for the policy-off pin).
      expect(
        find.byKey(const ValueKey('wallet-parked-sync-off')),
        findsNothing,
      );
    });

    testWidgets('S205-b: the sync-off pause note renders under the subtitle '
        'when the host\'s sync policy is off — parked sends drain only on '
        'sync passes, so "haven\'t been sent YET" alone would imply a '
        'progress that cannot happen', (tester) async {
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(amountZat: 70000)];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSyncPolicyProvider.overrideWithValue(false),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      // The home body is a lazy ListView and the (longer) sync-off Send
      // reason pushes the parked section past the initial build range —
      // DRIVE the scrollable until the note is built (the file's
      // scrollUntilVisible idiom).
      final note = find.byKey(const ValueKey('wallet-parked-sync-off'));
      await tester.scrollUntilVisible(
        note,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      expect(note, findsOneWidget);
      expect(
        tester.widget<Text>(note).data,
        l10n.walletParkedSyncPausedNote,
        reason:
            'the note says the pause plainly, never a failure claim — and '
            '(#401 R2d) names the per-row escape, because Send now keeps '
            'working while the queue is frozen',
      );
      // The section itself still renders normally around it.
      expect(find.text(l10n.walletParkedTitle), findsOneWidget);
    });

    testWidgets(
      'a queued SINGLE-STEP send renders on the same surface — visible + '
      'cancellable from a cold read (#331: invisible, it was a double-pay '
      'window across an offline relaunch)',
      (tester) async {
        // The relaunch shape: a fresh widget tree cold-pulls the parked list
        // (no sync edge — queueing happens offline by design) and the
        // plainly-queued send MUST be there, rendered through the SAME honest
        // kind-agnostic copy, next to a one-time-address row.
        final fake = _activeFake()
          ..parkedSendsResult = [
            parkedSendFixture(
              id: 3,
              kind: ParkedSendKind.singleStep,
              amountZat: 40000,
              createdAt: 1700000456,
            ),
            parkedSendFixture(id: 7, amountZat: 70000),
          ];
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        expect(find.text(l10n.walletParkedTitle), findsOneWidget);
        expect(
          find.text(
            l10n.walletParkedRowTimed(
              l10n.walletAmount(formatZec(40000)),
              _timeOf(1700000456),
            ),
          ),
          findsOneWidget,
          reason: 'the single-step row is VISIBLE',
        );
        expect(
          find.text(
            l10n.walletParkedRowTimed(
              l10n.walletAmount(formatZec(70000)),
              _timeOf(1700000000),
            ),
          ),
          findsOneWidget,
          reason: 'both kinds share the surface',
        );

        // ... and cancellable — the escape hatch that closes the window. Two
        // rows ⇒ two Cancel buttons; the FIRST is the single-step row's.
        final cancelBtns = find.widgetWithText(
          TextButton,
          l10n.walletParkedCancel,
        );
        expect(cancelBtns, findsNWidgets(2));
        await tester.ensureVisible(cancelBtns.first);
        await tester.pumpAndSettle();
        await tester.tap(cancelBtns.first);
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletParkedCancelConfirmDiscard));
        await tester.pumpAndSettle();
        expect(fake.cancelParkedSendCount, 1);
        expect(fake.lastCancelId, 3, reason: 'the single-step row cancelled');
        expect(fake.lastCancelCreatedAt, 1700000456);
        await tester.pumpAndSettle(
          const Duration(seconds: 5),
        ); // drain snackbar
      },
    );

    testWidgets('hidden entirely when there are no parked sends', (
      tester,
    ) async {
      final fake = _activeFake()..parkedSendsResult = const [];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(find.text(l10n.walletParkedTitle), findsNothing);
    });

    testWidgets('a read failure surfaces an honest line, never a silent-hide', (
      tester,
    ) async {
      final fake = _activeFake()..listParkedSendsThrows = StateError('boom');
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(find.text(l10n.walletParkedError), findsOneWidget);
      expect(find.text(l10n.walletParkedTitle), findsNothing);
    });

    testWidgets(
      'the read-error line carries an inline retry that re-pulls in place — '
      'the home has no pull-to-refresh, so this is the only recovery short of a '
      'full app resume (S199-c UX+reliability MED)',
      (tester) async {
        final fake = _activeFake()..listParkedSendsThrows = StateError('boom');
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        // The honest error line AND an inline "Try again" (not a dead end).
        expect(find.text(l10n.walletParkedError), findsOneWidget);
        final retry = find.byKey(const Key('parked-error-retry'));
        expect(retry, findsOneWidget);
        expect(
          find.descendant(
            of: retry,
            matching: find.text(l10n.walletParkedErrorRetry),
          ),
          findsOneWidget,
        );
        final readsBefore = fake.listParkedSendsCount;

        // The transient fault clears; tapping retry re-pulls the identity-scoped
        // reader and the section recovers to data IN PLACE (error gone, row shown).
        fake
          ..listParkedSendsThrows = null
          ..parkedSendsResult = [parkedSendFixture(amountZat: 70000)];
        await tester.ensureVisible(retry);
        await tester.pumpAndSettle();
        await tester.tap(retry);
        await tester.pumpAndSettle();

        expect(
          fake.listParkedSendsCount,
          greaterThan(readsBefore),
          reason: 'the retry re-pulled the identity-scoped parked reader',
        );
        expect(find.text(l10n.walletParkedError), findsNothing);
        expect(find.text(l10n.walletParkedTitle), findsOneWidget);
        expect(
          find.text(
            l10n.walletParkedRowTimed(
              l10n.walletAmount(formatZec(70000)),
              _timeOf(1700000000),
            ),
          ),
          findsOneWidget,
          reason: 'the recovered read renders the pending payment in place',
        );
      },
    );

    testWidgets(
      'retry on a PERSISTENT fault is never a dead end — the error line and '
      'the button re-render, and each tap re-pulls (S200 review pin)',
      (tester) async {
        final fake = _activeFake()..listParkedSendsThrows = StateError('boom');
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);
        final retry = find.byKey(const Key('parked-error-retry'));
        final readsBefore = fake.listParkedSendsCount;

        // The fault does NOT clear; tap anyway.
        await tester.ensureVisible(retry);
        await tester.pumpAndSettle();
        await tester.tap(retry);
        await tester.pumpAndSettle();

        expect(
          fake.listParkedSendsCount,
          greaterThan(readsBefore),
          reason: 'the tap re-pulled even though the fault persists',
        );
        expect(
          find.text(l10n.walletParkedError),
          findsOneWidget,
          reason: 'the honest error line re-renders — never a silent hide',
        );
        expect(
          retry,
          findsOneWidget,
          reason:
              'the retry affordance survives the failed retry (no dead end)',
        );
        expect(
          tester.widget<TextButton>(retry).onPressed,
          isNotNull,
          reason: 'the settled error arm re-enables the button for another try',
        );
      },
    );

    testWidgets(
      '#407 R5: the retry ANNOUNCES itself — the live region is on the label '
      'that swaps, not on a container whose semantics never change',
      (tester) async {
        // MY OWN CLEARANCE WAS WRONG HERE. The error arm wraps
        // `Semantics(container: true, liveRegion: true)` around the column, and
        // the code comment claimed "the semantics change re-fires the live
        // region when the identical error re-lands". Measured across the
        // refreshing flip, that node's SemanticsData is BYTE-IDENTICAL — a
        // liveRegion on a node whose data never changes fires once, at insert,
        // and never again. The insert-time announcement is the half I checked;
        // the re-fire on retry is the half that did not exist. Fourth and fifth
        // instances of the #403 R1 shape.
        //
        // The pin is the LABEL: it must swap text on refresh, and the flagged
        // node must be the one carrying that text.
        final fake = _activeFake()..listParkedSendsThrows = StateError('boom');
        final handle = tester.ensureSemantics();
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);
        final retry = find.byKey(const Key('parked-error-retry'));

        expect(find.text(l10n.walletParkedErrorRetry), findsOneWidget);
        expect(find.text(l10n.walletParkedErrorRetryInProgress), findsNothing);

        // Park the re-pull so the in-flight arm is observable.
        fake.listParkedSendsGate = Completer<void>();
        await tester.ensureVisible(retry);
        await tester.pumpAndSettle();
        await tester.tap(retry);
        await tester.pump();

        // TWO pumps, measured: the read provider re-runs on the first, and the
        // section's derived watch lands its rebuild on the next frame.
        await tester.pump();
        expect(
          find.descendant(
            of: retry,
            matching: find.byType(CircularProgressIndicator),
          ),
          findsOneWidget,
          reason: 'the sighted half — the S200 spinner — is up too',
        );
        expect(
          find.text(l10n.walletParkedErrorRetryInProgress),
          findsOneWidget,
          reason: 'the label SWAPS — which is what a screen reader hears',
        );
        expect(
          tester
              .getSemantics(find.text(l10n.walletParkedErrorRetryInProgress))
              .flagsCollection
              .isLiveRegion,
          isTrue,
          reason:
              'and the flag is ON that node, so the swap is an announcement '
              'rather than a silent repaint',
        );

        fake.listParkedSendsGate!.complete();
        await tester.pumpAndSettle();
        expect(
          find.text(l10n.walletParkedErrorRetry),
          findsOneWidget,
          reason: 'settled back to the idle label',
        );
        handle.dispose();
      },
    );

    testWidgets('cancel: confirm → discard calls cancel with (id, createdAt)', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [
          parkedSendFixture(id: 7, amountZat: 70000, createdAt: 1700000123),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final cancelBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedCancel,
      );
      await tester.ensureVisible(cancelBtn);
      await tester.pumpAndSettle();
      await tester.tap(cancelBtn);
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletParkedCancelConfirmTitle), findsOneWidget);

      await tester.tap(find.text(l10n.walletParkedCancelConfirmDiscard));
      await tester.pumpAndSettle();

      expect(fake.cancelParkedSendCount, 1);
      expect(fake.lastCancelId, 7);
      expect(fake.lastCancelCreatedAt, 1700000123);
      expect(find.text(l10n.walletParkedCancelDone), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5)); // drain snackbar
    });

    testWidgets('cancel false → honest "already on its way", NOT "cancelled"', (
      tester,
    ) async {
      // A SINGLE-STEP fixture deliberately: a drained
      // single-step row is NOT in-flight-listed, so the ACTIVITY list is the
      // only surface the "check your activity" copy can point at.
      final fake = _activeFake()
        ..parkedSendsResult = [
          parkedSendFixture(kind: ParkedSendKind.singleStep),
        ]
        ..cancelParkedSendResult = false;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final txReadsBefore = fake.transactionsCount;

      final cancelBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedCancel,
      );
      await tester.ensureVisible(cancelBtn);
      await tester.pumpAndSettle();
      await tester.tap(cancelBtn);
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletParkedCancelConfirmDiscard));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletParkedCancelAlreadySending), findsOneWidget);
      // The double-pay-safe contract: a false is NEVER shown as "cancelled".
      expect(find.text(l10n.walletParkedCancelDone), findsNothing);
      // #309 H1: a false means the drain won the race — the send just moved
      // parked → IN-FLIGHT, so the in-flight cue must have been re-pulled in
      // the same completion (else the send shows on ZERO surfaces).
      expect(
        fake.listInFlightSendsCount,
        greaterThan(1),
        reason: 'the cancel completion re-pulled the in-flight cue',
      );
      // ... and the ACTIVITY list re-pulled too: for a
      // drained SINGLE-STEP row the activity row is the only witness.
      expect(
        fake.transactionsCount,
        greaterThan(txReadsBefore),
        reason: 'the cancel completion re-pulled the activity list',
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('cancel: "Keep it" dismisses without calling cancel', (
      tester,
    ) async {
      final fake = _activeFake()..parkedSendsResult = [parkedSendFixture()];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final cancelBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedCancel,
      );
      await tester.ensureVisible(cancelBtn);
      await tester.pumpAndSettle();
      await tester.tap(cancelBtn);
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletParkedCancelConfirmKeep));
      await tester.pumpAndSettle();

      expect(fake.cancelParkedSendCount, 0);
      expect(find.text(l10n.walletParkedCancelConfirmTitle), findsNothing);
    });

    testWidgets('cancel throws → honest "unchanged, try again"', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture()]
        ..cancelParkedSendThrows = StateError('busy');
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final cancelBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedCancel,
      );
      await tester.ensureVisible(cancelBtn);
      await tester.pumpAndSettle();
      await tester.tap(cancelBtn);
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletParkedCancelConfirmDiscard));
      await tester.pumpAndSettle();

      expect(fake.cancelParkedSendCount, 1);
      expect(find.text(l10n.walletParkedCancelFailed), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });
  });

  group('paused (attempt-capped) parked send (#315 slice 1)', () {
    testWidgets(
      'a paused row reads "paused" with the hint — never "saved & pending"',
      (tester) async {
        // The honesty split: the wallet gave up auto-retrying this send, so it
        // must be DISTINGUISHABLE from a healthy pending one (the review
        // requirement — a user told "will send when ready" waits forever).
        final fake = _activeFake()
          ..parkedSendsResult = [
            parkedSendFixture(
              amountZat: 70000,
              createdAt: 1700000123,
              paused: true,
            ),
          ];
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);
        final amount = l10n.walletAmount(formatZec(70000));
        final time = _timeOf(1700000123);

        expect(
          find.text(l10n.walletParkedRowPausedTimed(amount, time)),
          findsOneWidget,
        );
        expect(
          find.text(l10n.walletParkedRowTimed(amount, time)),
          findsNothing,
          reason: 'a paused row must not read like a healthy pending one',
        );
        expect(find.text(l10n.walletParkedPausedHint), findsOneWidget);
        // Both affordances are offered: send-it-now AND discard (Cancel). Since
        // #361 the action carries the SAME "Send now" label as a healthy row —
        // it re-arms the budget and then SIGNS, so the weaker "Retry" verb would
        // under-promise the spend prompt it raises (arch review M4).
        expect(
          find.widgetWithText(TextButton, l10n.walletParkedSendNow),
          findsOneWidget,
        );
        expect(
          find.widgetWithText(TextButton, l10n.walletParkedCancel),
          findsOneWidget,
        );
      },
    );

    testWidgets('a healthy row shows no paused hint, and the SAME Send now '
        'action a paused row carries', (tester) async {
      final fake = _activeFake()..parkedSendsResult = [parkedSendFixture()];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      expect(find.text(l10n.walletParkedPausedHint), findsNothing);
      // The retired "Retry" label appears on no row since #361 — the key itself is
      // gone as of #400 R9, so assert on the literal it used to render.
      expect(
        find.widgetWithText(TextButton, 'Retry'),
        findsNothing,
        reason: 'the retired Retry label appears on no row since #361',
      );
      // …but the FR-23-b action itself is on every row (#361), worded for a
      // healthy one: a committed send must always have a user-paced way out,
      // and at host custody it is the ONLY way the queue drains.
      expect(
        find.widgetWithText(TextButton, l10n.walletParkedSendNow),
        findsOneWidget,
      );
    });

    testWidgets(
      'FR-23-b: Send now signs a HEALTHY row inside the authorize bracket',
      (tester) async {
        final fake = _activeFake()
          ..parkedSendsResult = [
            parkedSendFixture(id: 11, createdAt: 1700000999),
          ];
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        final sendNow = find.widgetWithText(
          TextButton,
          l10n.walletParkedSendNow,
        );
        await tester.ensureVisible(sendNow);
        await tester.pumpAndSettle();
        await tester.tap(sendNow);
        await tester.pumpAndSettle();

        expect(fake.authorizeParkedSendCount, 1);
        expect(fake.lastAuthorizeId, 11);
        expect(fake.lastAuthorizeCreatedAt, 1700000999);
        expect(
          fake.retryParkedSendCount,
          0,
          reason: 'a HEALTHY row has no retry budget to re-arm',
        );
        expect(find.text(l10n.walletParkedAuthorizeSent), findsOneWidget);
        await tester.pumpAndSettle(const Duration(seconds: 5));
      },
    );

    testWidgets('FR-23-b: the row single-flights — a double tap opens ONE '
        'bracket', (tester) async {
      // Money-safety does not depend on this (the SDK's atomic claim means a
      // second authorization can only lose the race), but WITHOUT it a
      // double-tap stacks two host prompts and then contradicts itself in two
      // snackbars — the exact ambiguity that makes a user re-enter a payment.
      final gate = Completer<void>();
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture()]
        ..authorizeParkedSendGate = gate.future;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pump(); // let the disable land, keep the call in flight

      // #400 R4 — the in-flight cue. At held custody there is no host prompt to
      // look at and the call runs an unbounded prove, so a bare disabled button
      // is indistinguishable from a dead app.
      final inProgress = find.widgetWithText(
        TextButton,
        l10n.walletParkedSendNowInProgress,
      );
      expect(inProgress, findsOneWidget);
      expect(
        find.descendant(
          of: inProgress,
          matching: find.byType(CircularProgressIndicator),
        ),
        findsOneWidget,
      );
      // #400 R4 — Cancel is disabled meanwhile: its confirm body claims "nothing
      // leaves your wallet", which the running sign may already have falsified.
      expect(
        tester
            .widget<TextButton>(
              find.widgetWithText(TextButton, l10n.walletParkedCancel),
            )
            .onPressed,
        isNull,
      );

      await tester.tap(inProgress, warnIfMissed: false);
      await tester.pump();
      expect(
        fake.authorizeParkedSendCount,
        1,
        reason: 'the second tap hit a disabled button',
      );
      gate.complete();
      await tester.pumpAndSettle();
      expect(fake.authorizeParkedSendCount, 1);
      // …and the latch releases, so the row is tappable again.
      expect(
        tester
            .widget<TextButton>(
              find.widgetWithText(TextButton, l10n.walletParkedSendNow),
            )
            .onPressed,
        isNotNull,
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('#400 R7: the single-flight is SECTION-wide — a second ROW '
        'cannot open a concurrent bracket', (tester) async {
      // Two ordinary taps on two rows used to open TWO host brackets. On a
      // single-slot host that is a money bug, not a cosmetic one: B's staged
      // credential overwrites A's, A's FR-17 binding no longer matches, and A is
      // refused — the user authorized two payments and one silently did not
      // happen.
      final gate = Completer<void>();
      final fake = _activeFake()
        ..parkedSendsResult = [
          parkedSendFixture(id: 1, createdAt: 1700000100),
          parkedSendFixture(id: 2, createdAt: 1700000200),
        ]
        ..authorizeParkedSendGate = gate.future;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final buttons = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      expect(buttons, findsNWidgets(2));
      await tester.ensureVisible(buttons.first);
      await tester.pumpAndSettle();
      await tester.tap(buttons.first);
      await tester.pump();

      // The OTHER row's button is disabled while this bracket is open.
      final other = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      expect(other, findsOneWidget, reason: 'the owning row swapped its label');
      expect(tester.widget<TextButton>(other).onPressed, isNull);
      await tester.tap(other, warnIfMissed: false);
      await tester.pump();
      expect(fake.authorizeParkedSendCount, 1);

      gate.complete();
      await tester.pumpAndSettle();
      expect(fake.authorizeParkedSendCount, 1);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('FR-23-b: the bracket carries the ROW\'s own FR-17 binding, '
        'kind and principal', (tester) async {
      // The host-custody safety contribution of this whole feature: a bound host
      // stages the seed for the binding it is prompting about, and a stage
      // recorded for any OTHER row is refused by the SDK. If the intent stopped
      // forwarding `send.binding`, every host-custody authorization would either
      // fail closed or (worse, on an unbound host) prompt about row A while
      // signing row B — with nothing else in the suite noticing.
      final authorizer = _RecordingAuthorizer();
      final fake = _activeFake()
        ..parkedSendsResult = [
          parkedSendFixture(id: 5, createdAt: 1700000321, amountZat: 61000),
        ];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSendAuthorizerProvider.overrideWithValue(authorizer),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pumpAndSettle();

      final intent = authorizer.lastIntent;
      expect(intent, isNotNull);
      expect(intent!.kind, WalletSpendKind.queuedSend);
      expect(
        intent.amountZat,
        61000,
        reason: 'the typed PRINCIPAL — the fee is computed at signing, on top',
      );
      expect(
        intent.bindingToken,
        parkedSendFixture(id: 5, createdAt: 1700000321).binding,
        reason: "THIS row's FR-17 binding, not another row's and not null",
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets(
      'FR-23-b: "still waiting" is NOT phrased as a failure (double-pay rule)',
      (tester) async {
        // The money-critical copy arm: nothing was signed, nothing was lost.
        // A user who reads this as "failed" re-enters the payment and BOTH
        // send. The row must stay on the surface, unchanged.
        final fake = _activeFake()
          ..parkedSendsResult = [parkedSendFixture()]
          ..authorizeParkedSendResult = ParkedAuthorization.stillQueued;
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        final sendNow = find.widgetWithText(
          TextButton,
          l10n.walletParkedSendNow,
        );
        await tester.ensureVisible(sendNow);
        await tester.pumpAndSettle();
        await tester.tap(sendNow);
        await tester.pumpAndSettle();

        expect(
          find.text(l10n.walletParkedAuthorizeStillWaiting),
          findsOneWidget,
        );
        expect(find.text(l10n.walletParkedAuthorizeFailed), findsNothing);
        expect(find.text(l10n.walletParkedAuthorizeSent), findsNothing);
        await tester.pumpAndSettle(const Duration(seconds: 5));
      },
    );

    testWidgets(
      'FR-23-b: a host DENIAL is silent — its own prompt already said so',
      (tester) async {
        // The #327 seam contract: the host communicates its own denial, so the
        // package shows nothing and changes nothing.
        final fake = _activeFake()..parkedSendsResult = [parkedSendFixture()];
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              walletSessionProvider.overrideWithValue(fake),
              walletSendAuthorizerProvider.overrideWithValue(
                const _DenyingAuthorizer(),
              ),
            ],
            child: MaterialApp(
              localizationsDelegates:
                  WalletLocalizations.localizationsDelegates,
              supportedLocales: WalletLocalizations.supportedLocales,
              theme: lightTheme,
              home: const WalletScreen(),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        final sendNow = find.widgetWithText(
          TextButton,
          l10n.walletParkedSendNow,
        );
        await tester.scrollUntilVisible(
          sendNow,
          300,
          scrollable: find.byType(Scrollable).first,
        );
        await tester.pumpAndSettle();
        await tester.tap(sendNow);
        await tester.pumpAndSettle();

        expect(
          fake.authorizeParkedSendCount,
          0,
          reason: 'a denied bracket never reaches the SDK',
        );
        expect(find.byType(SnackBar), findsNothing);
        await tester.pumpAndSettle(const Duration(seconds: 5));
      },
    );

    testWidgets('a PAUSED row re-arms its budget, THEN signs — one tap, in '
        'that order (FR-23-b, #361)', (tester) async {
      // The compose rule: the re-arm moves no money and signs nothing, but
      // without it a budget-capped row would authorize straight into "still
      // waiting" (the drain's cap gate refuses before the seed is ever pulled).
      // Both calls carry the SAME (id, createdAt) pin. One tap, no dialog: the
      // payment was confirmed when the user queued it; this decides nothing new.
      final fake = _activeFake()
        ..parkedSendsResult = [
          parkedSendFixture(id: 9, createdAt: 1700000456, paused: true),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final parkedReadsBefore = fake.listParkedSendsCount;

      final retryBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedSendNow,
      );
      await tester.ensureVisible(retryBtn);
      await tester.pumpAndSettle();
      await tester.tap(retryBtn);
      await tester.pumpAndSettle();

      expect(fake.retryParkedSendCount, 1);
      expect(fake.lastRetryId, 9);
      expect(fake.lastRetryCreatedAt, 1700000456);
      expect(fake.authorizeParkedSendCount, 1);
      expect(fake.lastAuthorizeId, 9);
      expect(fake.lastAuthorizeCreatedAt, 1700000456);
      expect(find.text(l10n.walletParkedAuthorizeSent), findsOneWidget);
      expect(
        fake.listParkedSendsCount,
        greaterThan(parkedReadsBefore),
        reason: 'the completion re-pulled the parked surface',
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('notFound → honest "isn\'t waiting anymore", never a re-send '
        'invitation', (tester) async {
      // The row left the parked set under us (sent / cancelled / rowid reused).
      // Mirrors the cancel-false contract: point at the surfaces, never imply
      // it is safe to enter the payment again.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)]
        ..authorizeParkedSendResult = ParkedAuthorization.notFound;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final retryBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedSendNow,
      );
      await tester.ensureVisible(retryBtn);
      await tester.pumpAndSettle();
      await tester.tap(retryBtn);
      await tester.pumpAndSettle();

      // #401 R2b: `notFound` gets its OWN copy. It does NOT mean "gone" — the
      // commonest cause is the background drain winning the claim, after which
      // the row re-renders as PREPARING on this very screen. `expired` keeps
      // walletParkedRetryStale, where "isn't waiting anymore" is exactly true.
      expect(find.text(l10n.walletParkedAlreadyInProgress), findsOneWidget);
      expect(find.text(l10n.walletParkedRetryStale), findsNothing);
      expect(find.text(l10n.walletParkedAuthorizeSent), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('S205-b successor: the "still waiting" copy promises NO retry '
        'timing, so the host\'s sync-off policy cannot make it false', (
      tester,
    ) async {
      // Pre-#361 the paused-row snackbar promised "it will try again shortly",
      // which a sync-off host silently broke. The
      // FR-23-b copy makes no timing promise at all, so ONE string stays true
      // under every sync policy — the honest fix for the whole class.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)]
        ..authorizeParkedSendResult = ParkedAuthorization.stillQueued;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSyncPolicyProvider.overrideWithValue(false),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      // Lazy ListView + the longer sync-off Send reason: drive the scroll
      // until the button is built (scrollUntilVisible idiom).
      final retryBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedSendNow,
      );
      await tester.scrollUntilVisible(
        retryBtn,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      await tester.tap(retryBtn);
      await tester.pumpAndSettle();

      expect(fake.retryParkedSendCount, 1, reason: 'the re-arm still ran');
      // #400 R5: a PAUSED row that was re-armed gets the sibling string. Neither
      // one promises a retry TIME, which is the property this test guards
      // — a sync-off host cannot make either of them false.
      expect(find.text(l10n.walletParkedAuthorizeRearmed), findsOneWidget);
      expect(find.text(l10n.walletParkedAuthorizeStillWaiting), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('a throw anywhere in the tap → honest "unchanged, try again"', (
      tester,
    ) async {
      // The re-arm is INSIDE the try, so its throw takes the same honest arm as
      // an authorize throw — the saved payment is untouched either way.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)]
        ..retryParkedSendThrows = StateError('busy');
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final retryBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedSendNow,
      );
      await tester.ensureVisible(retryBtn);
      await tester.pumpAndSettle();
      await tester.tap(retryBtn);
      await tester.pumpAndSettle();

      expect(fake.retryParkedSendCount, 1);
      expect(
        fake.authorizeParkedSendCount,
        0,
        reason: 'the throw short-circuits before any signing call',
      );
      expect(find.text(l10n.walletParkedAuthorizeFailed), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('a paused row is still cancellable (the cap never traps)', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [
          parkedSendFixture(id: 4, createdAt: 1700000789, paused: true),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final cancelBtn = find.widgetWithText(
        TextButton,
        l10n.walletParkedCancel,
      );
      await tester.ensureVisible(cancelBtn);
      await tester.pumpAndSettle();
      await tester.tap(cancelBtn);
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletParkedCancelConfirmDiscard));
      await tester.pumpAndSettle();

      expect(fake.cancelParkedSendCount, 1);
      expect(fake.lastCancelId, 4);
      expect(find.text(l10n.walletParkedCancelDone), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });
  });

  // ── #400 R8: the gaps the reliability pass ranked ────────────────────
  //
  // The identity fence had ZERO tests anywhere in the package — it is the money
  // guard that decides whether an approval may land, it was BROKEN once already
  // (the fold shipped a widget-lifetime leg that silently refused live
  // spends), and nothing in the suite would have caught either the break or the
  // fix. These pin both of its legs plus the row states around them.
  group('#400: the parked-send identity fence, row states and honest copy', () {
    testWidgets('the fence refuses BEFORE the SDK call when the identity flips '
        'while the prompt is open', (tester) async {
      // the user approved a spend for wallet A, but by the time they hit
      // Approve every visible surface belongs to wallet B — so the approval must
      // not reach A's money. Silent by contract (WalletSpendSessionChanged shows
      // nothing): the row is not even the current identity's.
      final a = _activeFake()..parkedSendsResult = [parkedSendFixture()];
      final b = _activeFake();
      final authorizer = _FlipAuthorizer();
      await tester.pumpWidget(_flipHarness(a, authorizer));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final container = ProviderScope.containerOf(
        tester.element(find.byType(WalletScreen)),
      );
      // The host swaps wallets while its prompt is up, before the action runs.
      authorizer.beforeAction = () =>
          container.read(_sessionSlot.notifier).swap(b);

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pumpAndSettle();

      expect(
        a.authorizeParkedSendCount,
        0,
        reason: 'the dead identity is never asked to sign',
      );
      expect(b.authorizeParkedSendCount, 0);
      expect(find.byType(SnackBar), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('#400 R6: a flip AFTER the signature delivers nothing over the '
        'new wallet', (tester) async {
      // The pre-action fence says nothing about the tens of seconds of proving
      // that follow it. Announcing "Sending your payment now." over wallet B is a
      // false claim about B — and on a duress/decoy flip it discloses that the
      // hidden wallet had a payment in flight.
      final a = _activeFake()
        ..parkedSendsResult = [parkedSendFixture()]
        ..authorizeParkedSendResult = ParkedAuthorization.signed;
      final b = _activeFake();
      final authorizer = _FlipAuthorizer();
      await tester.pumpWidget(_flipHarness(a, authorizer));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final container = ProviderScope.containerOf(
        tester.element(find.byType(WalletScreen)),
      );
      authorizer.afterAction = () =>
          container.read(_sessionSlot.notifier).swap(b);

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pumpAndSettle();

      expect(
        a.authorizeParkedSendCount,
        1,
        reason:
            'the signature DID happen — this is about delivery, not the sign',
      );
      expect(
        find.text(l10n.walletParkedAuthorizeSent),
        findsNothing,
        reason: 'the outcome belongs to a wallet that is no longer on screen',
      );
      expect(find.byType(SnackBar), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('a DENIED authorization does NOT re-arm a paused row', (
      tester,
    ) async {
      // The crypto audit fix: the re-arm clears the cap gate that is the
      // ONLY thing keeping a given-up row off the UNATTENDED background drain, so
      // running it outside the bracket meant a denial still re-enabled the send at
      // held custody. The pre-existing denial test used a fixture with
      // `paused: false`, so the re-arm never ran regardless of where it lived and
      // the regression was unpinnable (#400 R8b) — this one is paused.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSendAuthorizerProvider.overrideWithValue(
              const _DenyingAuthorizer(),
            ),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.scrollUntilVisible(
        sendNow,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pumpAndSettle();

      expect(
        fake.retryParkedSendCount,
        0,
        reason: 'a denial must not clear the cap gate',
      );
      expect(fake.authorizeParkedSendCount, 0);
      expect(find.byType(SnackBar), findsNothing);
      // …and the single-flight latch RELEASED on the denial path (#400 R7): a
      // user who dismissed by accident must be able to tap again, and a latch
      // leaked on an early-return arm would disable Send now on every row of the
      // section with no cue at all.
      expect(
        tester
            .widget<TextButton>(
              find.widgetWithText(TextButton, l10n.walletParkedSendNow),
            )
            .onPressed,
        isNotNull,
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('#400 R2: a MID-SIGNATURE row reads "preparing" and offers NO '
        'action at all', (tester) async {
      // The claim commits before the proving step, so an OOM kill there leaves a
      // committed spend that used to appear on NO surface at all — the #331
      // silent-hide shape, reachable since FR-23-b by a user tap.
      //
      // Neither verb can act on it: both are `Queued`-guarded. Send now would
      // answer notFound; Cancel would answer false — behind a dialog that first
      // promises "nothing leaves your wallet", which a claimed row cannot
      // guarantee (past the engine's create its notes are already spent).
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(sending: true)];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      expect(find.text(l10n.walletParkedPreparingHint), findsOneWidget);
      expect(
        find.widgetWithText(TextButton, l10n.walletParkedSendNow),
        findsNothing,
        reason:
            'every sign verb needs a queued row — it could only answer "no"',
      );
      expect(
        find.widgetWithText(TextButton, l10n.walletParkedCancel),
        findsNothing,
        reason: 'cancel is queued-guarded too, and its confirm copy would lie',
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('#400 R2: the MID-SIGNATURE hint tells a sync-off host the '
        'truth — the wallet cannot finish it alone', (tester) async {
      // The self-recovery this row relies on IS a sync pass. Promising it to a
      // host that has none is how a user waits forever and then re-enters the
      // payment.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(sending: true)];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSyncPolicyProvider.overrideWithValue(false),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      // Lazy ListView + the longer sync-off Send reason push the row below the
      // fold — drive the scroll until it builds (the sibling sync-off idiom).
      final hint = find.text(l10n.walletParkedPreparingHintSyncPaused);
      await tester.scrollUntilVisible(
        hint,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      expect(hint, findsOneWidget);
      expect(find.text(l10n.walletParkedPreparingHint), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('#400: a paused row that is ALSO mid-signature reads as '
        'preparing, and hides the reopen-sending flow', (tester) async {
      // `sending` wins over `paused`: "preparing" is the true present tense, and
      // the cap only governs the NEXT attempt. The reopen CTA is suppressed
      // because its own success copy tells the user to send the paused payment
      // afterwards — and this row shows no such button.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true, sending: true)];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      expect(find.text(l10n.walletParkedPreparingHint), findsOneWidget);
      expect(find.text(l10n.walletParkedPausedHint), findsNothing);
      expect(find.text(l10n.walletReclaimButton), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('#400 R1: under a sync-off policy the SIGNED copy names the '
        'residual the wallet cannot fix on its own', (tester) async {
      // Signed ≠ delivered: the broadcast is a detached best-effort kick with a
      // bounded retry, and its durable fallback runs only after a completed sync
      // pass. A sync-off host has none, so "Sending your payment now." full stop
      // would leave a stranded payment with no correction and no explanation.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture()]
        ..authorizeParkedSendResult = ParkedAuthorization.signed;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSyncPolicyProvider.overrideWithValue(false),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.scrollUntilVisible(
        sendNow,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pumpAndSettle();

      expect(
        find.text(l10n.walletParkedAuthorizeSentSyncPaused),
        findsOneWidget,
      );
      expect(find.text(l10n.walletParkedAuthorizeSent), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('a landed outcome is delivered even when the row left the list '
        'mid-flight', (tester) async {
      // H1, previously untested: the container and messenger are captured at
      // tap time precisely so a signature that lands while the user is elsewhere
      // still gets a confirmation. Without it a signed, broadcasting payment shows
      // nothing until the next sync edge (#309 H1).
      final gate = Completer<void>();
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture()]
        ..authorizeParkedSendResult = ParkedAuthorization.signed
        ..authorizeParkedSendGate = gate.future;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final container = ProviderScope.containerOf(
        tester.element(find.byType(WalletScreen)),
      );

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pump();

      // The row disappears under the user while the bracket is open.
      fake.parkedSendsResult = [];
      container.invalidate(walletParkedSendsReadProvider);
      await tester.pumpAndSettle();
      expect(
        find.widgetWithText(TextButton, l10n.walletParkedCancel),
        findsNothing,
      );

      gate.complete();
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletParkedAuthorizeSent), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });
  });

  group('reopen sending (reclaim) affordance (#315 slice 2)', () {
    testWidgets('the CTA appears ONLY when a send is paused', (tester) async {
      // A healthy queued row shows NO reclaim CTA (the window isn't stuck); a
      // paused row surfaces "Reopen sending" (the honest bricked-window signal).
      final healthy = _activeFake()..parkedSendsResult = [parkedSendFixture()];
      await tester.pumpWidget(_harness(healthy));
      await tester.pumpAndSettle();
      var l10n = _l10nAt(tester);
      expect(
        find.text(l10n.walletReclaimButton),
        findsNothing,
        reason: 'no CTA when nothing is paused',
      );

      final paused = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)];
      await tester.pumpWidget(_harness(paused));
      await tester.pumpAndSettle();
      l10n = _l10nAt(tester);
      expect(find.text(l10n.walletReclaimButton), findsOneWidget);
    });

    testWidgets(
      'confirm → discloses cost → runs the reclaim → Minted snackbar',
      (tester) async {
        final fake = _activeFake()
          ..parkedSendsResult = [parkedSendFixture(paused: true)]
          ..reclaimResult = const ReclaimOutcome.minted(amountZat: 50000);
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        final cta = find.text(l10n.walletReclaimButton);
        await tester.ensureVisible(cta);
        await tester.pumpAndSettle();
        await tester.tap(cta);
        await tester.pumpAndSettle();
        // The honest-cost disclosure dialog interposes BEFORE any bridge call.
        expect(find.text(l10n.walletReclaimConfirmTitle), findsOneWidget);
        expect(find.text(l10n.walletReclaimConfirmBody), findsOneWidget);
        expect(fake.reclaimCount, 0, reason: 'nothing runs until confirmed');

        await tester.tap(find.text(l10n.walletReclaimConfirmAction));
        await tester.pumpAndSettle();
        expect(fake.reclaimCount, 1, reason: 'authorized + ran once');
        // Minted is an INITIATED state, never "done".
        expect(find.text(l10n.walletReclaimStarted), findsOneWidget);
        await tester.pumpAndSettle(const Duration(seconds: 5));
      },
    );

    testWidgets(
      'an unrecognised outcome (core/bridge version skew) → NEUTRAL copy, '
      'never the "started" success line (S161 review B, H8)',
      (tester) async {
        final fake = _activeFake()
          ..parkedSendsResult = [parkedSendFixture(paused: true)]
          ..reclaimResult = const ReclaimOutcome.unknown();
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await tester.ensureVisible(find.text(l10n.walletReclaimButton));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletReclaimButton));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletReclaimConfirmAction));
        await tester.pumpAndSettle();
        // The forward-compat arm must render the NEUTRAL "check your sends" copy —
        // never the success ("started") line, which would falsely claim a mint.
        expect(find.text(l10n.walletReclaimUnknown), findsOneWidget);
        expect(find.text(l10n.walletReclaimStarted), findsNothing);
        await tester.pumpAndSettle(const Duration(seconds: 5));
      },
    );

    testWidgets('"Not now" dismisses without running the reclaim', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await tester.ensureVisible(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimConfirmCancel));
      await tester.pumpAndSettle();
      expect(fake.reclaimCount, 0);
      expect(find.text(l10n.walletReclaimConfirmTitle), findsNothing);
    });

    testWidgets('NothingToReclaim → honest "nothing to reopen"', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)]
        ..reclaimResult = const ReclaimOutcome.nothingToReclaim();
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await tester.ensureVisible(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimConfirmAction));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletReclaimNothing), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('NotBroadcast → honest "couldn\'t reach the network"', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)]
        ..reclaimResult = const ReclaimOutcome.notBroadcast(amountZat: 50000);
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await tester.ensureVisible(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimConfirmAction));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletReclaimNotBroadcast), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('insufficient funds → honest "you need shielded ZEC"', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)]
        ..reclaimThrows = const WalletApiError(
          code: 'RW-SEND-001',
          message: 'static',
          kind: WalletErrorKind.insufficientFunds(
            availableZat: 0,
            requiredZat: 50000,
            pendingIncomingZat: 0,
          ),
        );
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await tester.ensureVisible(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimConfirmAction));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletReclaimNeedsFunds), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('a denied authorization is silent (never "reopen failed")', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)];
      final denier = FakeSendAuthorizer(denyAll: true);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSendAuthorizerProvider.overrideWithValue(denier),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await tester.ensureVisible(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimConfirmAction));
      await tester.pumpAndSettle();
      // The host's prompt was the communication — no bridge call, no snackbar.
      expect(fake.reclaimCount, 0);
      expect(find.text(l10n.walletReclaimFailed), findsNothing);
      expect(find.text(l10n.walletReclaimStarted), findsNothing);
      expect(denier.intents.single.kind, WalletSpendKind.reclaim);
      // #383 R3 / field pin: a reclaim POSITIVELY returns the funds
      // to this wallet's own shielded pool — the prompt may honestly say
      // "funds stay in your wallet".
      expect(denier.intents.single.recipientIsSelf, isTrue);
    });

    testWidgets('in flight → the CTA is disabled (no redundant re-tap mint)', (
      tester,
    ) async {
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)]
        ..reclaimNeverCompletes = true;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await tester.ensureVisible(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimConfirmAction));
      await tester
          .pump(); // let the in-flight latch flip; the reclaim never resolves
      // The button now reads "Reopening…" and is disabled.
      expect(find.text(l10n.walletReclaimInProgress), findsOneWidget);
      final button = tester.widget<FilledButton>(
        find.ancestor(
          of: find.text(l10n.walletReclaimInProgress),
          matching: find.byType(FilledButton),
        ),
      );
      expect(button.onPressed, isNull, reason: 'disabled while in flight');
      expect(fake.reclaimCount, 1, reason: 'exactly one run in flight');
    });

    testWidgets(
      'the CTA + honest-cost disclosure survive 3× text scale on a narrow '
      'phone — a11y: no overflow, the dialog scrolls (S161 review D, H9)',
      (tester) async {
        // Real-world edge case: a low-vision user at 3× text scale on a 320-wide
        // phone. A RenderFlex overflow throws during layout/paint, which the test
        // binding surfaces via takeException — so a clipped CTA or dialog FAILS here.
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        tester.view.devicePixelRatio = 1.0;
        tester.view.physicalSize = const Size(320, 720);

        final fake = _activeFake()
          ..parkedSendsResult = [parkedSendFixture(paused: true)];
        await tester.pumpWidget(_harnessScaled(fake, 3.0));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        // The paused section + CTA sit below the fold at 3× (the body lazily
        // builds them). scrollUntilVisible builds AND lays them out — a RenderFlex
        // overflow in the section or the CTA would throw during that layout.
        final cta = find.text(l10n.walletReclaimButton);
        await tester.scrollUntilVisible(
          cta,
          300,
          scrollable: find.byType(Scrollable).first,
        );
        await tester.pumpAndSettle();
        expect(
          tester.takeException(),
          isNull,
          reason:
              'the paused section + Reopen CTA must not overflow at 3× text scale',
        );
        expect(cta, findsOneWidget);

        // The disclosure is a full sentence — at 3× it must SCROLL, not clip or
        // overflow (the AlertDialog is scrollable:).
        await tester.tap(cta);
        await tester.pumpAndSettle();
        expect(find.text(l10n.walletReclaimConfirmTitle), findsOneWidget);
        expect(find.text(l10n.walletReclaimConfirmBody), findsOneWidget);
        expect(
          tester.takeException(),
          isNull,
          reason:
              'the honest-cost dialog must scroll, not overflow, at 3× text scale',
        );
      },
    );
  });

  group('recover-now affordance (2e-2b-v-4b)', () {
    testWidgets('shown with funds → confirm → calls sweep', (tester) async {
      final fake = _activeFake()
        ..recoverableEphemeralFundsResult = [
          const RecoverableEphemeralFunds(
            recoverableZat: 250000,
            isFinal: true,
          ),
        ]
        ..sweepResult = ephemeralSweepSummaryFixture(recoveredZat: 250000);
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      expect(find.text(l10n.walletRecoverNow), findsOneWidget);
      await tester.tap(find.text(l10n.walletRecoverNow));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletRecoverConfirmTitle), findsOneWidget);

      await tester.tap(find.text(l10n.walletRecoverConfirmAction));
      await tester.pumpAndSettle();
      expect(fake.sweepCount, 1);
      // The provisional recovered amount is surfaced ("recovering"), not "+X".
      expect(
        find.text(l10n.walletRecoverDone(l10n.walletAmount(formatZec(250000)))),
        findsOneWidget,
      );
      await tester.pumpAndSettle(const Duration(seconds: 5)); // drain snackbars
    });

    testWidgets('partial faults / truncation → honest "try again"', (
      tester,
    ) async {
      final fake = _activeFake()
        ..recoverableEphemeralFundsResult = [
          const RecoverableEphemeralFunds(
            recoverableZat: 250000,
            isFinal: true,
          ),
        ]
        ..sweepResult = ephemeralSweepSummaryFixture(swept: 0, failed: 1);
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await tester.tap(find.text(l10n.walletRecoverNow));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletRecoverConfirmAction));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletRecoverRetry), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('nothing sweepable → honest "nothing to recover"', (
      tester,
    ) async {
      final fake = _activeFake()
        ..recoverableEphemeralFundsResult = [
          const RecoverableEphemeralFunds(
            recoverableZat: 250000,
            isFinal: true,
          ),
        ]
        ..sweepResult = ephemeralSweepSummaryFixture(swept: 0);
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await tester.tap(find.text(l10n.walletRecoverNow));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletRecoverConfirmAction));
      await tester.pumpAndSettle();
      expect(find.text(l10n.walletRecoverNothing), findsOneWidget);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('partial success → reports the amount AND flags the remainder', (
      tester,
    ) async {
      // swept>0 AND failed>0: the copy must NOT read as a clean "Recovering X"
      // (which would HIDE the remainder) — it reports the amount + "another try".
      final fake = _activeFake()
        ..recoverableEphemeralFundsResult = [
          const RecoverableEphemeralFunds(
            recoverableZat: 250000,
            isFinal: true,
          ),
        ]
        ..sweepResult = ephemeralSweepSummaryFixture(
          swept: 1,
          recoveredZat: 250000,
          failed: 1,
        );
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await tester.tap(find.text(l10n.walletRecoverNow));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletRecoverConfirmAction));
      await tester.pumpAndSettle();
      expect(
        find.text(
          l10n.walletRecoverDonePartial(l10n.walletAmount(formatZec(250000))),
        ),
        findsOneWidget,
      );
      // The clean-success copy must NOT appear (it would hide the remainder).
      expect(
        find.text(l10n.walletRecoverDone(l10n.walletAmount(formatZec(250000)))),
        findsNothing,
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets(
      'in-flight: the button disables (single-flight) during a sweep',
      (tester) async {
        final fake = _activeFake()
          ..recoverableEphemeralFundsResult = [
            const RecoverableEphemeralFunds(
              recoverableZat: 250000,
              isFinal: true,
            ),
          ]
          ..sweepNeverCompletes = true; // a stalled / slow sweep
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        await tester.tap(find.text(l10n.walletRecoverNow));
        await tester.pumpAndSettle();
        await tester.tap(find.text(l10n.walletRecoverConfirmAction));
        await tester
            .pump(); // enter the in-flight state (the sweep never resolves)

        // The button shows the in-progress label and is DISABLED.
        final inFlightBtn = find.widgetWithText(
          OutlinedButton,
          l10n.walletRecoverInProgress,
        );
        expect(inFlightBtn, findsOneWidget);
        expect(find.text(l10n.walletRecoverNow), findsNothing);
        expect(
          tester.widget<OutlinedButton>(inFlightBtn).onPressed,
          isNull,
          reason: 'disabled while a sweep runs',
        );

        // A re-tap launches NO second sweep (single-flight).
        await tester.tap(inFlightBtn, warnIfMissed: false);
        await tester.pump();
        expect(fake.sweepCount, 1);
      },
    );

    testWidgets('"Not now" dismisses without sweeping', (tester) async {
      final fake = _activeFake()
        ..recoverableEphemeralFundsResult = [
          const RecoverableEphemeralFunds(
            recoverableZat: 250000,
            isFinal: true,
          ),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await tester.tap(find.text(l10n.walletRecoverNow));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletRecoverConfirmCancel));
      await tester.pumpAndSettle();
      expect(fake.sweepCount, 0);
    });

    testWidgets('hidden when there are no recoverable funds', (tester) async {
      final fake = _activeFake()..recoverableEphemeralFundsResult = const [];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(find.text(l10n.walletRecoverNow), findsNothing);
    });

    testWidgets('GATED ON SUCCESS: a failed read hides the CTA (never "safe")', (
      tester,
    ) async {
      // The spec security note: a failed read must NOT read as "nothing to
      // recover" (hiding held funds). The CTA is absent on error — neutral, never
      // a "you're safe" claim — and the sweep is never offered from an error.
      final fake = _activeFake()
        ..recoverableEphemeralFundsThrows = StateError('read failed');
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(find.text(l10n.walletRecoverNow), findsNothing);
    });
  });

  group('in-flight "on its way — don\'t send it again" cue (#309)', () {
    testWidgets('renders the aggregate amount + the don\'t-re-send caution while a '
        'two-step is mid-flight', (tester) async {
      // The durable counterpart of the dismissible in-motion result screen: the
      // ONE permanently-true instruction survives on the wallet screen across the
      // forwarding window (the double-pay temptation window). Two in-flight rows
      // aggregate into one note (their gross totals sum).
      final fake = _activeFake()
        ..inFlightSendsResult = [
          inFlightSendFixture(amountZat: 60000),
          inFlightSendFixture(amountZat: 25000),
        ];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      expect(
        find.text(
          l10n.walletInFlightNote(2, l10n.walletAmount(formatZec(85000))),
        ),
        findsOneWidget,
      );
    });

    testWidgets('a single in-flight send reads singular ("it"), not plural', (
      tester,
    ) async {
      // Each two-step uses its OWN one-time address, so the copy pluralizes on
      // the row count — one row must read "a one-time address … don't send it
      // again" (and the two-row case above must not).
      final fake = _activeFake()
        ..inFlightSendsResult = [inFlightSendFixture(amountZat: 60000)];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      expect(
        find.text(
          l10n.walletInFlightNote(1, l10n.walletAmount(formatZec(60000))),
        ),
        findsOneWidget,
      );
      // with the sync policy at its default ON, the sync-off VARIANT
      // never renders (the counterfactual for the policy-off pins below).
      expect(
        find.text(
          l10n.walletInFlightNoteSyncPaused(
            1,
            l10n.walletAmount(formatZec(60000)),
          ),
        ),
        findsNothing,
      );
    });

    testWidgets('S205-c: under the host\'s sync-off policy the note is the '
        'VARIANT plural — one coherent "partway through … paused" story, '
        'never the S205-b claim-then-negation append', (tester) async {
      final fake = _activeFake()
        ..inFlightSendsResult = [inFlightSendFixture(amountZat: 60000)];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSyncPolicyProvider.overrideWithValue(false),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final variant = l10n.walletInFlightNoteSyncPaused(
        1,
        l10n.walletAmount(formatZec(60000)),
      );
      final base = l10n.walletInFlightNote(
        1,
        l10n.walletAmount(formatZec(60000)),
      );
      // Lazy ListView + the longer sync-off Send reason: drive the scroll
      // until the note is built (scrollUntilVisible idiom).
      final note = find.text(variant);
      await tester.scrollUntilVisible(
        note,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      // The VARIANT key carries the whole story as one sentence (the
      // append composed "still completing… Paused" — a claim then its
      // negation — and sentence-concatenation breaks CJK typography).
      expect(note, findsOneWidget);
      expect(
        find.textContaining(base),
        findsNothing,
        reason:
            'the "still completing" claim must not render anywhere — '
            'neither alone nor as the old appended composition',
      );
    });

    testWidgets('S205-c: the sync-off variant pluralizes on the row count '
        'like its sibling — N≥2 must not read "a one-time address" / '
        '"don\'t send it again"', (tester) async {
      final fake = _activeFake()
        ..inFlightSendsResult = [
          inFlightSendFixture(amountZat: 60000),
          inFlightSendFixture(amountZat: 25000),
        ];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            walletSessionProvider.overrideWithValue(fake),
            walletSyncPolicyProvider.overrideWithValue(false),
          ],
          child: MaterialApp(
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final note = find.text(
        l10n.walletInFlightNoteSyncPaused(
          2,
          l10n.walletAmount(formatZec(85000)),
        ),
      );
      await tester.scrollUntilVisible(
        note,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      expect(note, findsOneWidget);
      expect(
        find.textContaining(
          l10n.walletInFlightNote(2, l10n.walletAmount(formatZec(85000))),
        ),
        findsNothing,
      );
    });

    testWidgets(
      'hidden entirely when nothing is mid-flight (the common case)',
      (tester) async {
        final fake = _activeFake();
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        expect(find.byType(InFlightSendsSection), findsOneWidget);
        expect(
          find.descendant(
            of: find.byType(InFlightSendsSection),
            matching: find.byType(Text),
          ),
          findsNothing,
        );
      },
    );

    testWidgets(
      'the in-flight cue says it could not read instead of vanishing',
      (tester) async {
        // #308a (S2 §3.5d). The cue's silence on a failed read rendered the
        // same frame as "nothing is mid-flight" — the one state that tells the
        // user nothing about re-sending. Under S8 the core's fence, not this
        // read, guards a rescan, so the failure is an honesty defect: say so in
        // place, and keep the money surface standing.
        final fake = _activeFake()
          ..listInFlightSendsThrows = StateError('read failed');
        await tester.pumpWidget(_harness(fake));
        await tester.pumpAndSettle();
        expect(tester.takeException(), isNull);
        expect(
          find.descendant(
            of: find.byType(InFlightSendsSection),
            matching: find.text(_l10nAt(tester).walletInFlightReadError),
          ),
          findsOneWidget,
        );
      },
    );
  });
}

/// A host authorizer that DENIES every spend after (by contract) having shown
/// its own message — the #327 seam's "package shows nothing for a denial" arm,
/// exercised over the FR-23-b parked authorization.
class _DenyingAuthorizer implements WalletSendAuthorizer {
  const _DenyingAuthorizer();

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async => throw const WalletSpendAuthorizationDenied();
}

/// A pass-through authorizer with two hooks around the action, so a test can
/// model a host wallet FLIP at either edge of the bracket (#400 R8): while the
/// prompt is still open (the pre-action fence must refuse) or after the signature
/// landed (the delivery re-check must stay silent).
class _FlipAuthorizer implements WalletSendAuthorizer {
  void Function()? beforeAction;
  void Function()? afterAction;

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) async {
    beforeAction?.call();
    final result = await action();
    afterAction?.call();
    return result;
  }
}

/// Records the [WalletSpendIntent] the package hands the host, then runs the
/// action unmodified — the pass-through shape with a memory, so a test can pin
/// WHAT the host was told it was authorizing.
class _RecordingAuthorizer implements WalletSendAuthorizer {
  WalletSpendIntent? lastIntent;

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) {
    lastIntent = intent;
    return action();
  }
}

/// An authorizer whose bracket NEVER settles — the "hung host prompt" the seam
/// contract explicitly permits (no timeout). It flips the session first, so the
/// test can observe what the OTHER identity sees while A's bracket is still open.
class _HangingFlipAuthorizer implements WalletSendAuthorizer {
  void Function()? beforeAction;

  @override
  Future<T> authorizeSpend<T>(
    WalletSpendIntent intent,
    Future<T> Function() action,
  ) {
    beforeAction?.call();
    return Completer<T>().future; // never completes, never throws
  }
}

void _main401() {
  group('#401 R6 — the pins the fold shipped without', () {
    testWidgets('R6b: an identity flip CLEARS the section single-flight, so a '
        'hung prompt on wallet A cannot disable wallet B\'s only drain path', (
      tester,
    ) async {
      // The latch is container-scoped (it must outlive the row's dispose), so the
      // ONLY thing that bounds a wedged bracket is its identity dependency. Nothing
      // pinned that — and the two lifetimes this took before both got it wrong: a
      // root provider wedged every identity for the process, widget state died on a
      // scroll. Here A's prompt hangs forever; B must still be able to send.
      final a = _activeFake()..parkedSendsResult = [parkedSendFixture()];
      final b = _activeFake()
        ..parkedSendsResult = [parkedSendFixture()]
        ..authorizeParkedSendResult = ParkedAuthorization.signed;
      final authorizer = _HangingFlipAuthorizer();
      await tester.pumpWidget(_flipHarness(a, authorizer));
      await tester.pumpAndSettle();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(WalletScreen)),
        listen: false,
      );
      authorizer.beforeAction = () =>
          container.read(_sessionSlot.notifier).swap(b);
      final l10n = _l10nAt(tester);

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow); // claims the latch, then hangs on B's identity
      await tester.pumpAndSettle();

      // B's row renders its own Send now, ENABLED. Without the identity key the
      // button is disabled for as long as A's prompt sits open — which, by the
      // seam contract, can be forever.
      final bSendNow = find.widgetWithText(
        TextButton,
        l10n.walletParkedSendNow,
      );
      expect(bSendNow, findsOneWidget);
      expect(
        tester.widget<TextButton>(bSendNow).onPressed,
        isNotNull,
        reason: "wallet A's wedged bracket must not disable wallet B",
      );
      // And it is not showing A's spinner on a rowid collision (both fixtures are
      // id 1 — a per-DB rowid, so identical ids across wallets are ordinary).
      expect(find.text(l10n.walletParkedSendNowInProgress), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('R6c: a refresh that flips the row to `sending` MID-BRACKET '
        'keeps the button and its progress, instead of deleting the cue', (
      tester,
    ) async {
      // The SDK marks the row `sending` at the atomic claim, which is BEFORE the
      // proving step — so any parked-list refresh during the tens of seconds the
      // call runs flips `send.sending` true underneath an in-flight authorization.
      // `if (!send.sending || authorizingThis)` is the escape hatch; simplifying it
      // away deletes the spinner and the "Sending…" label mid-proof and leaves a
      // dead-looking row, which is exactly the re-entry the cue exists to prevent.
      final gate = Completer<void>();
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture()]
        ..authorizeParkedSendGate = gate.future;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final container = ProviderScope.containerOf(
        tester.element(find.byType(WalletScreen)),
        listen: false,
      );

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pump();

      // The claim landed in the store; a resume/sync-edge re-pull now sees it.
      fake.parkedSendsResult = [parkedSendFixture(sending: true)];
      container.invalidate(walletParkedSendsReadProvider);
      await tester.pump();
      await tester.pump();

      expect(
        find.text(l10n.walletParkedSendNowInProgress),
        findsOneWidget,
        reason: 'the in-flight cue must survive the row flipping to `sending`',
      );
      expect(find.byType(CircularProgressIndicator), findsWidgets);
      gate.complete();
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('R6d: a re-arm that returns FALSE does not claim the row will '
        'be tried again', (tester) async {
      // `retryParkedSend` false means the row was already un-paused or gone, so
      // nothing was re-armed. The fake defaults to true, so this direction shipped
      // untested — and the copy it selects is a statement about a re-arm that did
      // not happen.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)]
        ..retryParkedSendResult = false
        ..authorizeParkedSendResult = ParkedAuthorization.stillQueued;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletParkedAuthorizeStillWaiting), findsOneWidget);
      expect(find.text(l10n.walletParkedAuthorizeRearmed), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('R6d: a whole-scope teardown mid-bracket delivers NOTHING and '
        'throws nothing — the container/messenger guards', (tester) async {
      // Two guards shipped untested. The bracket captures the CONTAINER at tap time
      // precisely so a landed outcome survives the row's dispose — but if the whole
      // scope goes away (the host replaced the tree), every `container.read` throws
      // riverpod's internal StateError and the messenger's element is gone. Both
      // are caught deliberately: a throw here would happen AFTER the money moved,
      // outside any handler that could report it.
      final gate = Completer<void>();
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture()]
        ..authorizeParkedSendResult = ParkedAuthorization.signed
        ..authorizeParkedSendGate = gate.future;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final sendNow = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(sendNow);
      await tester.pumpAndSettle();
      await tester.tap(sendNow);
      await tester.pump();

      // The whole scope goes away while the signature is still running.
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
      gate.complete();
      await tester.pumpAndSettle(const Duration(seconds: 5));

      expect(
        tester.takeException(),
        isNull,
        reason: 'a post-teardown read/snackbar must never escape as an error',
      );
      expect(find.byType(SnackBar), findsNothing);
    });

    testWidgets('R6e: the preparing hint indents from the START edge in RTL '
        '(the package ships ar + he and had NO directional test at all)', (
      tester,
    ) async {
      // #400 made this padding EdgeInsetsDirectional and nothing verified it in the
      // direction the change was for. In RTL the 28dp indent must sit on the RIGHT,
      // under the label's text and past the leading icon — a hard `left` puts it on
      // the wrong side and breaks exactly the alignment it exists for.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(sending: true)];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [walletSessionProvider.overrideWithValue(fake)],
          child: MaterialApp(
            locale: const Locale('he'),
            localizationsDelegates: WalletLocalizations.localizationsDelegates,
            supportedLocales: WalletLocalizations.supportedLocales,
            theme: lightTheme,
            home: const WalletScreen(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      expect(
        Directionality.of(tester.element(find.byType(WalletScreen))),
        ui.TextDirection.rtl,
        reason: 'the harness itself must actually be RTL',
      );

      final hint = find.text(l10n.walletParkedPreparingHint);
      await tester.scrollUntilVisible(
        hint,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      final hintBox = tester.getRect(hint);
      final rowBox = tester.getRect(
        find.ancestor(of: hint, matching: find.byType(Padding)).last,
      );
      expect(
        rowBox.right - hintBox.right,
        greaterThan(20),
        reason: 'the indent is on the RIGHT in RTL (start edge)',
      );
      expect(
        hintBox.left - rowBox.left,
        lessThan(rowBox.right - hintBox.right),
        reason:
            'and NOT on the left, which is where a hard `left` would put it',
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });
  });

  // #403 R2/R4 — the paused-sync notes must name only what is on screen, and
  // must be keyed on the SSOT rather than on the host policy alone.
  group('#403 R2/R4 — the paused notes', () {
    Widget pausedHarness(WalletSession session) => ProviderScope(
      overrides: [
        walletSessionProvider.overrideWithValue(session),
        walletSyncPolicyProvider.overrideWithValue(false),
      ],
      child: MaterialApp(
        localizationsDelegates: WalletLocalizations.localizationsDelegates,
        supportedLocales: WalletLocalizations.supportedLocales,
        theme: lightTheme,
        home: const WalletScreen(),
      ),
    );

    testWidgets('R2: a section whose rows are ALL mid-signature drops the note '
        'that names Send now — the button is not there', (tester) async {
      // #401 R2d gave this surface its own note precisely BECAUSE it has a
      // per-row escape. Send now is suppressed on every `sending` row, so with
      // no queued row the note named a control that is not on screen — one line
      // above the row whose KNOWN RESIDUAL is that it has no in-app exit. The
      // shared note it replaced named no affordance, so that was a regression
      // introduced by the fix, not a pre-existing gap.
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(sending: true)];
      await tester.pumpWidget(pausedHarness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      // Lazy ListView: the section must be BUILT before any finder can see it.
      await tester.scrollUntilVisible(
        find.text(l10n.walletParkedTitle),
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletParkedSyncPausedNote), findsNothing);
      expect(
        find.text(l10n.walletSyncPausedMoneyNote),
        findsOneWidget,
        reason: 'the affordance-free fallback, already translated everywhere',
      );
      expect(
        find.widgetWithText(TextButton, l10n.walletParkedSendNow),
        findsNothing,
        reason: 'the premise: the named control really is absent',
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('R2: one QUEUED row is enough to keep the note that names the '
        'escape', (tester) async {
      // The counterfactual. Without it "fall back to the plain note" and "never
      // name the escape again" are the same test — and the escape is a tap that
      // releases committed funds, which is the whole reason #401 R2d exists.
      final fake = _activeFake()
        ..parkedSendsResult = [
          parkedSendFixture(sending: true),
          parkedSendFixture(id: 8, createdAt: 1700000500),
        ];
      await tester.pumpWidget(pausedHarness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await tester.scrollUntilVisible(
        find.text(l10n.walletParkedTitle),
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletParkedSyncPausedNote), findsOneWidget);
      expect(find.text(l10n.walletSyncPausedMoneyNote), findsNothing);
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('R4: the in-flight cue takes the PAUSED variant under a FAILED '
        'sync start, with the host policy still ON', (tester) async {
      // The fourth site of the converged #401 R5 keying, missed by that fold:
      // `in_flight_sends_section` still tested `disabledByHost` directly. Under
      // `failed` the loop never ran, so the second leg forwards on a pass that
      // cannot happen — and the unqualified note says the wallet "is still
      // completing" that payment.
      final fake = _activeFake()
        ..failStart = true
        ..inFlightSendsResult = [inFlightSendFixture(amountZat: 60000)];
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final amount = l10n.walletAmount(formatZec(60000));

      final variant = find.text(l10n.walletInFlightNoteSyncPaused(1, amount));
      await tester.scrollUntilVisible(
        variant,
        300,
        scrollable: find.byType(Scrollable).first,
      );
      await tester.pumpAndSettle();
      expect(variant, findsOneWidget);
      expect(
        find.text(l10n.walletInFlightNote(1, amount)),
        findsNothing,
        reason: 'the unqualified note claims progress that cannot happen',
      );
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });
  });

  // ONE NODE, OR IT ANNOUNCES NOTHING (#403 R1).
  //
  // #401 R3b put `liveRegion` AROUND three buttons and shipped the claim without
  // the behaviour. `ButtonStyleButton` builds its own `Semantics(container: true)`,
  // so an outer annotation cannot merge into it — it becomes a SEPARATE, forever
  // EMPTY parent node, while the label that actually changes sits on the child.
  // The engine announces updates to the FLAGGED node, so a flagged node whose data
  // never changes is silence: the parked Send now was mute for the whole unbounded
  // prove, i.e. the double-pay path R3b reopened for non-sighted users stayed open.
  //
  // These pins assert the only property that separates the two shapes: the
  // live-region flag and the SWAPPED label sit on the SAME semantics node.
  // `containsSemantics` against the BUTTON's own node fails on the broken shape
  // (that node has `isLiveRegion: false`) and passes on the fixed one — verified
  // in both directions, unlike the #401 R6a pin this fold had to replace.
  group('#403 R1 — the in-flight live regions must actually announce', () {
    testWidgets('parked Send now: the flag and the timed in-progress label are '
        'ONE node', (tester) async {
      final handle = tester.ensureSemantics();
      final gate = Completer<void>();
      final send = parkedSendFixture();
      final fake = _activeFake()
        ..parkedSendsResult = [send]
        ..authorizeParkedSendGate = gate.future;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final idle = find.widgetWithText(TextButton, l10n.walletParkedSendNow);
      await tester.ensureVisible(idle);
      await tester.pumpAndSettle();
      // The section is NOT a permanent announcer: at rest the button's node
      // carries no live region, so an ordinary parked list stays silent until the
      // user's own tap starts something.
      expect(tester.getSemantics(idle), isNot(isSemantics(isLiveRegion: true)));

      await tester.tap(idle);
      await tester.pump(); // the bracket is open and hanging on the gate

      final busy = find.widgetWithText(
        TextButton,
        l10n.walletParkedSendNowInProgress,
      );
      expect(busy, findsOneWidget);
      expect(
        tester.getSemantics(busy),
        isSemantics(
          isLiveRegion: true,
          isButton: true,
          // #401 R3c: the amount/time binding survives the in-flight state, so
          // two same-amount rows stay distinguishable while one is proving.
          label: l10n.walletParkedSendNowInProgressSemanticTimed(
            l10n.walletAmount(formatZec(send.amountZat)),
            _timeOf(send.createdAt),
          ),
        ),
        reason:
            'a flag on a wrapper node ABOVE the button announces nothing — the '
            'engine speaks updates to the FLAGGED node',
      );

      handle.dispose();
      gate.complete();
      await tester.pumpAndSettle(const Duration(seconds: 5));
    });

    testWidgets('reclaim CTA: the flag lands on the button, not on the node '
        'that absorbed the constant explainer', (tester) async {
      // The broken shape wrapped the whole Column, so the flagged node's label
      // was the CONSTANT explainer text — a live region whose content never
      // changes announces nothing even with the flag correctly set.
      final handle = tester.ensureSemantics();
      final fake = _activeFake()
        ..parkedSendsResult = [parkedSendFixture(paused: true)]
        ..reclaimNeverCompletes = true;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      await tester.ensureVisible(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimButton));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReclaimConfirmAction));
      await tester.pump();

      expect(
        tester.getSemantics(
          find.widgetWithText(FilledButton, l10n.walletReclaimInProgress),
        ),
        isSemantics(
          isLiveRegion: true,
          isButton: true,
          label: l10n.walletReclaimInProgress,
        ),
      );
      handle.dispose();
    });

    testWidgets('ephemeral-sweep CTA: the flag and the swapped label are ONE '
        'node (a re-tap burns a real fee)', (tester) async {
      final handle = tester.ensureSemantics();
      final fake = _activeFake()
        ..recoverableEphemeralFundsResult = [
          const RecoverableEphemeralFunds(
            recoverableZat: 250000,
            isFinal: true,
          ),
        ]
        ..sweepNeverCompletes = true;
      await tester.pumpWidget(_harness(fake));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final idle = find.widgetWithText(OutlinedButton, l10n.walletRecoverNow);
      expect(tester.getSemantics(idle), isNot(isSemantics(isLiveRegion: true)));

      await tester.tap(find.text(l10n.walletRecoverNow));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletRecoverConfirmAction));
      await tester.pump();

      expect(
        tester.getSemantics(
          find.widgetWithText(OutlinedButton, l10n.walletRecoverInProgress),
        ),
        isSemantics(
          isLiveRegion: true,
          isButton: true,
          label: l10n.walletRecoverInProgress,
        ),
      );
      handle.dispose();
    });
  });
}
