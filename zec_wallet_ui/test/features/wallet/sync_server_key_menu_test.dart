import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/sync_server_sheet.dart';
import 'package:zec_wallet_ui/features/wallet/wallet_providers.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import 'package:zec_wallet_ui/testing.dart';

/// The picker's key field and the system clipboard (S15 security review, on
/// the iPhone walk's fix): the key is SHOWN by default, and a shown field
/// would offer Copy and Cut — a key on the clipboard outlives the sheet and,
/// on iOS, reaches the user's other devices. Its own file so the picker
/// file's registry citations do not move.
void main() {
  testWidgets('the_key_field_offers_paste_and_never_copy', (tester) async {
    // The clipboard holds text, so the toolbar offers Paste — the control
    // that proves the toolbar opened before the absences are read.
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async => call.method == 'Clipboard.hasStrings'
          ? <String, dynamic>{'value': true}
          : null,
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    final session = FakeWalletSession()
      ..syncServersResult = const [
        SyncServer(
          id: 'zec-rocks',
          label: 'zec.rocks',
          url: 'https://zec.rocks:443',
          authHeader: null,
          authValue: null,
        ),
      ]
      ..syncServerStatusResult = const SyncServerStatus(
        effectiveUrl: 'https://zec.rocks:443',
        defaultUrl: 'https://zec.rocks:443',
        choice: null,
        fallback: null,
      );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [walletSessionProvider.overrideWithValue(session)],
        child: MaterialApp(
          localizationsDelegates: WalletLocalizations.localizationsDelegates,
          supportedLocales: WalletLocalizations.supportedLocales,
          theme: lightTheme,
          home: const Scaffold(body: SyncServerSheet()),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('sync-server-custom-toggle')));
    await tester.pumpAndSettle();
    final key = find.byKey(const ValueKey('sync-server-key'));
    await tester.enterText(key, 'users-key-1');
    await tester.pumpAndSettle();
    await tester.longPress(key);
    await tester.pumpAndSettle();
    final labels = MaterialLocalizations.of(tester.element(key));
    expect(find.text(labels.pasteButtonLabel), findsOneWidget);
    expect(find.text(labels.copyButtonLabel), findsNothing);
    expect(find.text(labels.cutButtonLabel), findsNothing);
    expect(find.text(labels.selectAllButtonLabel), findsNothing);
  });
}
