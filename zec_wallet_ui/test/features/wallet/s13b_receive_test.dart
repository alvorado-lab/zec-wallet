import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/receive_screen.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_ui_config.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/shared/qr_tile.dart';

import 'package:zec_wallet_ui/testing.dart';

/// S13 Build B, the receive screen (plan §1.7): the paragraphs behind their
/// (i), Copy tonal, Request amount (the QR is `composePaymentUri`'s output),
/// and Share through the host's optional hook.
WalletLocalizations _l10n(WidgetTester tester) =>
    WalletLocalizations.of(tester.element(find.byType(ReceiveScreen)));

const _kAddr = 'u1s13breceiveaddressfortheshareandrequesttests';
const _kTAddr = 't1Pd3TK5rW8U62eKaHjBaB7gMzGjRJKZ6n';

Widget _harness({
  required FakeWalletSession session,
  WalletUiConfig config = const WalletUiConfig(),
}) {
  return ProviderScope(
    overrides: [
      walletSessionProvider.overrideWithValue(session),
      walletUiConfigProvider.overrideWithValue(config),
    ],
    child: MaterialApp(
      theme: lightTheme,
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      home: const ReceiveScreen(),
    ),
  );
}

FakeWalletSession _session() => FakeWalletSession()
  ..currentAddressResult = _kAddr
  ..currentTransparentAddressResult = _kTAddr;

String _qrPayload(WidgetTester tester) =>
    tester.widget<QrTile>(find.byType(QrTile)).payload;

final _requestButton = find.byKey(const Key('receive-request-amount'));
final _amountField = find.byKey(const Key('receive-request-amount-field'));
final _shareButton = find.byKey(const Key('receive-share'));
final _copyButton = find.byKey(const Key('receive-copy'));

Future<void> _openRequest(WidgetTester tester) async {
  await tester.ensureVisible(_requestButton);
  await tester.tap(_requestButton);
  await tester.pumpAndSettle();
}

Future<void> _enterAmount(WidgetTester tester, String text) async {
  await tester.enterText(_amountField, text);
  await tester.pumpAndSettle();
}

/// Records what the host's share hook was handed.
class _ShareRecorder {
  final calls = <(String, String?)>[];
  Object? throwWith;

  Future<void> call(String text, {String? subject}) async {
    calls.add((text, subject));
    final t = throwWith;
    if (t != null) throw t;
  }
}

/// Opens an (i) and returns whether [body] is shown in its sheet.
Future<void> _expectInfo(
  WidgetTester tester,
  Finder button,
  String body,
) async {
  expect(find.text(body), findsNothing, reason: 'not on screen before (i)');
  await tester.ensureVisible(button);
  await tester.tap(button);
  await tester.pumpAndSettle();
  expect(find.text(body), findsOneWidget, reason: 'the (i) opens it whole');
  await tester.tap(find.byKey(const ValueKey('wallet-info-close')));
  await tester.pumpAndSettle();
}

void main() {
  group('paragraphs behind (i)', () {
    testWidgets(
      'the address explanation sits behind the toggle (i), per type',
      (tester) async {
        await tester.pumpWidget(_harness(session: _session()));
        await tester.pumpAndSettle();
        final l10n = _l10n(tester);
        final info = find.byKey(const Key('receive-address-info'));
        expect(
          tester.getSemantics(info).label,
          l10n.walletInfoButtonLabel(l10n.walletReceiveTypeShielded),
        );
        await _expectInfo(tester, info, l10n.walletReceiveSubtitle);

        await tester.tap(find.text(l10n.walletReceiveTypeTransparent));
        await tester.pumpAndSettle();
        expect(
          tester.getSemantics(info).label,
          l10n.walletInfoButtonLabel(l10n.walletReceiveTypeTransparent),
        );
        await _expectInfo(tester, info, l10n.walletReceiveSubtitleTransparent);
        // The transparent warning is a STATE: it stays on screen.
        expect(
          find.byKey(const Key('receive-transparent-warning')),
          findsOneWidget,
        );
      },
    );

    testWidgets('the preparing hint sits behind the headline (i)', (
      tester,
    ) async {
      final session = _session()..currentAddressNeverCompletes = true;
      await tester.pumpWidget(_harness(session: session));
      await tester.pump();
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletReceivePreparing), findsOneWidget);
      await _expectInfo(
        tester,
        find.byKey(const Key('receive-preparing-info')),
        l10n.walletReceivePreparingHint,
      );
      // Drain the wedged derive's timeout timer.
      await tester.pump(
        walletAddressDeriveTimeout + const Duration(seconds: 1),
      );
    });

    testWidgets('the fresh note keeps "copy it now" on screen and moves the '
        'explanation behind its (i)', (tester) async {
      final session = _session()..mintDiversifiedAddressResult = 'u1minted';
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      final fresh = find.byKey(const Key('receive-fresh-address'));
      await tester.ensureVisible(fresh);
      await tester.tap(fresh);
      await tester.pumpAndSettle();
      final l10n = _l10n(tester);
      expect(find.text(l10n.walletReceiveFreshCopyNow), findsOneWidget);
      await _expectInfo(
        tester,
        find.byKey(const Key('receive-fresh-info')),
        l10n.walletReceiveFreshCaption,
      );
    });
  });

  testWidgets('Copy is tonal', (tester) async {
    await tester.pumpWidget(_harness(session: _session()));
    await tester.pumpAndSettle();
    final scheme = Theme.of(tester.element(_copyButton)).colorScheme;
    final fill = tester
        .widget<Material>(
          find.descendant(of: _copyButton, matching: find.byType(Material)),
        )
        .color;
    expect(fill, scheme.secondaryContainer);
    expect(fill, isNot(scheme.primary));
  });

  group('Request amount', () {
    testWidgets('no amount: the QR is the plain address, and the field is '
        'drawn only on request', (tester) async {
      await tester.pumpWidget(_harness(session: _session()));
      await tester.pumpAndSettle();
      expect(_amountField, findsNothing);
      expect(_qrPayload(tester), _kAddr);
      await _openRequest(tester);
      expect(_amountField, findsOneWidget);
      expect(
        find.text(_l10n(tester).walletReceiveRequestAmountLabel),
        findsOneWidget,
      );
      expect(_qrPayload(tester), _kAddr);
    });

    testWidgets('an amount makes the QR and the copied text exactly '
        'composePaymentUri(address, amount)', (tester) async {
      final session = _session();
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      await _openRequest(tester);
      await _enterAmount(tester, '1.5');
      final expected = session.composePaymentUri(
        recipient: _kAddr,
        amountZat: 150000000,
      );
      expect(_qrPayload(tester), expected);

      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied =
                (call.arguments as Map<Object?, Object?>)['text'] as String?;
          }
          return null;
        },
      );
      await tester.ensureVisible(_copyButton);
      await tester.tap(_copyButton);
      await tester.pumpAndSettle();
      expect(copied, expected);

      // Clearing the amount returns to the plain address.
      await _enterAmount(tester, '');
      expect(_qrPayload(tester), _kAddr);
    });

    testWidgets('the amount field takes a comma as the point (the shared '
        'formatter)', (tester) async {
      final session = _session();
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      await _openRequest(tester);
      await _enterAmount(tester, '0,5');
      expect(
        _qrPayload(tester),
        session.composePaymentUri(recipient: _kAddr, amountZat: 50000000),
      );
    });

    testWidgets('the request follows the address on screen (transparent tab)', (
      tester,
    ) async {
      final session = _session();
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      await _openRequest(tester);
      await _enterAmount(tester, '2');
      await tester.tap(find.text(_l10n(tester).walletReceiveTypeTransparent));
      await tester.pumpAndSettle();
      expect(
        _qrPayload(tester),
        session.composePaymentUri(recipient: _kTAddr, amountZat: 200000000),
      );
    });

    testWidgets('the request follows a fresh address: after the mint the QR '
        'and the copied text are composePaymentUri(the NEW address, amount)', (
      tester,
    ) async {
      const minted = 'u1s13bfreshlymintedaddressfortherequest';
      final session = _session()..mintDiversifiedAddressResult = minted;
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      await _openRequest(tester);
      await _enterAmount(tester, '0.75');
      expect(
        _qrPayload(tester),
        session.composePaymentUri(recipient: _kAddr, amountZat: 75000000),
      );

      final fresh = find.byKey(const Key('receive-fresh-address'));
      await tester.ensureVisible(fresh);
      await tester.tap(fresh);
      await tester.pumpAndSettle();
      expect(session.mintDiversifiedAddressCount, 1);
      final expected = session.composePaymentUri(
        recipient: minted,
        amountZat: 75000000,
      );
      expect(_qrPayload(tester), expected);

      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied =
                (call.arguments as Map<Object?, Object?>)['text'] as String?;
          }
          return null;
        },
      );
      await tester.ensureVisible(_copyButton);
      await tester.tap(_copyButton);
      await tester.pumpAndSettle();
      expect(copied, expected);
    });

    testWidgets('an amount the encoder refuses is never silently dropped: the '
        'QR is the plain address and the field says the payment was not '
        'built', (tester) async {
      final session = _session()..composeThrows = StateError('refused');
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      await _openRequest(tester);
      await _enterAmount(tester, '1.5');
      expect(session.composeCount, greaterThan(0));
      expect(_qrPayload(tester), _kAddr);
      final l10n = _l10n(tester);
      expect(
        tester.widget<TextField>(_amountField).decoration?.errorText,
        l10n.walletSendFaultUriInvalid,
      );
      expect(find.text(l10n.walletSendFaultUriInvalid), findsOneWidget);

      // Clearing the amount clears the error with it.
      await _enterAmount(tester, '');
      expect(
        tester.widget<TextField>(_amountField).decoration?.errorText,
        isNull,
      );
    });

    for (final bad in ['.', '0', '0.123456789', '99999999']) {
      testWidgets('a malformed amount "$bad" changes nothing and says why', (
        tester,
      ) async {
        final session = _session();
        await tester.pumpWidget(_harness(session: session));
        await tester.pumpAndSettle();
        await _openRequest(tester);
        await _enterAmount(tester, bad);
        expect(_qrPayload(tester), _kAddr);
        final field = tester.widget<TextField>(_amountField);
        expect(field.decoration?.errorText, isNotNull);
      });
    }

    testWidgets('a refused edit (a second separator) leaves the request as it '
        'was', (tester) async {
      final session = _session();
      await tester.pumpWidget(_harness(session: session));
      await tester.pumpAndSettle();
      await _openRequest(tester);
      await _enterAmount(tester, '1.5');
      final before = _qrPayload(tester);
      await _enterAmount(tester, '1.5.');
      expect(_qrPayload(tester), before);
    });
  });

  group('Share', () {
    testWidgets('no hook, no Share', (tester) async {
      await tester.pumpWidget(_harness(session: _session()));
      await tester.pumpAndSettle();
      expect(_shareButton, findsNothing);
      expect(find.text(_l10n(tester).walletReceiveShare), findsNothing);
    });

    testWidgets('the hook receives exactly the address and a null subject, '
        'only from the tap', (tester) async {
      final rec = _ShareRecorder();
      await tester.pumpWidget(
        _harness(
          session: _session(),
          config: WalletUiConfig(onShare: rec.call),
        ),
      );
      await tester.pumpAndSettle();
      expect(rec.calls, isEmpty, reason: 'never without the user\'s tap');
      await tester.ensureVisible(_shareButton);
      await tester.tap(_shareButton);
      await tester.pumpAndSettle();
      expect(rec.calls, [(_kAddr, null)]);
    });

    testWidgets('with an amount the hook receives the request URI, still no '
        'subject', (tester) async {
      final rec = _ShareRecorder();
      final session = _session();
      await tester.pumpWidget(
        _harness(
          session: session,
          config: WalletUiConfig(onShare: rec.call),
        ),
      );
      await tester.pumpAndSettle();
      await _openRequest(tester);
      await _enterAmount(tester, '0.25');
      await tester.ensureVisible(_shareButton);
      await tester.tap(_shareButton);
      await tester.pumpAndSettle();
      expect(rec.calls, [
        (
          session.composePaymentUri(recipient: _kAddr, amountZat: 25000000),
          null,
        ),
      ]);
    });

    for (final throws in [false, true]) {
      testWidgets(
        'nothing is shown after the hook ${throws ? 'throws' : 'completes'}',
        (tester) async {
          final rec = _ShareRecorder()
            ..throwWith = throws ? StateError('dismissed') : null;
          await tester.pumpWidget(
            _harness(
              session: _session(),
              config: WalletUiConfig(onShare: rec.call),
            ),
          );
          await tester.pumpAndSettle();
          final before = tester.allWidgets.length;
          await tester.ensureVisible(_shareButton);
          await tester.tap(_shareButton);
          await tester.pumpAndSettle();
          expect(rec.calls, hasLength(1));
          expect(tester.takeException(), isNull);
          expect(find.byType(SnackBar), findsNothing);
          expect(find.byType(Dialog), findsNothing);
          expect(find.byType(BottomSheet), findsNothing);
          expect(tester.allWidgets.length, before);
          // Still usable: a second tap reaches the hook again.
          await tester.tap(_shareButton);
          await tester.pumpAndSettle();
          expect(rec.calls, hasLength(2));
        },
      );
    }
  });
}
