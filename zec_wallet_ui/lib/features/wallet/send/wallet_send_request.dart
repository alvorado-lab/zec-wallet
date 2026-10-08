import 'package:flutter/foundation.dart' show listEquals;
import 'package:zec_wallet/zec_wallet.dart';

import '../wallet_session.dart';

/// FR-25 — the prefilled-send entry seam's typed request (ZIP-321-first).
///
/// A host with its OWN address-discovery channel (a messenger contact, a
/// scanned QR, a `zcash:` deep link, an NFC tap) hands this to
/// [WalletSendEntry.push] to open the package's send flow with fields
/// populated, so the ENTIRE money path — validation, review, ceiling,
/// authorizer — stays inside the audited package and the host contributes only
/// the entry point (no host rebuilds a send form).
///
/// CONTRACT — **prefilled ≠ pre-confirmed.** A prefill only SEEDS the editable
/// form; it runs the same `prepare → propose → review → authorize` path and the
/// same gating (recipient validity, sync state, spendable balance, host
/// ceiling) as an organic send. Fields stay user-editable unless
/// [lockRecipient] is set (recipient-only — the amount and memo NEVER lock).
///
/// Pure, immutable value data — no network, no keys, no money movement. The
/// ZIP-321 door is [WalletSendRequest.fromUri]; construct directly for a typed
/// prefill.
class WalletSendRequest {
  const WalletSendRequest({
    required this.address,
    this.amountZat,
    this.memo,
    this.label,
    this.lockRecipient = false,
    this.correlationId,
    this.machineMemo,
  }) : // ONE memo per payment. A request carrying both a text memo and machine
       // bytes has no honest resolution — something would have to pick, and
       // whichever it picked would be a memo the caller did not choose, written
       // once to a permanent ledger.
       //
       // THE BINDING REFUSAL IS THE ENCODER'S, and this is only its early copy.
       // `convert.rs::encode_payment_uri` rejects both-set typed before any URI
       // exists; that runs in every build and is what actually stops a memo the
       // caller did not choose from reaching a permanent ledger.
       //
       // This assert buys ONE thing: it fires at the construction site, in the
       // host's own debug build, instead of at compose time. It is stripped in
       // release, so it is a developer aid and nothing more.
       //
       // The comment here used to justify the assert-over-throw choice by "this
       // stays a CONST value type". That was vacuous: `WalletMachineMemo`'s
       // constructor is non-const and throwing, so a request carrying one can
       // never be const anyway — precisely the branch the assert exists for
       // (post-build review; corrected).
       assert(
         machineMemo == null || memo == null || memo.length == 0,
         'a payment carries ONE memo: set either `memo` (text the user can '
         'read and edit) or `machineMemo` (opaque host bytes), never both',
       );

  /// Recipient address, as the host discovered it. NOT pre-validated here — the
  /// send form classifies it against the wallet's own network on entry (a
  /// wrong-network / malformed typed address surfaces the same honest inline
  /// status an organic paste would). A URI-sourced request already carries the
  /// core-validated canonical encoding (see [fromUri]).
  final String address;

  /// Requested amount in **zatoshis** (exact; max supply < 2^53). `null` = the
  /// requester left the amount to the sender (the donation-QR form) — the form
  /// opens with an empty, user-filled amount. A non-positive value (`0` /
  /// negative) is treated as `null` — nothing is seeded.
  final int? amountZat;

  /// Optional human-readable memo to seed the (editable) memo field. Only a
  /// TEXT memo is representable in the editable field; a machine (ZIP-302 0xFF)
  /// or reserved memo carried by a URI is rejected typed at [fromUri] rather
  /// than silently dropped (a payment-reference the recipient may need must not
  /// vanish). A memo to a transparent recipient is still dropped at compose by
  /// the existing send-form gate — a prefill does not bypass that.
  final String? memo;

  /// The ZIP-321 `label` display field (a name the requester attached to the
  /// address), when the request came from a URI. Carried for the host's use —
  /// deliberately NOT auto-rendered by the send form: it is an
  /// attacker-controllable string from an untrusted QR/link (a phishing
  /// surface), so the host, which alone has trusted context (e.g. the contact
  /// it resolved), decides whether/how to show it. `null` for a typed request.
  final String? label;

  /// When `true`, the recipient field opens READ-ONLY (still selectable, so the
  /// user can verify who they're paying) — for a host that resolved the payee
  /// itself (pay-a-contact) and must not let the address be edited into a
  /// different one. Recipient-ONLY: the amount and memo always stay editable
  /// (the FR-25 contract). Default `false` — a fully editable prefill.
  ///
  /// THE LOCK IS CONDITIONAL, not absolute (host contract):
  /// - It is applied only when the recipient classifies SENDABLE against this
  ///   wallet's network at entry. An invalid / wrong-network / unclassifiable
  ///   [address] opens EDITABLE with the inline status explaining why — a
  ///   read-only field the user can never correct would be a dead-end. (A
  ///   `fromUri` request can't hit this — the parser already network-checked
  ///   it — only a typed request with a bad address can.)
  /// - It is RELEASED if the wallet session flips while the form is open (the
  ///   identity switch clears the whole draft; a still-locked empty field
  ///   would be a dead-end).
  /// So a host MUST NOT treat the lock as a security guarantee that the typed
  /// address will be paid: the binding verification is the review screen's
  /// address echo and the host's own authorizer prompt (which receives the
  /// abbreviated recipient in its spend intent).
  final bool lockRecipient;

  /// FR-26 — an OPAQUE host token, echoed verbatim onto the
  /// [WalletSendReport] this request's flow produces. `null` = the host set
  /// none, and the report carries `null`.
  ///
  /// The package never interprets, parses, renders, logs, persists or
  /// transmits it. It exists so a host that routes reports into a shared sink
  /// (rather than awaiting each [WalletSendEntry.push] at its call site) can
  /// join the report to the record it opened. That join has to travel WITH the
  /// request: matching on the abbreviated recipient + amount the authorizer
  /// bracket receives is a display-fact join on a money surface and can attach
  /// a payment to the wrong record, and assuming the in-flight authorization
  /// bracket belongs to the most recently opened flow is free at one in-flight
  /// send and is not a proof at N.
  ///
  /// It is HOST data on a money surface, so keep it a meaningless handle — a
  /// row id, a nonce. Anything descriptive put here (a contact name, a note)
  /// is the host's own leak, not the package's.
  final String? correlationId;

  /// FR-28 — opaque machine-memo bytes plus the purpose the user is shown.
  ///
  /// Mutually exclusive with [memo]: one payment, one memo. Unlike the text
  /// memo — which only SEEDS an editable field — these bytes are not editable
  /// and not re-derivable from the form, so the screen carries them through
  /// every re-compose (a user editing the amount or the recipient does not
  /// silently drop the host's reference; that silent drop is the defect this
  /// FR exists to fix).
  ///
  /// A memo needs a shielded recipient. If the user edits the recipient to a
  /// transparent address the send is refused typed, exactly as a text memo is —
  /// the bytes are never silently dropped to let the payment through.
  final WalletMachineMemo? machineMemo;

  /// Parse a ZIP-321 `zcash:` payment URI (a scanned QR, a deep link — HOSTILE
  /// input) into a prefill request, validated against THIS wallet's own network
  /// (the [session] is the sole network authority — a host can NOT coerce a
  /// wrong-network send; the same network-hiding seam as
  /// [WalletSession.composePaymentUri]). The audited core `payment_uri` parser
  /// is the single validator (DRY) — this never re-parses ZIP-321 in Dart.
  ///
  /// Throws [WalletSendRequestException] (typed, payload-free — §5.4) at the
  /// SEAM, never a half-filled form:
  /// - [WalletSendRequestFault.malformed] — not a valid `zcash:` request URI.
  /// - [WalletSendRequestFault.wrongNetwork] — valid, but for the other network.
  /// - [WalletSendRequestFault.multiplePayments] — a multi-leg request; the
  ///   send form is single-recipient and cannot hold it.
  /// - [WalletSendRequestFault.unsupportedMemo] — a machine/reserved memo the
  ///   editable field cannot represent.
  ///
  /// The [lockRecipient] argument is AUTHORITATIVE in both directions (a
  /// scanned pay-a-contact URI often wants the recipient locked; `false`
  /// overrides even a lock a custom port implementation set on its returned
  /// request). SYNCHRONOUS (the parse is local + sync).
  factory WalletSendRequest.fromUri(
    WalletSession session,
    String uri, {
    bool lockRecipient = false,
    String? correlationId,
  }) {
    final request = session.parseSendRequest(uri);
    return WalletSendRequest(
      address: request.address,
      amountZat: request.amountZat,
      memo: request.memo,
      label: request.label,
      lockRecipient: lockRecipient,
      // The host's own token, never anything read out of the (untrusted) URI.
      correlationId: correlationId,
    );
  }
}

/// FR-28 — OPAQUE machine-memo bytes a host attaches to a payment, together
/// with the human-readable purpose it must state for them.
///
/// **The purpose is a REQUIRED constructor argument, not a display rule the
/// host may skip.** That is the point of this being a type at all: bytes cannot
/// exist here without a sentence explaining them, so "we forgot the
/// disclosure" is not a reachable state. A display rule is documentation; a
/// required parameter is a mechanism, and only the second one shows when it is
/// skipped.
///
/// **WHAT THE PURPOSE STRING BUYS, STATED SO IT IS NOT OVERSOLD.** It is
/// ACCOUNTABILITY, not verification. The SDK cannot check that the sentence
/// describes the bytes, and a hostile host can lie in it. What the user gets is
/// that something was attached and who said why — never that it is benign.
///
/// **The SDK never interprets the bytes.** Their layout is the host's: a length
/// frame, a version, a purpose code, whatever the host's own format says. The
/// wallet treats them as an opaque, length-bounded blob and never renders them
/// — hex is meaningless to a user and alarming in the wrong direction.
class WalletMachineMemo {
  /// Throws [ArgumentError] on empty [bytes] or a blank [purpose]: both are
  /// host bugs, and the honest answer is a refusal before anything reaches a
  /// permanent ledger — not a silently dropped memo or an undisclosed one.
  WalletMachineMemo({required List<int> bytes, required String purpose})
    : bytes = List<int>.unmodifiable(bytes),
      purpose = _sanitizePurpose(purpose) {
    if (this.bytes.isEmpty) {
      throw ArgumentError.value(
        bytes,
        'bytes',
        'a machine memo with no bytes is nothing to attach',
      );
    }
    if (this.purpose.isEmpty) {
      throw ArgumentError.value(
        purpose,
        'purpose',
        'bytes cannot be attached without a human-readable purpose — the user '
            'is told what is being added to their transaction',
      );
    }
    if (this.purpose.length > purposeMaxChars) {
      throw ArgumentError.value(
        purpose,
        'purpose',
        'a purpose line is one sentence the user reads at the authorization '
            'step, not a document (max $purposeMaxChars characters)',
      );
    }
  }

  /// The bound on [purpose]. It is rendered on a money surface at any text
  /// scale, so it is a sentence, not a payload.
  static const purposeMaxChars = 140;

  /// Flatten a host-supplied purpose to ONE plain line.
  ///
  /// The string is attacker-controllable text rendered on a money surface, and
  /// a character bound is not a content bound. The security review
  /// reproduced the attack this closes: a purpose containing newlines renders
  /// its later lines as their own paragraphs in the disclosure card, in the
  /// default text colour — MORE prominent than the wallet's own muted caveat
  /// beneath it — so `'Invoice 42\n\nVerified by the wallet. No further review
  /// needed.'` puts a forged wallet sentence on the authorization step. Bidi
  /// overrides can reorder what the user reads, and a wall of newlines pushes
  /// the Confirm button down the page.
  ///
  /// So: strip the control and directional-formatting characters entirely,
  /// collapse every run of whitespace to a single space, and trim. What the
  /// host gets is one sentence in one direction — which is all the disclosure
  /// ever promised to render. Done HERE, in the constructor, because it is the
  /// one place on this path that cannot be skipped.
  static String _sanitizePurpose(String raw) {
    // Two passes, because the two classes deserve different treatment.
    // Directional formatting is ZERO-WIDTH — it is removed outright, and
    // removing it joins nothing. Control and separator characters occupy the
    // position of a break, so they become a SPACE: deleting them would run
    // "Invoice 42" and the next word together into "Invoice 42Verified".
    final unformatted = raw.replaceAll(
      // LRM/RLM, the bidi embedding+override set, the isolate set.
      RegExp(r'[\u200E\u200F\u202A-\u202E\u2066-\u2069]'),
      '',
    );
    final flattened = unformatted.replaceAll(
      // C0 + DEL + C1, and the line/paragraph separators.
      RegExp(r'[\u0000-\u001F\u007F-\u009F\u2028\u2029]'),
      ' ',
    );
    return flattened.replaceAll(RegExp(r'\s+'), ' ').trim();
  }

  /// The opaque bytes, exactly as the host supplied them.
  ///
  /// **WIRE REALITY.** The ZIP-302 `0xFF` field carries no length framing:
  /// anything under 511 bytes is ZERO-PADDED on the wire and reads back as the
  /// full 511. A host that needs its exact length frames it INSIDE these bytes.
  final List<int> bytes;

  /// One short, human-readable sentence saying what the attachment is for,
  /// shown to the user at the authorization step, per send, never dismissible
  /// and never remembered.
  final String purpose;
}

/// Why a ZIP-321 URI was rejected at the [WalletSendRequest.fromUri] seam. No
/// payload (§5.4 — never echoes URI bytes); the host maps it to honest copy.
enum WalletSendRequestFault {
  /// The CATCH-ALL for every non-[wrongNetwork] parse failure: not a usable
  /// `zcash:` request URI — unparseable / oversized / a syntactically bad leg,
  /// but also a well-formed leg the audited core rejects (e.g. an out-of-range
  /// amount). Maps the core parser's payload-free `PaymentUriInvalid`; a host
  /// surfaces it as "not a valid payment code".
  malformed,

  /// Well-formed, but a leg targets a DIFFERENT network than this wallet (e.g. a
  /// testnet URI scanned into a mainnet wallet) — distinct from [malformed] so
  /// the host can say "this is for a different network".
  wrongNetwork,

  /// The URI carries MORE THAN ONE payment leg. Valid ZIP-321, but the
  /// single-recipient send form cannot hold it — rejected rather than silently
  /// paying only the first leg (which would also break the single-leg `selfSend`
  /// authorization invariant; see the coupling note in send_authorization.dart).
  multiplePayments,

  /// The URI carries a machine (ZIP-302 0xFF) or reserved memo the editable
  /// text field cannot represent — rejected rather than silently dropped (a
  /// payment-reference the recipient may need must not vanish).
  unsupportedMemo,
}

/// A typed, payload-free rejection of a ZIP-321 prefill URI at the entry seam
/// ([WalletSendRequest.fromUri]) — surfaced to the host BEFORE any navigation,
/// so it shows an honest error rather than opening a half-filled send form.
class WalletSendRequestException implements Exception {
  const WalletSendRequestException(this.fault);

  final WalletSendRequestFault fault;

  @override
  String toString() => 'WalletSendRequestException(${fault.name})';
}

/// Map the audited parser's per-leg [ParsedPayment] result to a prefill request,
/// enforcing the seam's single-leg + text-memo policy. PURE — no bridge, no
/// network — so it is unit-tested at its boundary against fixture legs. The
/// adapter ([WalletSession.parseSendRequest]) calls this after the (audited,
/// network-checked) `parsePaymentUri` bridge crossing; malformed/wrong-network
/// are already typed by then and mapped from the bridge's `WalletApiError`.
///
/// Throws [WalletSendRequestException] for a shape the editable single-recipient
/// form cannot hold (empty ⇒ [WalletSendRequestFault.malformed]; multi-leg;
/// non-text memo).
WalletSendRequest walletSendRequestFromParsedLegs(List<ParsedPayment> legs) {
  // The core parser rejects an empty request before returning, but defend the
  // boundary: zero legs is nothing to prefill — treat as malformed, never an
  // empty form.
  if (legs.isEmpty) {
    throw const WalletSendRequestException(WalletSendRequestFault.malformed);
  }
  if (legs.length > 1) {
    throw const WalletSendRequestException(
      WalletSendRequestFault.multiplePayments,
    );
  }
  final leg = legs.first;
  final memo = switch (leg.memo) {
    ParsedMemo_Empty() => null,
    ParsedMemo_Text(:final text) => text,
    // Arbitrary (machine) / Reserved / an unknown future arm: not representable
    // in the editable text field — reject rather than silently drop.
    _ => throw const WalletSendRequestException(
      WalletSendRequestFault.unsupportedMemo,
    ),
  };
  return WalletSendRequest(
    address: leg.recipientAddress,
    amountZat: leg.amountZat,
    memo: memo,
    label: leg.label,
  );
}

/// Whether two requests are THE SAME request to the send flow — the identity a
/// final "no transaction" attaches to (stage S8 `deadline`, R05): after the
/// entry's grace revoked a request, a host re-navigation onto the send path
/// with the same request is refused, and one with a different request is a
/// new host act that pays.
///
/// The rule, in the host's terms: when BOTH requests carry a tag
/// ([WalletSendRequest.correlationId]) the host has said what identifies them,
/// so the tags decide — the same tag is the same request, a different tag is a
/// different one, whatever the fields say. Otherwise the payment fields decide:
/// untagged on both sides, which is what makes two `const` literals with the
/// same fields — one object in Dart — the same request without a special
/// case; and tagged on ONE side only, because a tag the other side never
/// carried is not a host's statement that two payments differ (the diff
/// review, ADR-0558: an untagged request the grace revoked, re-navigated with
/// the same fields plus a retry tag, paid — the paying direction on a money
/// predicate; the fields are the safe side, and a fresh `push` is always a new
/// grant). Pure + total.
bool sameSendRequest(WalletSendRequest a, WalletSendRequest b) {
  if (identical(a, b)) return true;
  final tagA = a.correlationId;
  final tagB = b.correlationId;
  if (tagA != null && tagB != null) return tagA == tagB;
  final memoA = a.machineMemo;
  final memoB = b.machineMemo;
  final sameMachineMemo = memoA == null || memoB == null
      ? memoA == memoB
      : memoA.purpose == memoB.purpose && listEquals(memoA.bytes, memoB.bytes);
  return a.address == b.address &&
      a.amountZat == b.amountZat &&
      a.memo == b.memo &&
      a.label == b.label &&
      a.lockRecipient == b.lockRecipient &&
      sameMachineMemo;
}

/// Map a `parsePaymentUri` bridge failure to the seam's typed fault. Reads the
/// typed `WalletApiError.kind` ONLY (never a payload — §5.4); a cross-network
/// URI is [WalletSendRequestFault.wrongNetwork] (distinct, renderable), and
/// anything else — malformed, oversized, a bad leg — collapses to
/// [WalletSendRequestFault.malformed]. Pure + total.
WalletSendRequestException walletSendRequestFaultFromApiError(Object error) {
  if (error is WalletApiError &&
      error.kind is WalletErrorKind_NetworkMismatch) {
    return const WalletSendRequestException(
      WalletSendRequestFault.wrongNetwork,
    );
  }
  return const WalletSendRequestException(WalletSendRequestFault.malformed);
}
