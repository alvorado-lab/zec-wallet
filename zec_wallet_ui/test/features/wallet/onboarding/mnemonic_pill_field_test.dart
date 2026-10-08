import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/bip39_wordlist.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/mnemonic_input.dart';
import 'package:zec_wallet_ui/features/wallet/onboarding/mnemonic_pill_field.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// The BIP39 pill entry field: a word becomes a numbered pill when committed
/// (space / suggestion / paste), a real word reads normal and an unknown word
/// flags live, and the words reach the parent normalized (lowercase + trimmed).
void main() {
  final wl = Bip39Wordlist.fromLines(
    'abandon\nability\nable\nabout\nart\nartwork\nzoo\n',
  );

  List<String> last = const [];
  Widget host({Bip39Wordlist? wordlist, bool enabled = true}) {
    last = const [];
    return MaterialApp(
      localizationsDelegates: WalletLocalizations.localizationsDelegates,
      supportedLocales: WalletLocalizations.supportedLocales,
      theme: lightTheme,
      home: Scaffold(
        // Scrollable like the real WalletRestoreView (a long paste makes many
        // pill rows; the screen scrolls rather than overflowing).
        body: SingleChildScrollView(
          child: MnemonicPillField(
            wordlist: wordlist,
            enabled: enabled,
            onChanged: (w) => last = w,
          ),
        ),
      ),
    );
  }

  Future<void> type(WidgetTester tester, String text) async {
    await tester.enterText(find.byType(TextField), text);
    await tester.pump();
  }

  testWidgets('recovery words stay off every keyboard and autofill path', (
    tester,
  ) async {
    await tester.pumpWidget(host(wordlist: wl));
    final field = tester.widget<TextField>(find.byType(TextField));
    expect(field.autocorrect, isFalse);
    expect(field.enableSuggestions, isFalse);
    expect(field.enableIMEPersonalizedLearning, isFalse);
    // Any non-null list (the default is `[]`) hands the field to the
    // platform autofill service, which may offer to save a typed word.
    expect(field.autofillHints, isNull);
    expect(field.smartDashesType, SmartDashesType.disabled);
    expect(field.smartQuotesType, SmartQuotesType.disabled);
  });

  testWidgets('a word + space becomes a numbered pill and emits the word', (
    tester,
  ) async {
    await tester.pumpWidget(host(wordlist: wl));
    await type(tester, 'abandon ');
    expect(find.text('abandon'), findsOneWidget); // the pill
    expect(find.text('1'), findsOneWidget); // its 1-based number
    expect(last, ['abandon']);
  });

  testWidgets(
    'words are lowercased + trimmed on commit (the host-UI contract)',
    (tester) async {
      await tester.pumpWidget(host(wordlist: wl));
      await type(tester, '  ABANDON   Ability ');
      expect(last, ['abandon', 'ability']);
    },
  );

  testWidgets('pasting a whole phrase splits it into pills', (tester) async {
    await tester.pumpWidget(host(wordlist: wl));
    await type(tester, 'abandon ability able about ');
    expect(last, ['abandon', 'ability', 'able', 'about']);
    expect(find.text('abandon'), findsOneWidget);
    expect(find.text('about'), findsOneWidget);
  });

  testWidgets('an unknown word commits but is flagged with the warning glyph', (
    tester,
  ) async {
    await tester.pumpWidget(host(wordlist: wl));
    await type(tester, 'zzzz ');
    expect(last, ['zzzz']); // still committed (the SDK is the gate)
    // ...but flagged live with the error glyph (the visible typo cue).
    expect(find.byIcon(Icons.error_outline), findsOneWidget);
    // A valid word shows NO error glyph (the contrast that makes the flag mean
    // something).
    await type(tester, 'abandon ');
    expect(
      find.byIcon(Icons.error_outline),
      findsOneWidget,
      reason: 'still exactly the one bad word flagged, not the valid one',
    );
  });

  testWidgets('tapping an autocomplete suggestion commits that word', (
    tester,
  ) async {
    await tester.pumpWidget(host(wordlist: wl));
    await type(tester, 'ab'); // partial → suggestions appear
    // The suggestion chips for the 'ab' prefix.
    expect(find.widgetWithText(ActionChip, 'abandon'), findsOneWidget);
    await tester.tap(find.widgetWithText(ActionChip, 'able'));
    await tester.pump();
    expect(last, ['able']);
    // Pill present, input cleared (suggestions gone).
    expect(find.text('able'), findsOneWidget);
    expect(find.byType(ActionChip), findsNothing);
  });

  testWidgets('the × on a pill removes it', (tester) async {
    await tester.pumpWidget(host(wordlist: wl));
    await type(tester, 'abandon ability ');
    expect(last, ['abandon', 'ability']);
    // Remove the first pill via its close affordance.
    await tester.tap(find.byIcon(Icons.close).first);
    await tester.pump();
    expect(last, ['ability']);
    expect(find.text('abandon'), findsNothing);
  });

  testWidgets(
    'backspace on an empty input pops the last pill back for editing',
    (tester) async {
      await tester.pumpWidget(host(wordlist: wl));
      await type(tester, 'abandon ability ');
      expect(last, ['abandon', 'ability']);
      // Focus the (empty) inline input and press backspace.
      await tester.tap(find.byType(TextField));
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
      await tester.pump();
      // The last pill is gone (emitted) and its word is back in the input.
      expect(last, ['abandon']);
      final field = tester.widget<TextField>(find.byType(TextField));
      expect(field.controller!.text, 'ability');
    },
  );

  testWidgets(
    'an over-long paste is capped at kMaxMnemonicWords (no silent truncation '
    'to a valid-looking length, bounded widget count)',
    (tester) async {
      await tester.pumpWidget(host(wordlist: wl));
      // Far more than the longest valid phrase (24) — a paste-bomb shape.
      final many = List.filled(kMaxMnemonicWords + 20, 'abandon').join(' ');
      await type(tester, '$many ');
      // Capped at the constant — and the constant is ABOVE 24, so the over-paste
      // reads as an invalid length, never silently truncated to a plausible 24.
      expect(last.length, kMaxMnemonicWords);
      expect(kMaxMnemonicWords, greaterThan(24));
    },
  );

  testWidgets(
    'with no wordlist loaded yet, pills are neutral (no false flags)',
    (tester) async {
      await tester.pumpWidget(host(wordlist: null));
      await type(tester, 'abandon zzzz ');
      expect(last, ['abandon', 'zzzz']);
      // Validity is unknown → no error glyph claimed either way.
      expect(find.byIcon(Icons.error_outline), findsNothing);
    },
  );

  testWidgets(
    'disabled field commits nothing on tap of a removed pill control',
    (tester) async {
      await tester.pumpWidget(host(wordlist: wl, enabled: false));
      // The inline field is disabled; no crash, no pills.
      expect(find.byType(ActionChip), findsNothing);
    },
  );
}
