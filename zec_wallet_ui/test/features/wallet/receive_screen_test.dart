import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/receive_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/address_text.dart';

import 'package:zec_wallet_ui/testing.dart';

import '../../support/a11y_activation.dart';

/// Receive-screen widget tests: the address is rendered for copy/share and the
/// copy action confirms — the screen's whole job is to surface the wallet's
/// public receive address honestly (never logged, §5.4; selectable + copyable).
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(ReceiveScreen)));

Widget _harness({
  required FakeWalletSession? session,
  ThemeData? theme,
  double? textScale,
}) {
  return ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session)],
    child: MaterialApp(
      // Default to the real app theme so the WalletColors extension is present
      // (the QR glow + AddressText read it, like the live app always has it).
      theme: theme ?? lightTheme,
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      // Force a text scale BELOW the MaterialApp (which rebuilds MediaQuery from the
      // view) via the builder — for the large-text-scale accessibility edge.
      builder: textScale == null
          ? null
          : (context, child) => MediaQuery.withClampedTextScaling(
              minScaleFactor: textScale,
              maxScaleFactor: textScale,
              child: child!,
            ),
      home: const ReceiveScreen(),
    ),
  );
}

/// A realistic full-length mainnet transparent P2PKH address (35 chars: `t1` + 33
/// base58) for the transparent-path money-correctness test — the QR + clipboard must
/// carry it verbatim, exactly like the shielded [_kFullUa] pin.
const _kFullTAddr = 't1Pd3TK5rW8U62eKaHjBaB7gMzGjRJKZ6n';

/// A realistic full-length mainnet unified address (~213 chars) for the
/// money-correctness tests: the QR AND the clipboard must carry it in its
/// ENTIRETY — a truncated address sends funds to nowhere (spec §3.3a). One
/// shared constant so both money paths stress the SAME realistic string.
const _kFullUa =
    'u1qp5q9x7n2m4k8j3h6g0fwe2rtyu1qp5q9x7n2m4k8j3h6g0fwe2rtyu1qp5q9x7n2m4k8j3'
    'h6g0fwe2rtyu1qp5q9x7n2m4k8j3h6g0fwe2rtyu1qp5q9x7n2m4k8j3h6g0fwe2rtyu1abcd';

void main() {
  testWidgets('renders the wallet address + a QR + a copy control', (
    tester,
  ) async {
    final fake = FakeWalletSession()
      ..currentAddressResult = 'u1receivetestaddresslowercase';
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();

    // The address shows via AddressText (bold ends, muted middle) — its spans
    // mean find.text won't match; assert the exact full address it carries.
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      'u1receivetestaddresslowercase',
    );
    expect(
      find.byType(QrImageView),
      findsOneWidget,
      reason: 'Recv-1: the QR renders',
    );
    expect(find.text(_l10n(tester).walletReceiveCopy), findsOneWidget);
    expect(fake.currentAddressCount, 1);
  });

  testWidgets(
    'each address type derives ONCE per identity — toggling away and back '
    'renders instantly from the held value, no repeated "Preparing your '
    'address…" (#385 E2E-1). NOTE: an INVARIANT pin, not a regression pin — '
    'the pre-#385 plain provider also cached per container in-package '
    '(S196-c review); what #385 adds is the identity-fenced BY-CONSTRUCTION '
    'shape (the discriminating test is the fence test), and the device '
    'symptom is re-verified on the #340 pass.',
    (tester) async {
      final fake = FakeWalletSession()
        ..currentAddressResult = 'u1cachedshielded'
        ..currentTransparentAddressResult = _kFullTAddr;
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      // First visit to the transparent tab: an honest derive.
      await tester.tap(find.text(l10n.walletReceiveTypeTransparent));
      await tester.pumpAndSettle();
      expect(fake.currentTransparentAddressCount, 1);
      // Away and back — the cached-address shape the device walk saw
      // re-deriving with a multi-second interim on every switch.
      await tester.tap(find.text(l10n.walletReceiveTypeShielded));
      await tester.pumpAndSettle();
      await tester.tap(find.text(l10n.walletReceiveTypeTransparent));
      await tester.pump();
      expect(
        find.text(l10n.walletReceivePreparing),
        findsNothing,
        reason: 'the cached address renders on the FIRST frame back',
      );
      expect(
        tester.widget<AddressText>(find.byType(AddressText)).address,
        _kFullTAddr,
      );
      await tester.pumpAndSettle();
      expect(
        fake.currentTransparentAddressCount,
        1,
        reason: 'one derive per session per type (#385)',
      );
      expect(fake.currentAddressCount, 1, reason: 'the shielded twin too');
    },
  );

  testWidgets('the QR encodes the EXACT full address (money-correctness)', (
    tester,
  ) async {
    // Recv-1 money-correctness (spec §3.3a): the bytes the QR encodes MUST equal
    // currentAddress() in their entirety — a QR that truncates/normalizes one
    // character sends funds to nowhere. A real mainnet unified address is
    // ~213 chars; pin that the whole thing crosses into the QR payload.
    const ua = _kFullUa;
    final fake = FakeWalletSession()..currentAddressResult = ua;
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();

    // The QR widget is keyed by the SAME untransformed payload it encodes, so a
    // found-by-full-address key proves the entire address reached the QR (no
    // truncation/formatting slipped in between). qr_flutter hides its `data`
    // field, so the shared-variable key is the observable seam.
    final keyed = find.byKey(const ValueKey<String>(ua));
    expect(
      keyed,
      findsOneWidget,
      reason: 'the QR must carry the entire ${ua.length}-char address verbatim',
    );
    expect(tester.widget(keyed), isA<QrImageView>());
    // and the same string is what the screen displays for copy/select.
    expect(tester.widget<AddressText>(find.byType(AddressText)).address, ua);
  });

  testWidgets('copy puts the address on the clipboard + confirms', (
    tester,
  ) async {
    // Capture the platform clipboard write (no real clipboard in a widget test).
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

    final fake = FakeWalletSession()..currentAddressResult = 'u1copyme';
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();

    // The address-type toggle pushes the copy button below the fold on a small
    // test viewport — scroll it into view before tapping (the scrollable body).
    await tester.ensureVisible(find.text(_l10n(tester).walletReceiveCopy));
    await tester.tap(find.text(_l10n(tester).walletReceiveCopy));
    await tester.pumpAndSettle();

    expect(copied, 'u1copyme');
    expect(find.text(_l10n(tester).walletReceiveCopied), findsOneWidget);
  });

  testWidgets('a lookup failure shows an honest error, no crash', (
    tester,
  ) async {
    final fake = FakeWalletSession()..currentAddressThrows = StateError('boom');
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();

    expect(find.text(_l10n(tester).walletReceiveError), findsOneWidget);
  });

  testWidgets('the address error offers a Try-again that re-derives the address', (
    tester,
  ) async {
    // No dead-ends on a money surface (honest degradation, invariant 6): a
    // failed load must be recoverable in place, never a stuck screen. The first
    // derive throws; tapping Try again invalidates the provider and the second
    // derive (now succeeding) renders the address.
    final fake = FakeWalletSession()..currentAddressThrows = StateError('boom');
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    expect(find.text(_l10n(tester).walletReceiveError), findsOneWidget);

    // The next derive succeeds — the retry must clear the error and show it.
    fake.currentAddressThrows = null;
    await tester.tap(find.byKey(const Key('receive-address-retry')));
    await tester.pumpAndSettle();

    expect(find.text(_l10n(tester).walletReceiveError), findsNothing);
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      fake.currentAddressResult,
    );
    expect(fake.currentAddressCount, greaterThanOrEqualTo(2));
  });

  testWidgets('the transparent-address error Try-again re-derives the t-address', (
    tester,
  ) async {
    // The retry must invalidate the SELECTED provider — the transparent branch,
    // not just the shielded one. A copy-paste swapping the two would leave this
    // still erroring after a tap (the shielded-only test wouldn't catch it).
    final fake = FakeWalletSession()
      ..currentTransparentAddressThrows = StateError('boom');
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    // Switch to the transparent tab — its provider throws → the error state.
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
    await tester.pumpAndSettle();
    expect(find.text(_l10n(tester).walletReceiveError), findsOneWidget);

    fake.currentTransparentAddressThrows = null;
    await tester.tap(find.byKey(const Key('receive-address-retry')));
    await tester.pumpAndSettle();

    expect(find.text(_l10n(tester).walletReceiveError), findsNothing);
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      fake.currentTransparentAddressResult,
    );
    expect(fake.currentTransparentAddressCount, greaterThanOrEqualTo(2));
  });

  testWidgets(
    'no session shows the not-ready state, not an error (deep-link)',
    (tester) async {
      await tester.pumpWidget(_harness(session: null));
      await tester.pumpAndSettle();
      // A null session is "not set up yet", distinct from a lookup failure.
      expect(find.text(_l10n(tester).walletReceiveUnavailable), findsOneWidget);
      expect(find.text(_l10n(tester).walletReceiveError), findsNothing);
    },
  );

  testWidgets('a full-length unified address is copied in its entirety', (
    tester,
  ) async {
    // Money-correctness: a real mainnet UA is ~213 chars. The display must not
    // truncate what reaches the clipboard — a truncated address sends funds to
    // nowhere. Pins the full string crosses to Clipboard.setData.
    const ua = _kFullUa;
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

    final fake = FakeWalletSession()..currentAddressResult = ua;
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    expect(tester.widget<AddressText>(find.byType(AddressText)).address, ua);

    // The QR + full-length address push the copy button below the fold in the
    // default test viewport; scroll it into view before tapping.
    await tester.ensureVisible(find.text(_l10n(tester).walletReceiveCopy));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_l10n(tester).walletReceiveCopy));
    await tester.pumpAndSettle();
    expect(
      copied,
      ua,
      reason: 'the entire ${ua.length}-char address is copied',
    );
  });

  testWidgets(
    'the QR tile stays white (scannable) in dark, light, and AMOLED themes',
    (tester) async {
      // Re-review real-world edge (AMOLED is the dangerous one): the AMOLED bg is
      // pure black, so if the QR tile ever drew its background from a theme token
      // instead of the pinned white, the QR would be black-on-black and unscannable
      // — funds unreachable. Pin that the scanner-required white survives EVERY
      // theme (flutter-patterns: test all modes before "done").
      for (final theme in [darkTheme, lightTheme, amoledDarkTheme]) {
        final fake = FakeWalletSession()..currentAddressResult = _kFullUa;
        await tester.pumpWidget(_harness(session: fake, theme: theme));
        await tester.pumpAndSettle();
        expect(find.byType(QrImageView), findsOneWidget);
        final tile = tester.widget<Container>(
          find.byKey(const Key('receive-qr-tile')),
        );
        expect(
          (tile.decoration as BoxDecoration).color,
          const Color(0xFFFFFFFF),
          reason: 'the QR tile must stay white to scan in every theme',
        );
      }
    },
  );

  testWidgets(
    'a wedged address load times out into the honest error, not an infinite spinner',
    (tester) async {
      // Mobile/unstable real-world edge: currentAddress() is a LOCAL derivation,
      // so a hang means a wedged FFI boundary. The provider's timeout must convert
      // an infinite spinner on a money surface into the recoverable error state.
      final fake = FakeWalletSession()..currentAddressNeverCompletes = true;
      await tester.pumpWidget(_harness(session: fake));
      await tester.pump();
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      // Pin the honest loading copy (a mis-wired l10n key would still pass the
      // spinner assertion while the user sees an empty headline).
      expect(find.text(_l10n(tester).walletReceivePreparing), findsOneWidget);
      expect(find.text(_l10n(tester).walletReceiveError), findsNothing);

      // Advance the test clock past the bound — the timeout fires, the screen
      // degrades honestly (no infinite spinner).
      await tester.pump(
        walletAddressDeriveTimeout + const Duration(seconds: 1),
      );
      await tester.pumpAndSettle();
      expect(find.text(_l10n(tester).walletReceiveError), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsNothing);
    },
  );

  testWidgets('a full-length address renders without a layout overflow', (
    tester,
  ) async {
    // A 213-char address on a small phone surface must not RenderFlex-overflow
    // or clip (the address card scrolls, the text wraps). Guards a future
    // refactor that puts the address in an unconstrained Row.
    await tester.binding.setSurfaceSize(const Size(320, 480));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final fake = FakeWalletSession()..currentAddressResult = _kFullUa;
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    expect(
      tester.takeException(),
      isNull,
      reason: 'no overflow with a 213-char address',
    );
    expect(find.byType(QrImageView), findsOneWidget);
    expect(find.text(_kFullUa), findsOneWidget);
  });

  testWidgets(
    'the copy button is an independent semantics node (no iOS merge)',
    (tester) async {
      // flutter-patterns § iOS Semantics merging: container:true keeps the copy
      // button from merging into the SelectableText address into one element (a
      // VoiceOver tap would otherwise retarget to the text and never fire copy).
      final handle = tester.ensureSemantics();
      final fake = FakeWalletSession()..currentAddressResult = _kFullUa;
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      expect(
        find.bySemanticsLabel(_l10n(tester).walletReceiveCopy),
        findsOneWidget,
      );
      handle.dispose();
    },
  );

  // The row above asserts a LABEL exists, which it did throughout the life of
  // the defect below it. `excludeSemantics: true` dropped the FilledButton's
  // own node, its tap action with it, so the exported node advertised a button
  // a screen reader could not activate. Contract: `test/support/a11y_activation.dart`.
  testWidgets(
    'the copy button is ACTIVATABLE by a screen reader, and the semantics '
    'action puts the full address on the clipboard',
    (tester) async {
      final handle = tester.ensureSemantics();
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

      final fake = FakeWalletSession()..currentAddressResult = _kFullUa;
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();

      // NOT `tester.tap` — a pointer event hits the live FilledButton beneath
      // and never consults the semantics tree, so it passes over the defect.
      expectActivatable(
        tester,
        find.semantics.byLabel(_l10n(tester).walletReceiveCopy),
        reason: 'copying the receive address is the screen\'s whole job',
      );
      await tester.pumpAndSettle();

      // The action is wired to the SAME callback the finger runs: the whole
      // address, and the confirmation.
      expect(copied, _kFullUa);
      expect(find.text(_l10n(tester).walletReceiveCopied), findsOneWidget);
      handle.dispose();
    },
  );

  // ── Recv-2 (ADR-0528): the SHIELDED ⇄ TRANSPARENT address-type toggle ──────

  testWidgets('the address-type toggle defaults to shielded', (tester) async {
    // Recv-2 named test `transparent_address_toggle_defaults_to_shielded`: on first
    // render the SHIELDED address is shown (the recommended default), NOT the
    // transparent one — the user must opt into the public address. Proven by the
    // shielded address being present, the transparent absent, and only the shielded
    // provider having been read.
    final fake = FakeWalletSession()
      ..currentAddressResult = 'u1shieldeddefault'
      ..currentTransparentAddressResult = 't1transparentoptin';
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();

    expect(
      find.text('u1shieldeddefault'),
      findsOneWidget,
      reason: 'the shielded address is the default',
    );
    expect(
      find.text('t1transparentoptin'),
      findsNothing,
      reason: 'the transparent address is NOT shown until opted into',
    );
    expect(
      find.byKey(const Key('receive-transparent-warning')),
      findsNothing,
      reason: 'no public-address warning on the shielded default',
    );
    expect(fake.currentAddressCount, 1);
    expect(
      fake.currentTransparentAddressCount,
      0,
      reason: 'the transparent address is not even fetched by default',
    );
  });

  testWidgets('toggling to transparent shows the t-address + the PUBLIC warning', (
    tester,
  ) async {
    // Opting into Transparent swaps the rendered address to the t-addr, shows the
    // honest PUBLIC warning (never hidden — §3.3a), and the QR re-keys to the
    // transparent payload (money-correctness across the toggle).
    final fake = FakeWalletSession()
      ..currentAddressResult = 'u1shieldeddefault'
      ..currentTransparentAddressResult = 't1transparentoptin';
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();

    // tap the Transparent segment
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
    await tester.pumpAndSettle();

    expect(
      find.text('t1transparentoptin'),
      findsOneWidget,
      reason: 'the transparent address is now shown',
    );
    expect(
      find.text('u1shieldeddefault'),
      findsNothing,
      reason: 'the shielded address is swapped out',
    );
    expect(
      find.byKey(const Key('receive-transparent-warning')),
      findsOneWidget,
      reason:
          'the PUBLIC honest-framing warning is shown for the transparent address',
    );
    // pin the actual honest-framing COPY (not just the key) — a mis-wired l10n key would
    // otherwise pass with a placeholder string (re-review NIT fold).
    expect(
      find.descendant(
        of: find.byKey(const Key('receive-transparent-warning')),
        matching: find.text(_l10n(tester).walletReceiveTransparentWarning),
      ),
      findsOneWidget,
      reason:
          'the warning renders the honest transparent-address copy, not a placeholder',
    );
    // the QR carries the EXACT transparent address (money-correctness across the toggle)
    expect(
      find.byKey(const ValueKey<String>('t1transparentoptin')),
      findsOneWidget,
    );
    expect(fake.currentTransparentAddressCount, 1);

    // toggling back returns to the shielded address (and drops the warning)
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeShielded));
    await tester.pumpAndSettle();
    expect(find.text('u1shieldeddefault'), findsOneWidget);
    expect(find.byKey(const Key('receive-transparent-warning')), findsNothing);
  });

  testWidgets('a wedged transparent-address load times out into the honest error', (
    tester,
  ) async {
    // The transparent path gets the SAME honest-degradation timeout as the shielded
    // one (a wedged FFI derive ⇒ error state, not an infinite spinner on a money
    // surface). Toggle to transparent with a never-completing fake, advance past the
    // timeout, assert the error copy (not a stuck progress indicator).
    final fake = FakeWalletSession()
      ..currentAddressResult = 'u1shieldeddefault'
      ..currentTransparentAddressNeverCompletes = true;
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
    await tester.pump(); // start the future
    await tester.pump(walletAddressDeriveTimeout + const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.text(_l10n(tester).walletReceiveError), findsOneWidget);
    expect(find.byType(CircularProgressIndicator), findsNothing);
    // a11y (#386 made this error reachable; fold): the loading→error
    // transition WHILE the user is on-screen must be ANNOUNCED, like every
    // sibling honest-degradation element — else a TalkBack user keeps hearing
    // "Preparing your address…" through a genuine wedge.
    final handle = tester.ensureSemantics();
    expect(
      tester
          .getSemantics(find.byKey(const Key('receive-address-error')))
          .flagsCollection
          .isLiveRegion,
      isTrue,
      reason: 'the honest address-load error must be a live region',
    );
    handle.dispose();
  });

  testWidgets('copy in transparent mode copies the t-address (money-correctness)', (
    tester,
  ) async {
    // Money-correctness on the TOGGLED path: Copy while viewing the transparent
    // address must put the t-addr on the clipboard, not the shielded UA. A refactor
    // that fed the wrong provider's value to the copy handler would pass the shielded
    // copy tests but be caught here.
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

    final fake = FakeWalletSession()
      ..currentAddressResult = 'u1shieldeddefault'
      ..currentTransparentAddressResult = 't1copythetransparentone';
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
    await tester.pumpAndSettle();

    await tester.ensureVisible(find.text(_l10n(tester).walletReceiveCopy));
    await tester.tap(find.text(_l10n(tester).walletReceiveCopy));
    await tester.pumpAndSettle();
    expect(
      copied,
      't1copythetransparentone',
      reason: 'Copy in transparent mode must copy the t-address verbatim',
    );
  });

  testWidgets('the transparent layout renders without overflow on a small screen', (
    tester,
  ) async {
    // The transparent view adds the PUBLIC warning banner — the highest-risk extra
    // height on a small viewport. Render it at 320×480 with a full-length t-addr and
    // assert no layout overflow (the body scrolls).
    await tester.binding.setSurfaceSize(const Size(320, 480));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final fake = FakeWalletSession()
      ..currentAddressResult = 'u1shieldeddefault'
      ..currentTransparentAddressResult =
          _kFullUa; // a long string in the t-slot
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key('receive-transparent-warning')),
      findsOneWidget,
    );
    expect(
      tester.takeException(),
      isNull,
      reason:
          'the transparent view (warning + QR + long address) must not overflow',
    );
  });

  // ── re-review round: transparent-path error + lifecycle real-world edges ──────

  testWidgets('a transparent address lookup failure shows an honest error', (
    tester,
  ) async {
    // Error-path parity with the shielded test: a (non-timeout) throw from the
    // transparent FFI call ⇒ the honest error state, and NO public-address warning
    // (we never show the warning over a failed load).
    final fake = FakeWalletSession()
      ..currentAddressResult = 'u1shieldeddefault'
      ..currentTransparentAddressThrows = StateError('bridge error');
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
    await tester.pumpAndSettle();
    expect(find.text(_l10n(tester).walletReceiveError), findsOneWidget);
    expect(find.byKey(const Key('receive-transparent-warning')), findsNothing);
  });

  testWidgets('a transparent address error persists after toggling away and back', (
    tester,
  ) async {
    // Real-world-edge: after the transparent load errors, toggling to shielded and
    // BACK must still show the error (the cached AsyncError), never a spinner or a
    // blank — a regression that re-triggered the future on every toggle would slip past
    // the other tests.
    final fake = FakeWalletSession()
      ..currentAddressResult = 'u1shieldeddefault'
      ..currentTransparentAddressThrows = StateError('bridge error');
    await tester.pumpWidget(_harness(session: fake));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
    await tester.pumpAndSettle();
    expect(find.text(_l10n(tester).walletReceiveError), findsOneWidget);
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeShielded));
    await tester.pumpAndSettle();
    expect(find.text('u1shieldeddefault'), findsOneWidget);
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
    await tester.pumpAndSettle();
    expect(
      find.text(_l10n(tester).walletReceiveError),
      findsOneWidget,
      reason: 'the cached error must survive a toggle-away and retoggle',
    );
    expect(find.byType(CircularProgressIndicator), findsNothing);
  });

  testWidgets(
    'the transparent QR encodes the EXACT full t-address (money-correctness)',
    (tester) async {
      // The shielded money-correctness pin (the full ~213-char UA) has a transparent
      // counterpart: a realistic 35-char mainnet P2PKH must reach the QR verbatim — a
      // truncated t-addr sends a refund/CEX withdrawal to nowhere.
      final fake = FakeWalletSession()
        ..currentAddressResult = 'u1shieldeddefault'
        ..currentTransparentAddressResult = _kFullTAddr;
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pumpAndSettle();
      final keyed = find.byKey(const ValueKey<String>(_kFullTAddr));
      expect(
        keyed,
        findsOneWidget,
        reason: 'the QR must carry the entire t-address verbatim',
      );
      expect(tester.widget(keyed), isA<QrImageView>());
      expect(find.text(_kFullTAddr), findsOneWidget);
    },
  );

  testWidgets(
    'toggling to transparent while shielded is still loading shows the t-address',
    (tester) async {
      // The shielded load hangs (never completes); toggling to transparent must switch to
      // the resolved transparent provider — the shielded spinner must NOT leak onto the
      // transparent tab (independent providers, no shared state).
      final fake = FakeWalletSession()
        ..currentAddressNeverCompletes = true
        ..currentTransparentAddressResult = 't1transparentreadynow';
      await tester.pumpWidget(_harness(session: fake));
      await tester.pump(); // shielded future starts; spinner shows
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pumpAndSettle();
      expect(
        find.text('t1transparentreadynow'),
        findsOneWidget,
        reason:
            'the transparent address renders even though the shielded load is wedged',
      );
      expect(find.byType(CircularProgressIndicator), findsNothing);
      // drain the still-pending shielded timeout timer (the wedged future we toggled away
      // from) so the test ends with no dangling timer.
      await tester.pump(
        walletAddressDeriveTimeout + const Duration(seconds: 1),
      );
    },
  );

  testWidgets(
    'the transparent warning + toggle render without overflow at 3x text scale',
    (tester) async {
      // Accessibility real-world-edge: at a large system text scale the honest-framing
      // warning is the highest-risk element (it MUST stay fully visible — a clipped
      // disclosure is dishonest). Render the transparent view at 3x on a small screen and
      // assert no overflow.
      await tester.binding.setSurfaceSize(const Size(360, 740));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final fake = FakeWalletSession()
        ..currentAddressResult = 'u1shieldeddefault'
        ..currentTransparentAddressResult = _kFullTAddr;
      await tester.pumpWidget(_harness(session: fake, textScale: 3.0));
      await tester.pumpAndSettle();
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key('receive-transparent-warning')),
        findsOneWidget,
      );
      expect(
        tester.takeException(),
        isNull,
        reason: 'warning + toggle must not overflow at 3x text scale',
      );
    },
  );

  testWidgets(
    'the transparent warning is a live region (announced on appearance)',
    (tester) async {
      // a11y: the PUBLIC warning is an honest-degradation element a screen-reader user
      // must hear when it appears on toggle — assert the `liveRegion` semantics flag.
      final handle = tester.ensureSemantics();
      final fake = FakeWalletSession()
        ..currentAddressResult = 'u1shieldeddefault'
        ..currentTransparentAddressResult = 't1transparentoptin';
      await tester.pumpWidget(_harness(session: fake));
      await tester.pumpAndSettle();
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pumpAndSettle();
      expect(
        tester
            .getSemantics(find.byKey(const Key('receive-transparent-warning')))
            .flagsCollection
            .isLiveRegion,
        isTrue,
        reason: 'the public-address warning must be a live region',
      );
      handle.dispose();
    },
  );

  // ── Leave/re-enter retention (#386, the E2E-2 fold) ──────────────
  // The device symptom: leaving the Receive screen mid-first-derive appeared
  // to RESTART the derive on every re-entry, and the honest timeout error
  // never surfaced. Mechanism (probe-6d, riverpod 3.3.2 source-verified):
  // nothing disposes — the view PAUSES, and riverpod's container-default
  // RETRY swallowed the TimeoutException into AsyncLoading(retrying: true),
  // whose invalidate-rebuild was deferred until re-entry (the "restart").
  // These tests navigate for real (push/pop) so the pause semantics are the
  // production shape, and use BOUNDED pumps where an unbounded pumpAndSettle
  // would churn the fake clock through retry cycles and mask the symptom.

  testWidgets(
    'an in-flight transparent derive SURVIVES leave/re-enter — the result '
    'lands; no restart from zero (#386 E2E-2)',
    (tester) async {
      final gate = Completer<void>();
      final fake = FakeWalletSession()
        ..currentAddressResult = 'u1shieldedinstant'
        ..currentTransparentAddressResult = _kFullTAddr
        ..currentTransparentAddressGate = gate;
      await tester.pumpWidget(_navHarness(session: fake));
      await _enterReceive(tester);
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pump();
      expect(find.text(_l10n(tester).walletReceivePreparing), findsOneWidget);
      expect(fake.currentTransparentAddressCount, 1);

      // Leave mid-derive; the engine finishes while nothing is on screen —
      // past the pre-#386 15 s bound (which FALSE-FIRED on exactly this
      // honest 25–45 s catch-up derive) but inside the honest 60 s bound.
      await _leaveReceive(tester);
      await tester.pump(const Duration(seconds: 20));
      gate.complete();
      await tester.pump();

      await _enterReceive(tester);
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pumpAndSettle();
      expect(
        fake.currentTransparentAddressCount,
        1,
        reason:
            'the in-flight derive must survive leave/re-enter — a second '
            'call means 25–45 s of engine work was silently thrown away and '
            'the user pays the full wait again',
      );
      expect(find.text(_l10n(tester).walletReceivePreparing), findsNothing);
      expect(find.text(_l10n(tester).walletReceiveError), findsNothing);
    },
  );

  testWidgets(
    'a timeout landing while AWAY surfaces the honest error on re-entry — '
    'never a silent fresh spinner, never a silent retry (#386 E2E-2)',
    (tester) async {
      final fake = FakeWalletSession()
        ..currentAddressResult = 'u1shieldedinstant'
        ..currentTransparentAddressNeverCompletes = true;
      await tester.pumpWidget(_navHarness(session: fake));
      await _enterReceive(tester);
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pump();
      expect(fake.currentTransparentAddressCount, 1);

      // Leave; the honest-degradation timeout fires while nothing watches.
      await _leaveReceive(tester);
      await tester.pump(
        walletAddressDeriveTimeout + const Duration(seconds: 1),
      );

      // Re-enter with BOUNDED pumps (probe-6d): pre-#386, riverpod's default
      // retry had converted the timeout into AsyncLoading(retrying: true) —
      // a spinner — and only an unbounded settle churning ~10 fake retry
      // cycles would ever reach an error, masking the device's perpetual
      // "Preparing your address…".
      await _enterReceive(tester);
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pump();
      await tester.pump();
      expect(
        find.text(_l10n(tester).walletReceiveError),
        findsOneWidget,
        reason:
            'the timeout must be VISIBLE on re-entry (on device it never '
            'surfaced — the retry swallow)',
      );
      expect(
        find.byType(CircularProgressIndicator),
        findsNothing,
        reason: 'no silent perpetual spinner on a money surface',
      );
      expect(
        fake.currentTransparentAddressCount,
        1,
        reason:
            'no silent retry — the screen\'s Try-again is the ONLY retry '
            '(each silent retry re-queued a fresh FFI derive behind the '
            'contended engine lock)',
      );
    },
  );
}

/// Push/pop navigation harness for the leave/re-enter retention tests — the
/// pause semantics under test only arise when the Receive route actually pops
/// (the plain [_harness] keeps the screen mounted for its whole test).
final _navKey = GlobalKey<NavigatorState>();

Widget _navHarness({required FakeWalletSession session}) {
  return ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session)],
    child: MaterialApp(
      navigatorKey: _navKey,
      theme: lightTheme,
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      home: const Scaffold(body: Center(child: Text('nav-home'))),
    ),
  );
}

Future<void> _enterReceive(WidgetTester tester) async {
  _navKey.currentState!.push(
    MaterialPageRoute<void>(builder: (_) => const ReceiveScreen()),
  );
  await tester.pumpAndSettle();
}

Future<void> _leaveReceive(WidgetTester tester) async {
  _navKey.currentState!.pop();
  await tester.pumpAndSettle();
}
