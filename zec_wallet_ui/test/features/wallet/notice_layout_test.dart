import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/router/wallet_routes.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/onboarding_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_rescan_controller.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_session.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/action_row_layout.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/testing.dart';

/// #409 R3/R4 — the notice cards' trailing action starved the message it sat
/// beside, and two overflow-menu labels claimed their full intrinsic width.
///
/// These pins measure WIDTH, not exceptions. Pre-fix the message is 59dp wide
/// on a 320dp phone AT THE DEFAULT TEXT SCALE and nothing is thrown — the
/// first RenderFlex overflow only arrives at 360dp/3.0x — so a `takeException`
/// assertion alone would have called the broken phone healthy. Every case here
/// was confirmed by mutation (the stack branch disabled, the width term
/// dropped, the retired scale(100) predicate restored, the Flexible removed).

WalletLocalizations _l10nAt(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(WalletScreen)));

Widget _harness({
  WalletSession? session,
  double? textScale,
  TextScaler? scaler,
  Locale? locale,
  List<Override> extraOverrides = const [],
}) {
  return ProviderScope(
    overrides: [
      if (session != null) walletSessionProvider.overrideWithValue(session),
      ...extraOverrides,
    ],
    child: MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      locale: locale,
      theme: lightTheme,
      home: const WalletScreen(),
      builder: (context, child) {
        if (scaler != null) {
          return MediaQuery(
            data: MediaQuery.of(context).copyWith(textScaler: scaler),
            child: child!,
          );
        }
        return textScale == null
            ? child!
            : MediaQuery.withClampedTextScaling(
                minScaleFactor: textScale,
                maxScaleFactor: textScale,
                child: child!,
              );
      },
    ),
  );
}

/// A wallet whose sync loop refuses to start — the state that renders
/// [_SyncStartFailedNotice], the notice carrying the Try-again button that
/// #409 R2's conservative `failed` reading depends on being READABLE.
FakeWalletSession _startFailedFake() => FakeWalletSession(
  current: const SyncStatus.idle(),
  snapshotValue: walletStateFixture(),
)..failStart = true;

/// A synced wallet, for the rescan-outcome (compact) notices.
FakeWalletSession _upToDateFake() => FakeWalletSession(
  current: const SyncStatus.upToDate(tip: 100),
  snapshotValue: walletStateFixture(),
);

/// The width the notice's message keeps, and the width of the card it sits in.
///
/// The card is resolved by KEY, never by "nearest `Container` ancestor". That
/// shortcut is what the first draft used, and it makes every ratio below
/// vacuous: wrapping the message in a bare `Container` — an ordinary styling
/// edit — moves the denominator onto the wrapper, so a 59dp message inside a
/// 288dp card reads as 100% and all four pins pass against the un-fixed
/// layout. Measured, not hypothesised (#409 R3 review, F1).
({double message, double card}) _measure(WidgetTester tester, String message) {
  final text = find.text(message);
  final card = find.byKey(_noticeCardKey);
  expect(
    card,
    findsOneWidget,
    reason: 'the notice card must be identifiable, or the ratio means nothing',
  );
  return (
    message: tester.getSize(text).width,
    card: tester.getSize(card).width,
  );
}

/// Mirrors `_ActionNotice.cardKey`, which is private to `wallet_screen.dart`.
const _noticeCardKey = ValueKey('wallet-action-notice-card');

/// The floor the message must clear, as a share of the card.
///
/// Derived, not guessed. STACKED the message spans the card minus its padding,
/// icon and gap: (288 − 32 − 24 − 12) / 288 = 0.757 for the prominent notice,
/// 0.826 for the compact one. INLINE — the defect — the trailing button takes
/// its label width first and the measured share never exceeds ~0.51 at these
/// widths (59dp/288 = 0.21 in the worst real case). So anything in 0.55..0.75
/// separates the two states.
///
/// 0.60 rather than 0.75 on purpose: at 0.75 the binding case cleared the bar
/// by **2.0dp**, so a cosmetic 2dp padding bump turned the pin red while the
/// message still held 74% of the card — a change-detector, not a starvation
/// pin, and the kind that gets "fixed" by lowering the constant.
const _messageFloor = 0.60;

/// Android 14+ scales text NON-LINEARLY: [MediaQuery.textScalerOf] returns a
/// `SystemTextScaler` that forwards to the platform curve, and that curve
/// boosts small text far more than large. This one is calibrated on the shape
/// of the real thing — body text at 1.5x while a 100pt probe reads only 1.1x.
///
/// It is the reason the retired `scale(100) > 140` predicate was wrong rather
/// than merely duplicated: it asks about a size no wallet surface renders.
class _NonLinearScaler extends TextScaler {
  const _NonLinearScaler();

  @override
  double scale(double fontSize) =>
      fontSize <= 20 ? fontSize * 1.5 : fontSize * 1.1;

  @override
  // ignore: deprecated_member_use
  double get textScaleFactor => 1.5;
}

void main() {
  group('the shared stacking rule (walletRowStacksAction)', () {
    testWidgets('a NON-LINEAR OS scale stacks on the BODY size, not a 100pt '
        'probe — the retired scale(100) > 140 predicate reads 1.1x here and '
        'would keep the starved row', (tester) async {
      late bool stacksWide;
      late bool scaleForces;
      late double legacyProbe;
      await tester.pumpWidget(
        MaterialApp(
          theme: lightTheme,
          home: MediaQuery(
            data: const MediaQueryData(textScaler: _NonLinearScaler()),
            child: Builder(
              builder: (context) {
                // 900dp — far past the width term, so ONLY the scale term can
                // force the stack. This is the assertion that separates the
                // two predicates.
                stacksWide = walletRowStacksAction(context, 900);
                scaleForces = walletTextScaleForcesStack(context);
                legacyProbe = MediaQuery.textScalerOf(context).scale(100);
                return const SizedBox();
              },
            ),
          ),
        ),
      );
      expect(
        legacyProbe,
        lessThan(140),
        reason: 'the retired predicate would read this device as unscaled',
      );
      expect(scaleForces, isTrue, reason: 'body text is at 1.5x');
      expect(stacksWide, isTrue);
    });

    testWidgets('width alone stacks a narrow row at 1.0x, and a wide row stays '
        'inline', (tester) async {
      late bool narrow;
      late bool wide;
      await tester.pumpWidget(
        MaterialApp(
          theme: lightTheme,
          home: Builder(
            builder: (context) {
              narrow = walletRowStacksAction(context, 320);
              wide = walletRowStacksAction(context, 900);
              return const SizedBox();
            },
          ),
        ),
      );
      expect(narrow, isTrue, reason: 'a phone-width row cannot share');
      expect(
        wide,
        isFalse,
        reason: 'desktop/tablet keeps the compact single row at ordinary scale',
      );
    });

    testWidgets('a NON-FINITE width stacks — the inline branch it would '
        'otherwise pick holds an Expanded, which asserts under unbounded '
        'constraints', (tester) async {
      late bool unbounded;
      late bool notANumber;
      await tester.pumpWidget(
        MaterialApp(
          theme: lightTheme,
          home: Builder(
            builder: (context) {
              unbounded = walletRowStacksAction(context, double.infinity);
              notANumber = walletRowStacksAction(context, double.nan);
              return const SizedBox();
            },
          ),
        ),
      );
      // `infinity < 520` is FALSE, so without the guard the degenerate input
      // selects exactly the branch that cannot survive it.
      expect(unbounded, isTrue);
      expect(notANumber, isTrue);
    });
  });

  group('#409 R3 — the notice cards keep their message readable', () {
    testWidgets('the sync-start-failed notice at 320dp/1.0x: the message keeps '
        'the card, and Try again is still there — pre-fix it kept 59dp of '
        '280 and threw NOTHING', (tester) async {
      await tester.binding.setSurfaceSize(const Size(320, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(_harness(session: _startFailedFake()));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final thrown = tester.takeException();
      final m = _measure(tester, l10n.walletSyncStartFailed);
      expect(thrown, isNull);
      expect(
        m.message,
        greaterThan(m.card * _messageFloor),
        reason: 'the message owns the row once the action stacks under it',
      );
      expect(find.text(l10n.walletSyncRetry), findsOneWidget);
    });

    testWidgets('the sync-start-failed notice at 360dp/3.0x — pre-fix a 0.0dp '
        'message and a 137px RenderFlex overflow', (tester) async {
      await tester.binding.setSurfaceSize(const Size(360, 2400));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        _harness(session: _startFailedFake(), textScale: 3.0),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      // Read the exception FIRST, so a mutation run reports whether this
      // corner is silent (it is — that is the point of the case).
      final thrown = tester.takeException();
      final m = _measure(tester, l10n.walletSyncStartFailed);
      expect(thrown, isNull);
      expect(m.message, greaterThan(m.card * _messageFloor));
    });

    testWidgets(
      'the rescan-failed notice at 320dp/1.3x (pre-fix: 80dp wide, no '
      'exception) — and Dismiss still works from the stacked layout',
      (tester) async {
        await tester.binding.setSurfaceSize(const Size(320, 1200));
        addTearDown(() => tester.binding.setSurfaceSize(null));
        final stub = _StubRescan(const WalletRescanFailed());
        await tester.pumpWidget(
          _harness(
            session: FakeWalletSession(
              current: const SyncStatus.upToDate(tip: 100),
              snapshotValue: walletStateFixture(),
            ),
            textScale: 1.3,
            extraOverrides: [
              walletRescanControllerProvider.overrideWith(() => stub),
            ],
          ),
        );
        await tester.pumpAndSettle();
        final l10n = _l10nAt(tester);

        final thrown = tester.takeException();
        final m = _measure(tester, l10n.walletRescanFailedNotice);
        expect(thrown, isNull);
        expect(m.message, greaterThan(m.card * _messageFloor));

        // The stacked action is still the SAME action — a layout fix that
        // detached the dismiss would be a worse defect than the one it fixed.
        await tester.tap(find.text(l10n.walletRescanFailedDismiss));
        await tester.pumpAndSettle();
        expect(stub.dismissCount, 1);
        expect(find.text(l10n.walletRescanFailedNotice), findsNothing);
      },
    );

    // The two WIDE cases below run on a COMPACT notice (the rescan-failed
    // line). Since stage S11 the start-failed notice is a card-notice, whose
    // button always sits under the message, so it can no longer show either
    // half of the inline-versus-stacked rule; the line still can.
    testWidgets('on a WIDE surface at 1.0x the action stays BESIDE the message '
        '— the stack is a narrow/large-scale remedy, not a new default', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(const Size(900, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        _harness(
          session: _upToDateFake(),
          extraOverrides: [
            walletRescanControllerProvider.overrideWith(
              () => _StubRescan(const WalletRescanFailed()),
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final messageRight = tester
          .getBottomRight(find.text(l10n.walletRescanFailedNotice))
          .dx;
      final actionLeft = tester
          .getTopLeft(find.text(l10n.walletRescanFailedDismiss))
          .dx;
      expect(
        actionLeft,
        greaterThan(messageRight),
        reason: 'inline: the action sits after the message, not under it',
      );
    });

    testWidgets('on a WIDE surface the TEXT SCALE still stacks it — the only '
        'case where the scale half of the rule can be observed through a '
        'rendered widget (#409 R3 review, F3)', (tester) async {
      // Every other case here is under 520dp, so the WIDTH half already
      // stacks them and the scale half is never exercised end to end: dropping
      // `walletTextScaleForcesStack` used to fail one pure-unit test and no
      // rendered pin at all. 900dp is past the width term by 380dp, so only
      // the scale term can produce a stack.
      await tester.binding.setSurfaceSize(const Size(900, 2400));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        _harness(
          session: _upToDateFake(),
          textScale: 2.0,
          extraOverrides: [
            walletRescanControllerProvider.overrideWith(
              () => _StubRescan(const WalletRescanFailed()),
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final messageBottom = tester
          .getBottomRight(find.text(l10n.walletRescanFailedNotice))
          .dy;
      final actionTop = tester
          .getTopLeft(find.text(l10n.walletRescanFailedDismiss))
          .dy;
      expect(
        actionTop,
        greaterThanOrEqualTo(messageBottom),
        reason: 'stacked: the action sits BELOW the message, not beside it',
      );
    });

    testWidgets('the start-failed CARD-notice puts its action under the '
        'message even on a wide surface at 1.0x (stage S11 C2: a card-notice '
        'draws its buttons below the message)', (tester) async {
      await tester.binding.setSurfaceSize(const Size(900, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(_harness(session: _startFailedFake()));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final messageBottom = tester
          .getBottomRight(find.text(l10n.walletSyncStartFailed))
          .dy;
      final actionTop = tester.getTopLeft(find.text(l10n.walletSyncRetry)).dy;
      expect(actionTop, greaterThanOrEqualTo(messageBottom));
    });

    testWidgets('de at 320dp/1.0x — the locale that motivates the fix: the '
        'pre-fix message was 0.0dp wide in a 1250dp-tall card, silently', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(const Size(320, 1400));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        _harness(session: _startFailedFake(), locale: const Locale('de')),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      final thrown = tester.takeException();
      final m = _measure(tester, l10n.walletSyncStartFailed);
      expect(thrown, isNull);
      expect(m.message, greaterThan(m.card * _messageFloor));
      final cardHeight = tester.getSize(find.byKey(_noticeCardKey)).height;
      expect(
        cardHeight,
        lessThan(400),
        reason: 'pre-fix this card was 1250dp tall for one sentence',
      );
    });

    testWidgets('the sync-not-running refusal renders ITS OWN copy and ITS OWN '
        'dismiss (#409 R3 review, F7 — this arm had no widget coverage at '
        'all, so a swapped message AND a dead Dismiss both survived)', (
      tester,
    ) async {
      final stub = _StubRescan(const WalletRescanBlockedBySyncNotRunning());
      await tester.pumpWidget(
        _harness(
          session: FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(),
          ),
          extraOverrides: [
            walletRescanControllerProvider.overrideWith(() => stub),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      // This is the ONE refusal entitled to say "unchanged" outright (#405) —
      // the settling-send copy promises an hours-scale wait instead, so a
      // copy-paste swap between the two tells a user the wrong thing about
      // their wallet.
      expect(
        find.text(l10n.walletRescanBlockedSyncNotRunningNotice),
        findsOneWidget,
      );
      expect(find.text(l10n.walletRescanBlockedSettlingNotice), findsNothing);
      expect(find.text(l10n.walletRescanFailedNotice), findsNothing);
      expect(find.text(l10n.walletRescanNeedsSpaceNotice), findsNothing);
      expect(find.byIcon(Icons.sync_disabled), findsOneWidget);

      await tester.tap(find.text(l10n.walletRescanFailedDismiss));
      await tester.pumpAndSettle();
      expect(
        stub.dismissCount,
        1,
        reason: 'the Dismiss must reach the notifier',
      );
      expect(
        find.text(l10n.walletRescanBlockedSyncNotRunningNotice),
        findsNothing,
      );
    });

    // Stage S11 C2: the prominent notice is a card-notice (a 20 glyph) and the
    // compact one the notice line (a 16 glyph); both forms set the message in
    // bodyMedium (§6.10), so the glyph is what tells the two apart.
    testWidgets('the two densities are actually different — prominent carries '
        'the body-sized copy and the card glyph (#409 R3 review, F8: '
        'flipping the start-failed notice to compact survived the suite)', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(const Size(320, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(_harness(session: _startFailedFake()));
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final context = tester.element(find.byType(WalletScreen));
      final textTheme = Theme.of(context).textTheme;

      // Scoped to the card: this screen also renders the sync BADGE, which
      // carries the same icon and the same sentence.
      final card = find.byKey(_noticeCardKey);
      final message = tester.widget<Text>(
        find.descendant(
          of: card,
          matching: find.text(l10n.walletSyncStartFailed),
        ),
      );
      expect(message.style?.fontSize, textTheme.bodyMedium?.fontSize);
      final icon = tester.widget<Icon>(
        find.descendant(of: card, matching: find.byIcon(Icons.sync_problem)),
      );
      expect(
        icon.size,
        20,
        reason: 'prominent is the card-notice (20 glyph); compact pins 16',
      );
    });

    testWidgets('a compact notice carries the body copy and the 16dp icon', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(const Size(320, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        _harness(
          session: FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(),
          ),
          extraOverrides: [
            walletRescanControllerProvider.overrideWith(
              () => _StubRescan(const WalletRescanFailedNeedsSpace()),
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);
      final context = tester.element(find.byType(WalletScreen));
      final textTheme = Theme.of(context).textTheme;

      final card = find.byKey(_noticeCardKey);
      final message = tester.widget<Text>(
        find.descendant(
          of: card,
          matching: find.text(l10n.walletRescanNeedsSpaceNotice),
        ),
      );
      expect(message.style?.fontSize, textTheme.bodyMedium?.fontSize);
      // The disk-full icon, not one of its three siblings' (a swapped icon
      // survived the suite before this).
      final icon = tester.widget<Icon>(
        find.descendant(of: card, matching: find.byIcon(Icons.disc_full)),
      );
      expect(icon.size, 16);
    });
  });

  group('#409 R4 — the overflow-menu labels', () {
    testWidgets('the appearance + security entries wrap like their five '
        'siblings (de at 320dp — "Erscheinungsbild" overflowed at 1.0x)', (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(const Size(320, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        _harness(
          session: FakeWalletSession(
            current: const SyncStatus.upToDate(tip: 100),
            snapshotValue: walletStateFixture(),
          ),
          locale: const Locale('de'),
          extraOverrides: [
            // Both entries are seam-gated: appearance renders only when the
            // host wired a route, security only under package custody.
            walletAppearanceRoutePathProvider.overrideWithValue('/appearance'),
            walletProvisionerProvider.overrideWithValue(
              FakeWalletProvisioner(),
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();
      final l10n = _l10nAt(tester);

      await tester.tap(find.byIcon(Icons.more_vert));
      await tester.pumpAndSettle();

      expect(find.text(l10n.walletAppearanceMenuItem), findsOneWidget);
      expect(find.text(l10n.walletSecurityMenuItem), findsOneWidget);
      expect(
        tester.takeException(),
        isNull,
        reason: 'a bare Text in the menu Row overflowed at de/1.0x',
      );
    });
  });
}

class _StubRescan extends WalletRescanController {
  _StubRescan(this._initial);
  final WalletRescanState _initial;
  int dismissCount = 0;

  @override
  WalletRescanState build() => _initial;

  @override
  void dismissFailure() {
    dismissCount++;
    state = const WalletRescanIdle();
  }
}
