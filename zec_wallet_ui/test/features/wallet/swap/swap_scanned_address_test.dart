import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/swap/swap_scanned_address.dart';

void main() {
  group('normalizeScannedAddress', () {
    // (input, expected, why) — real-world QR payloads across chains. The
    // function is purely syntactic: strip a leading `scheme:` and any
    // `?query`/`#fragment`; it has NO chain knowledge and does NO validation.
    const cases = <(String, String, String)>[
      // --- bare addresses pass through untouched ---
      ('bc1qexampleaddr0000', 'bc1qexampleaddr0000', 'bare bech32 unchanged'),
      ('0xabc123DEF456', '0xabc123DEF456', 'bare 0x unchanged'),
      ('Tx9YbExampleTronAddr', 'Tx9YbExampleTronAddr', 'bare base58 unchanged'),
      // --- BIP-21 (bitcoin) ---
      ('bitcoin:bc1qexampleaddr0000', 'bc1qexampleaddr0000', 'BIP-21 scheme'),
      (
        'bitcoin:bc1qexampleaddr0000?amount=0.5&label=wallet',
        'bc1qexampleaddr0000',
        'BIP-21 scheme + query params',
      ),
      (
        'BITCOIN:bc1qexampleaddr0000',
        'bc1qexampleaddr0000',
        'uppercase scheme (RFC 3986 schemes are case-insensitive)',
      ),
      (
        'bitcoin://bc1qexampleaddr0000',
        'bc1qexampleaddr0000',
        'scheme with an authority-style // prefix',
      ),
      (
        'bitcoin:bc1qexampleaddr0000#note',
        'bc1qexampleaddr0000',
        'scheme + fragment',
      ),
      // --- EIP-681 (ethereum) ---
      (
        'ethereum:0xabc123DEF456?value=1e18',
        '0xabc123DEF456',
        'EIP-681 scheme + value query',
      ),
      (
        'ethereum:0xabc123DEF456@1',
        '0xabc123DEF456@1',
        'EIP-681 chain-id suffix is LEFT INTACT (documented residual — @ strip '
            'is chain-specific; the user catches it at verification)',
      ),
      // --- other chains, same shape ---
      (
        'litecoin:ltc1qexampleaddr?amount=2',
        'ltc1qexampleaddr',
        'litecoin BIP-21-style',
      ),
      // --- defensive: query without a scheme (some encoders drop it) ---
      (
        'bc1qexampleaddr0000?amount=1',
        'bc1qexampleaddr0000',
        'query stripped even with no scheme (a bech32/0x address has no ?)',
      ),
      // --- whitespace hygiene ---
      (
        '  bc1qexampleaddr0000  ',
        'bc1qexampleaddr0000',
        'leading/trailing whitespace trimmed',
      ),
      (
        'bitcoin:bc1qexampleaddr0000 ',
        'bc1qexampleaddr0000',
        'trailing space after a scheme payload trimmed',
      ),
    ];

    for (final (input, expected, why) in cases) {
      test('"$input" -> "$expected"  ($why)', () {
        expect(normalizeScannedAddress(input), expected);
      });
    }

    // --- degenerate inputs: never throw, yield empty so the form's non-empty
    // guard (and the SDK) reject them honestly ---
    test('scheme-only yields empty', () {
      expect(normalizeScannedAddress('bitcoin:'), '');
    });
    test('empty yields empty', () {
      expect(normalizeScannedAddress(''), '');
    });
    test('whitespace-only yields empty', () {
      expect(normalizeScannedAddress('   '), '');
    });

    test('a leading-digit token is NOT treated as a scheme (kept intact)', () {
      // RFC-3986 schemes must start with a letter, so this is not unwrapped.
      expect(normalizeScannedAddress('1abc:rest'), '1abc:rest');
    });

    test('is idempotent on an already-bare address', () {
      const addr = 'bc1qexampleaddr0000';
      expect(normalizeScannedAddress(normalizeScannedAddress(addr)), addr);
    });

    // --- untrusted-input hardening (security review fold) ---
    group('hardening', () {
      test('an absurd payload (> kMaxScannedPayloadChars) yields empty', () {
        final huge = 'bitcoin:${'a' * (kMaxScannedPayloadChars + 100)}';
        expect(normalizeScannedAddress(huge), '');
      });

      test('an over-long address (> kMaxScannedAddressChars) yields empty', () {
        // Under the payload cap but past the address cap.
        final long = 'a' * (kMaxScannedAddressChars + 1);
        expect(normalizeScannedAddress(long), '');
        // …and right at the cap is kept.
        final atCap = 'a' * kMaxScannedAddressChars;
        expect(normalizeScannedAddress(atCap), atCap);
      });

      test('a CRLF cannot smuggle a second "line" into the field', () {
        // The §3a wire-format-injection shape: truncated at the first control
        // byte → one bare token, no control bytes, no second line.
        final crlf = '${String.fromCharCode(13)}${String.fromCharCode(10)}';
        final injected = 'bitcoin:bc1qADDR${crlf}EVIL?amount=1';
        final out = normalizeScannedAddress(injected);
        expect(out, 'bc1qADDR');
        expect(out.codeUnits.any((c) => c < 0x20), isFalse);
      });

      test('an interior TAB / space terminates the address token', () {
        final tab = String.fromCharCode(9);
        expect(normalizeScannedAddress('bc1qADDR${tab}EVIL'), 'bc1qADDR');
        expect(normalizeScannedAddress('bc1qADDR EVIL'), 'bc1qADDR');
      });

      test('a NUL (C0) or NEL (C1) control byte terminates the token', () {
        final nul = String.fromCharCode(0);
        final nel = String.fromCharCode(0x85);
        expect(normalizeScannedAddress('bc1qADDR${nul}EVIL'), 'bc1qADDR');
        expect(normalizeScannedAddress('bc1qADDR${nel}EVIL'), 'bc1qADDR');
      });
    });

    // --- real-world QR shapes the table above doesn't cover (S84 edge round).
    // Each pins the ACTUAL behavior of the purely-syntactic unwrap. The SDK is
    // still the sole validation gate; these only assert the envelope strip.
    group('real-world edge', () {
      test('a multi-param BIP-21 with a URL-encoded label keeps the % escapes '
          'in the dropped query (only the bare address survives)', () {
        // A real wallet "request" QR: amount + an %20-encoded label + message.
        // Everything from the first `?` is dropped; the %-escapes never reach
        // the field (they live in the query, which we discard wholesale).
        expect(
          normalizeScannedAddress(
            'bitcoin:bc1qaddr?amount=0.1&label=Some%20Shop&message=Order%20%231',
          ),
          'bc1qaddr',
        );
      });

      test('an EIP-681 @chainId AND a query together: query dropped, the @id '
          'residual is LEFT INTACT (documented — @ strip is chain-specific)', () {
        // `?value=…&gas=…` is stripped at the first `?`; the `@1` chain-id stays
        // (same documented residual as the `…@1` table row, now with a query).
        expect(
          normalizeScannedAddress('ethereum:0xabc123@1?value=1e18&gas=21000'),
          '0xabc123@1',
        );
      });

      test('a payjoin pj= param is dropped with the rest of the query', () {
        expect(
          normalizeScannedAddress(
            'bitcoin:bc1qaddr?amount=0.1&pj=https://example.com/pj',
          ),
          'bc1qaddr',
        );
      });

      test('a zcash: URI just unwraps to the bare z-addr (the Rust validator '
          'rejects a ZEC addr as a foreign refund — not this layer\'s job)', () {
        // The user could scan a ZEC address by mistake. We do NOT know chains,
        // so we unwrap the envelope and hand the opaque z-addr onward; the SDK
        // is the gate that rejects it as a source-chain refund.
        expect(
          normalizeScannedAddress('zcash:zs1examplezaddr?amount=1.0&memo=aGk'),
          'zs1examplezaddr',
        );
      });

      test(
        'an UPPERCASE bech32 address is NOT lowercased (case is preserved)',
        () {
          // Addresses are case-sensitive on some chains (e.g. base58check,
          // EIP-55 checksums) — the unwrap must never fold case.
          expect(normalizeScannedAddress('BC1QEXAMPLEADDR'), 'BC1QEXAMPLEADDR');
        },
      );

      test('a mixed-case EIP-55 checksum address keeps its exact casing', () {
        // Folding case here would silently invalidate the EIP-55 checksum.
        expect(
          normalizeScannedAddress('ethereum:0xAbCdEf0123'),
          '0xAbCdEf0123',
        );
      });

      test(
        'a mixed-case address behind a scheme + query is unwrapped WITHOUT '
        'case folding (scheme match is case-insensitive, payload is not)',
        () {
          expect(
            normalizeScannedAddress('bitcoin:BC1QExampleADDR0000?amount=1'),
            'BC1QExampleADDR0000',
          );
        },
      );

      test('a scheme-only payload WITH a query yields empty', () {
        // `bitcoin:?amount=1` — no address between the scheme and the `?`.
        expect(normalizeScannedAddress('bitcoin:?amount=1'), '');
      });

      test('an address exactly at the payload cap behind a scheme is kept', () {
        // The address itself is at the address cap; with the scheme the raw
        // payload stays under the PAYLOAD cap, so it survives.
        final addr = 'a' * kMaxScannedAddressChars;
        expect(normalizeScannedAddress('bitcoin:$addr'), addr);
      });

      test(
        'an address one past the address cap behind a scheme yields empty',
        () {
          final over = 'a' * (kMaxScannedAddressChars + 1);
          expect(normalizeScannedAddress('bitcoin:$over'), '');
        },
      );

      test(
        'a leading/trailing NBSP (U+00A0) is trimmed off the bare address',
        () {
          // Dart String.trim() removes Unicode whitespace incl. NBSP; a wrapped
          // address comes back bare.
          final nbsp = String.fromCharCode(0xA0);
          expect(normalizeScannedAddress('${nbsp}bc1qaddr$nbsp'), 'bc1qaddr');
        },
      );

      test('an interior NBSP (U+00A0) terminates the address token', () {
        // NBSP is Unicode whitespace, so a smuggled second token after it is
        // cut off.
        final nbsp = String.fromCharCode(0xA0);
        expect(normalizeScannedAddress('bc1qADDR${nbsp}EVIL'), 'bc1qADDR');
      });

      test('an interior wide Unicode space (U+2000 / U+3000) terminates the '
          'token (the single-token invariant covers the full Zs set)', () {
        // Operational-round fold: the truncation delegates to String.trim()'s
        // whitespace set, so ideographic/figure spaces cut the token too — not
        // just ASCII space + NBSP.
        final enQuad = String.fromCharCode(0x2000);
        final ideographic = String.fromCharCode(0x3000);
        expect(normalizeScannedAddress('bc1qADDR${enQuad}EVIL'), 'bc1qADDR');
        expect(
          normalizeScannedAddress('bc1qADDR${ideographic}EVIL'),
          'bc1qADDR',
        );
      });

      test('a Unicode line/paragraph separator (U+2028/U+2029) terminates the '
          'token (no second line smuggled in)', () {
        final lineSep = String.fromCharCode(0x2028);
        final paraSep = String.fromCharCode(0x2029);
        expect(normalizeScannedAddress('bc1qADDR${lineSep}EVIL'), 'bc1qADDR');
        expect(normalizeScannedAddress('bc1qADDR${paraSep}EVIL'), 'bc1qADDR');
      });

      // The headline decision (task call-out): a SHORT valid address hidden
      // behind a HUGE label. The payload cap is checked on the RAW string FIRST,
      // before the query is stripped — so the total length, not the address
      // length, decides. This documents the ACTUAL behavior so the trade-off is
      // an explicit, reviewed decision (a maliciously huge QR is rejected whole;
      // the user re-scans or pastes — honest, no silent truncation).
      test('a short address behind a >payload-cap label is REJECTED whole '
          '(payload cap is on the raw QR, not the post-strip address)', () {
        final huge = 'bitcoin:bc1qSHORT?label=${'x' * kMaxScannedPayloadChars}';
        expect(huge.length > kMaxScannedPayloadChars, isTrue);
        // NOT 'bc1qSHORT' — the whole oversized payload is refused up front.
        expect(normalizeScannedAddress(huge), '');
      });

      test('a short address behind a label sized to EXACTLY the payload cap is '
          'still extracted (the cap is inclusive at the boundary)', () {
        const addr = 'bc1qSHORT';
        const prefix = 'bitcoin:$addr?label=';
        final pad = kMaxScannedPayloadChars - prefix.length;
        final atCap = '$prefix${'x' * pad}';
        expect(atCap.length, kMaxScannedPayloadChars);
        expect(normalizeScannedAddress(atCap), addr);
      });

      test('one char past the payload cap with the same short address yields '
          'empty (the boundary is strict)', () {
        const addr = 'bc1qSHORT';
        const prefix = 'bitcoin:$addr?label=';
        final pad = kMaxScannedPayloadChars - prefix.length + 1;
        final overCap = '$prefix${'x' * pad}';
        expect(overCap.length, kMaxScannedPayloadChars + 1);
        expect(normalizeScannedAddress(overCap), '');
      });

      test('never throws on raw control/garbage bytes — yields a string', () {
        // A planted QR can carry arbitrary bytes; the unwrap must be total.
        final raws = <String>[
          '',
          '::::',
          '?#?#',
          'bitcoin: ',
          'a:b:c:d',
          String.fromCharCode(0x2028) + String.fromCharCode(0x2029),
          String.fromCharCode(0) + String.fromCharCode(0x7F),
        ];
        for (final raw in raws) {
          expect(() => normalizeScannedAddress(raw), returnsNormally);
          expect(normalizeScannedAddress(raw), isA<String>());
        }
      });
    });
  });
}
