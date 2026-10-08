import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/features/wallet/send/wallet_send_request.dart';
import 'package:zec_wallet_ui/testing.dart';

/// FR-25 — the prefilled-send seam's typed request + its ZIP-321 door. The pure
/// mapper (`walletSendRequestFromParsedLegs`) and the API-error mapper are
/// unit-tested at their boundary against fixture legs — no bridge, no device —
/// exactly the split the adapter uses (the audited native `parse_payment_uri`
/// does the ZIP-321 work; this Dart layer only projects + enforces the seam
/// policy). [WalletSendRequest.fromUri] is exercised over the fake session.
void main() {
  ParsedPayment leg({
    String address = 'u1recipient',
    AddressKind kind = AddressKind.unified,
    bool memoCapable = true,
    int? amountZat,
    ParsedMemo memo = const ParsedMemo.empty(),
    String? label,
    String? message,
  }) => ParsedPayment(
    recipientAddress: address,
    recipientKind: kind,
    recipientMemoCapable: memoCapable,
    amountZat: amountZat,
    memo: memo,
    label: label,
    message: message,
  );

  group('walletSendRequestFromParsedLegs', () {
    test('maps a single leg field-for-field (text memo)', () {
      final r = walletSendRequestFromParsedLegs([
        leg(
          address: 'u1alice',
          amountZat: 150000,
          memo: const ParsedMemo.text(text: 'coffee'),
          label: 'Alice',
        ),
      ]);
      expect(r.address, 'u1alice');
      expect(r.amountZat, 150000);
      expect(r.memo, 'coffee');
      expect(r.label, 'Alice');
      // A URI never pre-locks the recipient — the host opts in via fromUri.
      expect(r.lockRecipient, isFalse);
    });

    test('an amount-less (donation) leg carries a null amount', () {
      final r = walletSendRequestFromParsedLegs([leg(amountZat: null)]);
      expect(r.amountZat, isNull);
    });

    test('an empty memo maps to a null memo (no phantom text)', () {
      final r = walletSendRequestFromParsedLegs([
        leg(memo: const ParsedMemo.empty()),
      ]);
      expect(r.memo, isNull);
    });

    test('zero legs is rejected malformed (never an empty form)', () {
      expect(
        () => walletSendRequestFromParsedLegs([]),
        throwsA(
          isA<WalletSendRequestException>().having(
            (e) => e.fault,
            'fault',
            WalletSendRequestFault.malformed,
          ),
        ),
      );
    });

    test('more than one leg is rejected (single-recipient form)', () {
      expect(
        () => walletSendRequestFromParsedLegs([leg(), leg(address: 'u1bob')]),
        throwsA(
          isA<WalletSendRequestException>().having(
            (e) => e.fault,
            'fault',
            WalletSendRequestFault.multiplePayments,
          ),
        ),
      );
    });

    test('a machine (arbitrary) memo is rejected, never silently dropped', () {
      expect(
        () => walletSendRequestFromParsedLegs([
          leg(memo: const ParsedMemo.arbitrary(len: 64)),
        ]),
        throwsA(
          isA<WalletSendRequestException>().having(
            (e) => e.fault,
            'fault',
            WalletSendRequestFault.unsupportedMemo,
          ),
        ),
      );
    });

    test('a reserved memo is rejected as unsupported', () {
      expect(
        () => walletSendRequestFromParsedLegs([
          leg(memo: const ParsedMemo.reserved(len: 8)),
        ]),
        throwsA(
          isA<WalletSendRequestException>().having(
            (e) => e.fault,
            'fault',
            WalletSendRequestFault.unsupportedMemo,
          ),
        ),
      );
    });

    test('an unknown (future) memo arm is rejected, never crashes', () {
      expect(
        () => walletSendRequestFromParsedLegs([
          leg(memo: const ParsedMemo.unknown()),
        ]),
        throwsA(isA<WalletSendRequestException>()),
      );
    });
  });

  group('walletSendRequestFaultFromApiError', () {
    WalletApiError apiError(WalletErrorKind kind) =>
        WalletApiError(code: 'RW-TEST', message: 'static', kind: kind);

    test('NetworkMismatch maps to wrongNetwork (distinct, renderable)', () {
      expect(
        walletSendRequestFaultFromApiError(
          apiError(const WalletErrorKind.networkMismatch()),
        ).fault,
        WalletSendRequestFault.wrongNetwork,
      );
    });

    test('PaymentUriInvalid maps to malformed', () {
      expect(
        walletSendRequestFaultFromApiError(
          apiError(const WalletErrorKind.paymentUriInvalid()),
        ).fault,
        WalletSendRequestFault.malformed,
      );
    });

    test('any other kind collapses to malformed', () {
      expect(
        walletSendRequestFaultFromApiError(
          apiError(const WalletErrorKind.addressInvalid()),
        ).fault,
        WalletSendRequestFault.malformed,
      );
    });

    test('a non-FRB error is malformed (never leaks a raw type)', () {
      expect(
        walletSendRequestFaultFromApiError(Exception('boom')).fault,
        WalletSendRequestFault.malformed,
      );
    });
  });

  group('WalletSendRequest.fromUri (over the fake session)', () {
    test('delegates to the session parse; no lock by default', () {
      final session = FakeWalletSession()
        ..parseSendRequestResult = const WalletSendRequest(
          address: 'u1carol',
          amountZat: 42,
          memo: 'hi',
          label: 'Carol',
        );
      final r = WalletSendRequest.fromUri(session, 'zcash:u1carol?amount=…');
      expect(session.lastParsedUri, 'zcash:u1carol?amount=…');
      expect(r.address, 'u1carol');
      expect(r.amountZat, 42);
      expect(r.memo, 'hi');
      expect(r.label, 'Carol');
      expect(r.lockRecipient, isFalse);
    });

    test('lockRecipient:true rides through, preserving every other field', () {
      final session = FakeWalletSession()
        ..parseSendRequestResult = const WalletSendRequest(
          address: 'u1dan',
          amountZat: 99,
          memo: 'note',
          label: 'Dan',
        );
      final r = WalletSendRequest.fromUri(
        session,
        'zcash:u1dan',
        lockRecipient: true,
      );
      expect(r.lockRecipient, isTrue);
      expect(r.address, 'u1dan');
      expect(r.amountZat, 99);
      expect(r.memo, 'note');
      expect(r.label, 'Dan');
    });

    test('a seam rejection propagates as the typed exception', () {
      final session = FakeWalletSession()
        ..parseSendRequestThrows = const WalletSendRequestException(
          WalletSendRequestFault.wrongNetwork,
        );
      expect(
        () => WalletSendRequest.fromUri(session, 'zcash:t1testnet'),
        throwsA(
          isA<WalletSendRequestException>().having(
            (e) => e.fault,
            'fault',
            WalletSendRequestFault.wrongNetwork,
          ),
        ),
      );
    });
  });

  group('one memo per request (the both-set assert)', () {
    // debt: the assert shipped with NO test. It is a developer aid — it is
    // stripped in release, and the BINDING refusal is the audited encoder's
    // (`convert.rs::encode_payment_uri`, pinned in Rust by
    // `encode_payment_uri_refuses_both_memos`). Pinned here so the early copy
    // does not silently stop firing; `flutter test` runs with asserts ENABLED,
    // which is exactly the build it exists for.
    WalletMachineMemo memoBytes() => WalletMachineMemo(
      bytes: const [0x52, 0x4C, 0x4D, 0x01],
      purpose: 'Attach a payment reference',
    );

    test('text + machine bytes together trips the assert', () {
      expect(
        () => WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          memo: 'dinner',
          machineMemo: memoBytes(),
        ),
        throwsA(isA<AssertionError>()),
      );
    });

    test(
      'either memo ALONE is accepted — the refusal is about the CONFLICT',
      () {
        // Without this row a build that rejected every machine memo would pass
        // the row above and look correct.
        expect(
          WalletSendRequest(
            address: 'u1alice',
            amountZat: 150000,
            machineMemo: memoBytes(),
          ).machineMemo,
          isNotNull,
        );
        expect(
          const WalletSendRequest(
            address: 'u1alice',
            amountZat: 150000,
            memo: 'dinner',
          ).memo,
          'dinner',
        );
      },
    );

    test('an EMPTY text memo is not a conflict — it is no memo at all', () {
      // The assert reads `memo == null || memo.length == 0`, so the empty
      // string must NOT trip it: a form that cleared its memo field and then
      // attached bytes is the ordinary path, not a caller bug.
      expect(
        WalletSendRequest(
          address: 'u1alice',
          amountZat: 150000,
          memo: '',
          machineMemo: memoBytes(),
        ).machineMemo,
        isNotNull,
      );
    });
  });

  // Stage S8 `deadline` (R05), the identity a final "no transaction" attaches
  // to. The matrix is the contract's row 3 plus the two cells the diff
  // review found empty (ADR-0558): a tag on ONE side only. The widget rows in
  // wallet_send_deadline_test.dart drive the doors; this pins the predicate.
  group('sameSendRequest', () {
    const a = WalletSendRequest(address: 'u1alice', amountZat: 150000);
    const aTagged = WalletSendRequest(
      address: 'u1alice',
      amountZat: 150000,
      correlationId: 'inv-1',
    );
    const otherAmount = WalletSendRequest(
      address: 'u1alice',
      amountZat: 250000,
    );
    const otherAmountTagged = WalletSendRequest(
      address: 'u1alice',
      amountZat: 250000,
      correlationId: 'inv-1',
    );
    const otherTag = WalletSendRequest(
      address: 'u1alice',
      amountZat: 150000,
      correlationId: 'inv-2',
    );

    test('the same object is the same request', () {
      expect(sameSendRequest(a, a), isTrue);
      expect(sameSendRequest(aTagged, aTagged), isTrue);
    });

    test('BOTH tagged: the tags decide, whatever the fields say', () {
      expect(
        sameSendRequest(aTagged, otherAmountTagged),
        isTrue,
        reason: 'the same host record, corrected amount, is the same request',
      );
      expect(
        sameSendRequest(aTagged, otherTag),
        isFalse,
        reason: 'a different host record is a new act (row 789)',
      );
    });

    test('untagged on both sides: the payment fields decide', () {
      expect(
        sameSendRequest(
          a,
          const WalletSendRequest(address: 'u1alice', amountZat: 150000),
        ),
        isTrue,
        reason: 'const-equal fields are one request (and one object in Dart)',
      );
      expect(sameSendRequest(a, otherAmount), isFalse);
    });

    test(
      'a tag on ONE side only does not make a new request — the fields decide',
      () {
        // The cell: an untagged request the grace revoked, re-navigated
        // with the same fields plus a retry tag, paid on the fold.
        expect(sameSendRequest(a, aTagged), isTrue);
        expect(sameSendRequest(aTagged, a), isTrue, reason: 'symmetric');
        expect(
          sameSendRequest(a, otherAmountTagged),
          isFalse,
          reason: 'different fields under a one-sided tag differ',
        );
      },
    );

    test('a machine memo is compared by purpose and bytes, not identity', () {
      WalletSendRequest withMemo(List<int> bytes, String purpose) =>
          WalletSendRequest(
            address: 'u1alice',
            amountZat: 150000,
            machineMemo: WalletMachineMemo(bytes: bytes, purpose: purpose),
          );
      expect(
        sameSendRequest(
          withMemo([1, 2, 3], 'invoice'),
          withMemo([1, 2, 3], 'invoice'),
        ),
        isTrue,
      );
      expect(
        sameSendRequest(
          withMemo([1, 2, 3], 'invoice'),
          withMemo([1, 2, 4], 'invoice'),
        ),
        isFalse,
      );
      expect(sameSendRequest(withMemo([1, 2, 3], 'invoice'), a), isFalse);
    });
  });
}
