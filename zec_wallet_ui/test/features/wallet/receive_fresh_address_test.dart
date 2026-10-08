import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/receive_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/address_text.dart';
import 'package:zec_wallet_ui/shared/qr_tile.dart';

import 'package:zec_wallet_ui/testing.dart';

import '../../support/a11y_activation.dart';

/// Recv-4 / FR-8 widget tests: the "Use a fresh address" affordance mints the
/// next diversified UA and switches the WHOLE shielded display (QR + text +
/// copy — the Recv-1 money-correctness invariant) to it, visit-scoped, with
/// honest failure and a11y announcement. The mint itself is pinned Rust-side
/// (the §8 KATs); these tests pin the SCREEN's contract over the port.
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(ReceiveScreen)));

Widget _harness({required FakeWalletSession? session}) {
  return ProviderScope(
    overrides: [walletSessionProvider.overrideWithValue(session)],
    child: MaterialApp(
      theme: lightTheme,
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      home: const ReceiveScreen(),
    ),
  );
}

/// A realistic full-length minted diversified UA — the money-correctness pin:
/// the QR payload, the rendered text, and the clipboard must all carry it in
/// its ENTIRETY after the mint (a truncated address sends funds to nowhere).
const _kMintedUa =
    'u1p6fl75fp46x33tgj56crf9u6rqjk7fr7qp5ltlxmmt8agqflphfawg22jy2kgdx2am7s6h3'
    'zm98hge65j9wqx28fc8ayjea3wk7aefge4cdu2efsjk4am3s6sxz09c7gjl6k4df7t3klnt29'
    '99hv3yhd860ljn8gkdk3qvjh6ud3tzpe';

final _freshButton = find.byKey(const Key('receive-fresh-address'));
final _freshNote = find.byKey(const Key('receive-fresh-note'));

QrTile _qrTile(WidgetTester tester) =>
    tester.widget<QrTile>(find.byType(QrTile));

void main() {
  testWidgets('fresh_address_tap_switches_qr_text_and_copy_to_the_minted_ua', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..mintDiversifiedAddressResult = _kMintedUa;
    await tester.pumpWidget(_harness(session: session));
    await tester.pumpAndSettle();

    // Baseline: the default address renders and the affordance is offered.
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      session.currentAddressResult,
    );
    expect(_freshButton, findsOneWidget);
    expect(_freshNote, findsNothing);

    await tester.ensureVisible(_freshButton);
    await tester.tap(_freshButton);
    await tester.pumpAndSettle();

    // The WHOLE display switched together (Recv-1 money-correctness): text,
    // QR payload, and the fresh note.
    expect(session.mintDiversifiedAddressCount, 1);
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      _kMintedUa,
    );
    expect(_qrTile(tester).payload, _kMintedUa);
    expect(_freshNote, findsOneWidget);
    // S13 §1.7: the "copy it now" state stays on screen; the explanation
    // moved behind the note's (i).
    expect(find.text(_l10n(tester).walletReceiveFreshCopyNow), findsOneWidget);

    // Copy now copies the FULL minted UA verbatim.
    String? copied;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          copied = (call.arguments as Map<Object?, Object?>)['text'] as String?;
        }
        return null;
      },
    );
    await tester.ensureVisible(find.text(_l10n(tester).walletReceiveCopy));
    await tester.tap(find.text(_l10n(tester).walletReceiveCopy));
    await tester.pumpAndSettle();
    expect(copied, _kMintedUa);
  });

  testWidgets('repeated_taps_mint_distinct_addresses', (tester) async {
    final session = FakeWalletSession()
      ..mintDiversifiedAddressResult = _kMintedUa;
    await tester.pumpWidget(_harness(session: session));
    await tester.pumpAndSettle();

    await tester.ensureVisible(_freshButton);
    await tester.tap(_freshButton);
    await tester.pumpAndSettle();
    await tester.ensureVisible(_freshButton);
    await tester.tap(_freshButton);
    await tester.pumpAndSettle();

    // The fake advances per mint (real mint semantics: every call is a NEW
    // address) — the screen renders the SECOND one.
    expect(session.mintDiversifiedAddressCount, 2);
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      '${_kMintedUa}2',
    );
  });

  // BOTH polarities of the mint button's semantics ACTION, in one row, because
  // "every control carries a tap action" is a bar met by pinning `onTap:` on
  // unconditionally — which would leave a screen reader double-tapping a
  // disabled money button that silently ignores it.
  // Contract: `test/support/a11y_activation.dart`.
  testWidgets(
    'fresh_address_button_is_activatable_by_a_screen_reader_and_inert_while_minting',
    (tester) async {
      final handle = tester.ensureSemantics();
      final session = FakeWalletSession()
        ..mintDiversifiedAddressResult = _kMintedUa;
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      final label = _l10n(tester).walletReceiveFreshAddress;

      // ENABLED: the node offers the action, and dispatching it the way an
      // assistive technology does actually mints. `excludeSemantics: true`
      // dropped the OutlinedButton's own action, so before the fix the node
      // carried none and the mint was unreachable without a finger.
      expectActivatable(
        tester,
        find.semantics.byLabel(label),
        reason: 'FR-8: a fresh unlinkable address is the privacy affordance',
      );
      await tester.pumpAndSettle();
      expect(session.mintDiversifiedAddressCount, 1);
      expect(
        tester.widget<AddressText>(find.byType(AddressText)).address,
        _kMintedUa,
      );

      // DISABLED: with a mint in flight the node must still be ANNOUNCED as a
      // button, must say it is disabled, and must offer no action at all.
      final wedged = FakeWalletSession()
        ..mintDiversifiedAddressNeverCompletes = true;
      await tester.pumpWidget(_harness(session: wedged));
      await tester.pumpAndSettle();
      await tester.ensureVisible(_freshButton);
      await tester.tap(_freshButton);
      await tester.pump();
      expectInert(
        tester,
        find.semantics.byLabel(label),
        isButton: true,
        isEnabled: false,
        reason: 'a second mint mid-flight is refused, so do not advertise it',
      );

      // The wedged mint's pending timeout timer must resolve before teardown.
      await tester.pumpAndSettle(const Duration(seconds: 30));
      handle.dispose();
    },
  );

  testWidgets('in_flight_mint_disables_the_button_and_keeps_the_default', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..mintDiversifiedAddressNeverCompletes = true;
    await tester.pumpWidget(_harness(session: session));
    await tester.pumpAndSettle();

    await tester.ensureVisible(_freshButton);
    await tester.tap(_freshButton);
    await tester.pump();

    // Disabled + in-button spinner; the default address stays on screen (no
    // blank money surface mid-mint).
    expect(
      tester.widget<OutlinedButton>(find.byType(OutlinedButton)).enabled,
      isFalse,
    );
    expect(
      find.descendant(
        of: _freshButton,
        matching: find.byType(CircularProgressIndicator),
      ),
      findsOneWidget,
    );
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      session.currentAddressResult,
    );

    // The wedge bound converts a hung FFI into the honest BUSY copy (
    // fold): a tripped timeout usually means a long signing pass holds the DB
    // lock — "busy, retry in a moment", never a false "couldn't create" (and
    // never a forever-spinner; the pending timeout timer resolves, which the
    // test binding requires).
    await tester.pump(walletFfiWedgeTimeout + const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.text(_l10n(tester).walletReceiveFreshBusy), findsOneWidget);
    expect(find.text(_l10n(tester).walletReceiveFreshError), findsNothing);
    expect(
      tester.widget<OutlinedButton>(find.byType(OutlinedButton)).enabled,
      isTrue,
    );
  });

  testWidgets('mint_failure_keeps_the_default_address_and_shows_the_snackbar', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..mintDiversifiedAddressThrows = StateError('typed bridge failure');
    await tester.pumpWidget(_harness(session: session));
    await tester.pumpAndSettle();

    await tester.ensureVisible(_freshButton);
    await tester.tap(_freshButton);
    await tester.pumpAndSettle();

    // Honest, recoverable failure: snackbar copy + the default address stays;
    // the button re-enables for a retry.
    expect(find.text(_l10n(tester).walletReceiveFreshError), findsOneWidget);
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      session.currentAddressResult,
    );
    expect(_freshNote, findsNothing);
    expect(
      tester.widget<OutlinedButton>(find.byType(OutlinedButton)).enabled,
      isTrue,
    );
  });

  testWidgets('minted_display_is_visit_scoped_leave_and_reenter_resets', (
    tester,
  ) async {
    final session = FakeWalletSession()
      ..mintDiversifiedAddressResult = _kMintedUa;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [walletSessionProvider.overrideWithValue(session)],
        child: MaterialApp(
          theme: lightTheme,
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          home: const Scaffold(body: SizedBox()),
        ),
      ),
    );
    final nav = tester.state<NavigatorState>(find.byType(Navigator));
    nav.push(MaterialPageRoute<void>(builder: (_) => const ReceiveScreen()));
    await tester.pumpAndSettle();

    await tester.ensureVisible(_freshButton);
    await tester.tap(_freshButton);
    await tester.pumpAndSettle();
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      _kMintedUa,
    );

    nav.pop();
    await tester.pumpAndSettle();
    nav.push(MaterialPageRoute<void>(builder: (_) => const ReceiveScreen()));
    await tester.pumpAndSettle();

    // Every entry starts at the default address (the [_selected] rule); the
    // minted address keeps working Rust-side — only the display choice resets.
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      session.currentAddressResult,
    );
    expect(_freshNote, findsNothing);
  });

  testWidgets('session_flip_clears_the_minted_display', (tester) async {
    final sessionA = FakeWalletSession()
      ..mintDiversifiedAddressResult = _kMintedUa;
    await tester.pumpWidget(_harness(session: sessionA));
    await tester.pumpAndSettle();
    await tester.ensureVisible(_freshButton);
    await tester.tap(_freshButton);
    await tester.pumpAndSettle();
    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      _kMintedUa,
    );

    // An identity flip while the screen is open: the minted address belongs to
    // the OLD wallet and must not render under the new one.
    final sessionB = FakeWalletSession()
      ..currentAddressResult = 'u1freshwalletdefaultaddressyyyyyyyyyyyyyyyy';
    await tester.pumpWidget(_harness(session: sessionB));
    await tester.pumpAndSettle();

    expect(
      tester.widget<AddressText>(find.byType(AddressText)).address,
      sessionB.currentAddressResult,
    );
    expect(_freshNote, findsNothing);
  });

  testWidgets(
    'transparent_tab_is_untouched_and_the_minted_ua_survives_toggle',
    (tester) async {
      final session = FakeWalletSession()
        ..mintDiversifiedAddressResult = _kMintedUa;
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();

      await tester.ensureVisible(_freshButton);
      await tester.tap(_freshButton);
      await tester.pumpAndSettle();

      // Toggle to transparent: no fresh affordance, no fresh note, the t-addr.
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pumpAndSettle();
      expect(_freshButton, findsNothing);
      expect(_freshNote, findsNothing);
      expect(
        tester.widget<AddressText>(find.byType(AddressText)).address,
        session.currentTransparentAddressResult,
      );

      // Back to shielded: the minted display is retained for THIS visit.
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeShielded));
      await tester.pumpAndSettle();
      expect(
        tester.widget<AddressText>(find.byType(AddressText)).address,
        _kMintedUa,
      );
      expect(_freshNote, findsOneWidget);
    },
  );

  testWidgets('fresh_note_live_region_is_one_shot_per_mint', (tester) async {
    final session = FakeWalletSession()
      ..mintDiversifiedAddressResult = _kMintedUa;
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(_harness(session: session));
    await tester.pumpAndSettle();

    // The button is ONE a11y node with the stable action label.
    expect(
      tester.getSemantics(_freshButton).label,
      _l10n(tester).walletReceiveFreshAddress,
    );

    await tester.ensureVisible(_freshButton);
    await tester.tap(_freshButton);
    // The MINT frame: the note inserts as a liveRegion — the sanctioned
    // announce mechanism (SemanticsService.announce is deprecated) — so a
    // screen-reader hears the display switch.
    await tester.pump();
    expect(
      tester.getSemantics(_freshNote).flagsCollection.isLiveRegion,
      isTrue,
      reason: 'the mint frame must announce (an always-off mutant fails here)',
    );
    expect(
      tester.getSemantics(_freshNote).label,
      contains(_l10n(tester).walletReceiveFreshCopyNow),
    );

    // The post-frame disarm: the SAME note is silent one frame later, so
    // re-inserting it can never re-announce (an always-on mutant fails here —
    // the one-shot contract).
    await tester.pump();
    expect(
      tester.getSemantics(_freshNote).flagsCollection.isLiveRegion,
      isFalse,
    );

    // A transparent⇄shielded toggle re-inserts the note SILENT.
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
    await tester.pumpAndSettle();
    await tester.tap(find.text(_l10n(tester).walletReceiveTypeShielded));
    await tester.pumpAndSettle();
    expect(_freshNote, findsOneWidget);
    expect(
      tester.getSemantics(_freshNote).flagsCollection.isLiveRegion,
      isFalse,
      reason: 'a tab toggle must not re-announce a mint that never happened',
    );

    // A SECOND mint re-arms: the new mint's frame announces again.
    await tester.ensureVisible(_freshButton);
    await tester.tap(_freshButton);
    await tester.pump();
    expect(
      tester.getSemantics(_freshNote).flagsCollection.isLiveRegion,
      isTrue,
      reason: 'each mint announces exactly once',
    );
    semantics.dispose();
  });

  testWidgets('mint_does_not_move_the_copy_or_fresh_buttons', (tester) async {
    // The device walk observed a mis-tap: the fresh note used to insert
    // ABOVE the actions and shift them mid-reach. Pin the fix: the
    // buttons' geometry is IDENTICAL before and after the mint; the note
    // renders BELOW the fresh button. The default address is EQUAL-LENGTH to
    // the minted one (as real UAs are) so the QR/text card cannot resize and
    // the only geometry candidate is the note insertion — the thing under test.
    final session = FakeWalletSession()
      ..currentAddressResult = 'u1${'q' * (_kMintedUa.length - 2)}'
      ..mintDiversifiedAddressResult = _kMintedUa;
    await tester.pumpWidget(_harness(session: session));
    await tester.pumpAndSettle();

    // Scroll the actions into view FIRST — the baseline is the geometry the
    // user's finger is aiming at when they tap.
    await tester.ensureVisible(_freshButton);
    await tester.pumpAndSettle();
    final copyButton = find.byType(FilledButton);
    final copyBefore = tester.getTopLeft(copyButton);
    final freshBefore = tester.getTopLeft(_freshButton);

    await tester.tap(_freshButton);
    await tester.pumpAndSettle();

    expect(tester.getTopLeft(copyButton), copyBefore);
    expect(tester.getTopLeft(_freshButton), freshBefore);
    expect(
      tester.getTopLeft(_freshNote).dy,
      greaterThan(tester.getBottomLeft(_freshButton).dy),
      reason: 'the note lands below the actions, never shifting them',
    );
  });
}
