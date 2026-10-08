import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/send/zec_amount.dart';
import 'package:zec_wallet_ui/shared/decimal_input_formatter.dart';

/// S13 §1.1 + revision 2 M1: the one amount formatter never DELETES — an edit
/// is taken whole or refused whole — so a comma cannot quietly turn "1,000"
/// into 1 ZEC.
void main() {
  const f = WalletDecimalInputFormatter();

  TextEditingValue v(String text) => TextEditingValue(
    text: text,
    selection: TextSelection.collapsed(offset: text.length),
  );

  /// Type [text] one character at a time from an empty field.
  String typed(String text) {
    var value = v('');
    for (final ch in text.split('')) {
      value = f.formatEditUpdate(value, v(value.text + ch));
    }
    return value.text;
  }

  /// Paste [text] into a field holding [before].
  String pasted(String text, {String before = ''}) =>
      f.formatEditUpdate(v(before), v(before + text)).text;

  int zatOf(String text) => (parseZecAmount(text) as ZecAmountValid).zat;

  test('typing "0,5" is 0.5 ZEC (50,000,000 zat)', () {
    expect(typed('0,5'), '0.5');
    expect(zatOf(typed('0,5')), 50000000);
  });

  test('",5" is 0.5 and "5," is 5', () {
    expect(zatOf(typed(',5')), 50000000);
    expect(zatOf(typed('5,')), 500000000);
  });

  test('a second separator is refused, the field keeps its value', () {
    expect(typed('0.5.'), '0.5');
    expect(typed('0,5,'), '0.5');
    expect(typed('0.5,1'), '0.51');
  });

  test('pasting "1,000.50", "1.000,50" and "1,000" changes nothing', () {
    expect(pasted('1,000.50'), '');
    expect(pasted('1.000,50'), '');
    expect(pasted('1,000'), '');
    expect(pasted(',000', before: '1'), '1');
  });

  test('a paste that is not ambiguous is taken', () {
    expect(pasted('0,000'), '0.000');
    expect(pasted('1,5'), '1.5');
    expect(pasted('12,3456'), '12.3456');
  });

  test('a character outside [0-9.,] rejects the whole edit, never strips', () {
    expect(pasted('1 000'), '');
    expect(pasted('-5'), '');
    expect(pasted('5e3'), '');
    expect(typed('12a'), '12');
  });

  test('"1,000" is refused however it arrives — pasted over a selection, or '
      'a comma typed into the middle of "1000"', () {
    // Paste over a selected, LONGER value: the net length change is negative.
    final overSelection = f.formatEditUpdate(
      const TextEditingValue(
        text: '0.0001',
        selection: TextSelection(baseOffset: 0, extentOffset: 6),
      ),
      v('1,000'),
    );
    expect(overSelection.text, '0.0001');
    // A thousands mark typed into "1000" at offset 1.
    final midText = f.formatEditUpdate(
      const TextEditingValue(
        text: '1000',
        selection: TextSelection.collapsed(offset: 1),
      ),
      const TextEditingValue(
        text: '1,000',
        selection: TextSelection.collapsed(offset: 2),
      ),
    );
    expect(midText.text, '1000');
  });

  test('past maxLength the edit is refused whole, never truncated', () {
    const bounded = WalletDecimalInputFormatter(maxLength: 20);
    final long = '0' * 17 + '1000'; // 21 characters, 1000 ZEC
    expect(bounded.formatEditUpdate(v(''), v(long)).text, '');
    expect(
      bounded.formatEditUpdate(v(''), v('0' * 16 + '1000')).text,
      '0' * 16 + '1000',
    );
  });

  test('the selection survives the comma swap', () {
    final out = f.formatEditUpdate(v('0'), v('0,'));
    expect(out.text, '0.');
    expect(out.selection, const TextSelection.collapsed(offset: 2));
  });
}
