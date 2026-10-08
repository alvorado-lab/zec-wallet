import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/core/theme/theme.dart';
import 'package:zec_wallet_ui/shared/address_text.dart';

Widget _host(String address, {int edge = 6}) => MaterialApp(
  theme: lightTheme, // AddressText reads WalletColors
  home: Scaffold(
    body: Center(child: AddressText(address, edge: edge)),
  ),
);

/// The concatenated plain text the widget renders (across its spans) — proves the
/// FULL address is present (nothing is dropped; only the middle is visually muted).
String _renderedText(WidgetTester tester) {
  final selectable = tester.widget<SelectableText>(find.byType(SelectableText));
  final span = selectable.textSpan;
  return span?.toPlainText() ?? selectable.data ?? '';
}

void main() {
  testWidgets('a long address renders in full with bold head + tail spans', (
    tester,
  ) async {
    // zcash_address's own test vector (encoding.rs), not a wallet of ours.
    const addr = 't1Hsc1LR8yKnbbe3twRp88p6vFfC5t7DLbs';
    await tester.pumpWidget(_host(addr));

    // Money-correctness: the ENTIRE address is present (no truncation).
    expect(_renderedText(tester), addr);

    // Three spans: head (bold) + middle (muted) + tail (bold).
    final selectable = tester.widget<SelectableText>(
      find.byType(SelectableText),
    );
    final children = selectable.textSpan!.children!;
    expect(children.length, 3);
    final head = children[0] as TextSpan;
    final mid = children[1] as TextSpan;
    final tail = children[2] as TextSpan;
    expect(head.text, addr.substring(0, 6));
    expect(tail.text, addr.substring(addr.length - 6));
    expect(head.style!.fontWeight, FontWeight.w700);
    expect(tail.style!.fontWeight, FontWeight.w700);
    expect(mid.style!.fontWeight, FontWeight.w400); // the middle recedes
  });

  testWidgets('a short address is emphasized whole (no split)', (tester) async {
    const addr = 'u1copyme'; // <= edge*2
    await tester.pumpWidget(_host(addr));
    final selectable = tester.widget<SelectableText>(
      find.byType(SelectableText),
    );
    // Rendered as a single styled string, still the full value.
    expect(selectable.data, addr);
    expect(selectable.style!.fontWeight, FontWeight.w700);
  });

  // --- groupAddress: the shared verification-chunking SSOT --------------------

  group('groupAddress — loss-free, exact grouping', () {
    test('empty in → empty out', () => expect(groupAddress(''), ''));
    test(
      'shorter than a group → the whole string (no padding)',
      () => expect(groupAddress('u1q'), 'u1q'),
    );
    test(
      'exact multiple → even space-separated groups, no trailing space',
      () => expect(groupAddress('abcdefgh'), 'abcd efgh'),
    );
    test(
      'remainder → the last group is the short remainder',
      () => expect(groupAddress('abcdefghij'), 'abcd efgh ij'),
    );
    test(
      'a length-one remainder is preserved (boundary off-by-one)',
      () => expect(groupAddress('abcde'), 'abcd e'),
    );
    test('a custom group size groups accordingly', () {
      expect(groupAddress('abcdef', groupSize: 2), 'ab cd ef');
      expect(groupAddress('abcdefg', groupSize: 3), 'abc def g');
    });
    test('LOSS-FREE — stripping the inserted spaces yields the original', () {
      // The money-safety invariant: a user verifies the grouped form, so the
      // groups MUST reconstruct the exact address (no dropped/duplicated/reordered
      // glyph). A realistic ~78-char unified-address-length string (no spaces).
      const addr =
          'u1qsy8h0c2f3k4m5n6p7r8s9t0v1w2x3y4z5a6b7c8d9e0f1g2h3j4k5l6m7n8p9q0r1s2t3u4v5';
      expect(groupAddress(addr).replaceAll(' ', ''), addr);
    });
  });

  // --- AddressVerificationText: the equal-weight verification render ----------

  Widget hostVerify(String address) => MaterialApp(
    theme: lightTheme,
    home: Scaffold(body: Center(child: AddressVerificationText(address))),
  );

  testWidgets('AddressVerificationText renders the FULL address in chunks, '
      'NOT selectable', (tester) async {
    const addr = 'bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq';
    await tester.pumpWidget(hostVerify(addr));

    // The grouped form is present in full; copying is disabled (verification, not
    // copy — a SelectableText would capture the render-only spaces).
    expect(find.text(groupAddress(addr)), findsOneWidget);
    expect(find.byType(SelectableText), findsNothing);
    // The FULL ungrouped address is the screen-reader label (announced whole).
    expect(
      find.byWidgetPredicate(
        (w) => w is Semantics && w.properties.label == addr,
      ),
      findsOneWidget,
    );
  });

  testWidgets(
    'AddressVerificationText renders an em dash for an empty address',
    (tester) async {
      await tester.pumpWidget(hostVerify(''));
      expect(find.text('—'), findsOneWidget);
    },
  );
}
