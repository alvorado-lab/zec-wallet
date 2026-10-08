import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart'
    show Clipboard, HapticFeedback, LengthLimitingTextInputFormatter;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../../core/router/wallet_navigation.dart';
import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import '../../../core/theme/shapes.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/address_text.dart';
import '../../../shared/decimal_input_formatter.dart';
import '../../../shared/wallet_cta.dart';
import '../../../shared/wallet_dialog.dart';
import '../../../shared/wallet_info_button.dart';
import '../../../shared/wallet_notice.dart';
import '../../../shared/wallet_outcome_unknown.dart';
import '../swap/swap_address_scanner.dart';
import '../deshield_warning.dart';
import '../labeled_zat_row.dart';
import '../onboarding/onboarding_providers.dart';
import '../wallet_display_sync_status.dart';
import '../wallet_providers.dart';
import '../wallet_session.dart';
import '../sync_status_presentation.dart' show walletSyncPausedQualified;
import '../wallet_rescan_controller.dart'
    show WalletCatchUpNone, walletCatchUpCueProvider;
import '../wallet_sync_controller.dart' show walletSyncPassesRunProvider;
import '../zat_format.dart';
import 'form_fault_view.dart';
import '../../../shared/wallet_ack_checkbox.dart';
import 'large_send_confirm.dart';
import '../../../src/send/wallet_send_channel.dart';
import 'send_controller.dart';
import 'send_state.dart';
import 'wallet_send_report.dart';
import 'wallet_send_request.dart';

/// The send screen (inc-2d-ui): form → propose → confirm (de-shield disclosure)
/// → send → result, over the inc-2d-ffi bridge. Rendering layer ONLY — every
/// money step runs through [sendControllerProvider]; Rust is the single source of
/// truth (design invariant 1). Reachable only from the active wallet surface, so
/// a backed-up wallet is the structural precondition (the same money-safety gate
/// `walletSessionProvider` enforces).
///
/// Stateful so it can (a) hold the form's ephemeral text controllers across the
/// propose round-trip (a fixable fault returns to the form with the typed values
/// intact), and (b) request screenshot/recents protection ([ScreenSecurity]) for
/// its whole lifetime — the screen shows a recipient address + amounts, sensitive
/// financial data that must not land in the app-switcher snapshot. FLAG_SECURE is
/// Android-only; iOS/desktop rely on the manager-carried native cover (the same
/// honest-degradation as the backup screen).
class SendScreen extends ConsumerStatefulWidget {
  const SendScreen({super.key, this.prefill, this.reporter});

  /// FR-25 — an optional prefill (from [WalletSendEntry.push], via GoRouter
  /// `extra`). `null` = the organic empty form. Seeds the (screen-owned) form
  /// controllers in [initState]; `lockRecipient` opens the recipient read-only.
  /// A prefill is NOT pre-confirmed — the same validate → review → authorize
  /// path and the same gating run as an organic send.
  final WalletSendRequest? prefill;

  /// FR-26 — the one-shot channel [WalletSendEntry.push] hands in with the
  /// prefill, carrying this flow's report back to the host. `null` for an
  /// organic entry, and for a host that navigated the send path itself: those
  /// flows have nobody to report to.
  ///
  /// The screen ALWAYS answers a non-null channel — with the outcome the moment
  /// it lands, or, failing that, from [dispose]. A host awaiting the report
  /// must never be left hanging on a money surface.
  final WalletSendReporter? reporter;

  @override
  ConsumerState<SendScreen> createState() => _SendScreenState();
}

class _SendScreenState extends ConsumerState<SendScreen> {
  final _addressController = TextEditingController();
  final _amountController = TextEditingController();
  final _memoController = TextEditingController();

  /// The LIVE classification of the recipient field (the §5.1 privacy axis +
  /// validity). Recomputed SYNCHRONOUSLY on every recipient change via the local,
  /// offline-safe [classifyRecipient] seam — it gates the memo field and the
  /// Review/Queue actions, and drives the inline status line. Starts empty (the
  /// fresh form has no recipient yet).
  RecipientStatus get _recipientStatus => _recipientStatusValue;
  set _recipientStatus(RecipientStatus value) {
    _recipientStatusValue = value;
    // Every recipient change (typed, pasted, prefilled, cleared) withdraws
    // the Queue acknowledgement: it was given for the address it named.
    _queuePublicAcknowledged = false;
  }

  RecipientStatus _recipientStatusValue = const RecipientEmpty();

  /// The public-payment acknowledgement on the QUEUE path ("add
  /// the Send acknowledgement like Swap"). Queue saves a payment WITHOUT the
  /// review screen, so a queued payment to a transparent address would otherwise
  /// carry no warning at all. Reset by [_recipientStatus]'s setter.
  bool _queuePublicAcknowledged = false;

  /// Queue waits on the acknowledgement only for a public recipient.
  bool get _queueAllowed =>
      _recipientStatus.isSendable &&
      (_recipientStatus is! RecipientTransparent || _queuePublicAcknowledged);

  /// Captured in [initState] so [dispose] can release protection WITHOUT touching
  /// `ref` (Riverpod guidance — `ref` is unsafe once deactivated).
  late final ScreenSecurity _security;

  /// Captured in [initState] so [dispose] can ask which flows actually ENTERED
  /// a spend without touching `ref` (the [_security] discipline). The notifier
  /// is root-scoped and riverpod reuses the instance across a session flip, so
  /// the reference stays live.
  late final SendController _sendController;

  /// Anchors the inline fault so a newly-surfaced propose fault can be scrolled
  /// into view — at large text scale it renders below the fold, under a disabled
  /// Review button, and a sighted user would otherwise see no "why" (#329-2).
  final GlobalKey _faultKey = GlobalKey();

  /// FR-25: whether the recipient field is READ-ONLY (a host locked a resolved
  /// pay-a-contact). Recipient-only — amount/memo stay editable. Mutable, not a
  /// getter off `widget.prefill`, for two money-safe reasons: it is seeded in
  /// [initState] ONLY when the locked recipient is actually SENDABLE (a lock over
  /// an invalid/wrong-network address would otherwise strand the form read-only
  /// with a Review it can never enable), and it is RELEASED on a session flip
  /// (the flip clears the address, so a still-locked empty field would be a
  /// dead-end — the form must fall back to an editable organic send).
  bool _recipientLocked = false;

  /// FR-28: the host's machine memo for THIS entry — opaque bytes plus the
  /// purpose the user is shown before authorizing.
  ///
  /// Screen state, seeded from the request and cleared with it. Deliberately
  /// NOT derived from any field: the bytes are not editable and not
  /// re-derivable, so holding them anywhere the form rebuilds from is how they
  /// went missing at re-compose in the first place. Released on a session flip
  /// with the rest of the draft — a machine memo belongs to the identity that
  /// was asked to send it.
  WalletMachineMemo? _machineMemo;

  /// A scanned code that could not be used (S13 §1.2): shown inline under the
  /// recipient, and the draft is left exactly as it was. Cleared by the next
  /// recipient change.
  bool _scanFault = false;

  /// Every flow this screen has driven on the root-scoped controller — its
  /// entry reset's, and the one under each Review / Queue tap (S13 §1a H1).
  /// All of them are abandoned at teardown. Recorded per TAP, not only at
  /// entry: a session flip or an abandoned-flow restart mints a new id under
  /// a live screen, and the entry-time id alone let that one escape (the
  /// diff review's MEDIUM).
  final Set<int> _drivenFlows = {};

  /// Stage S8 `deadline` (R05): whether the request THIS screen holds has
  /// lost its ability to spend — the entry's mount grace ran out before the
  /// screen appeared, and the host was told "no transaction", which is final.
  ///
  /// A property of the REQUEST, and this State only mirrors it: the report
  /// channel is where the revocation arrives ([WalletSendReporter.isRevoked],
  /// read in [initState] and [didUpdateWidget]), but a bare-request
  /// re-navigation (`replace`/`go` onto the send path with the request alone)
  /// drops the channel and re-mints the flow id, and a FRESH mount with the
  /// same bare request has no previous State at all — the diff review's
  /// finding: the build threaded the bit through `didUpdateWidget` only,
  /// so a user who popped the expired screen and a host that `go`ed back with
  /// the same request paid. So the memory lives on the root-scoped controller
  /// ([SendController.isRevokedRequest], written by the entry's grace; ADR-0558)
  /// and a bare request asks it at every door, keyed on nothing but the
  /// request ([sameSendRequest]). A DIFFERENT request — or a fresh live
  /// channel — is a new host act that pays.
  ///
  /// Two consumers, both needed. The screen renders the expired state itself
  /// (no form, no pay path — and independently of the shared controller's
  /// state, so a stacked entry landing back on this screen cannot reveal a
  /// form under it). And it is handed to the controller at every entry reset,
  /// which refuses the spend at both of its seams before the host's authorizer
  /// is asked — the money gate a widget-only refusal cannot be.
  bool _expired = false;

  /// MED-2: the one-shot watch that finishes a PREFILLED entry's deferred
  /// controller reset when the entry landed mid-flight (see [_scheduleEntryReset]).
  ProviderSubscription<SendState>? _pendingEntryReset;

  /// FR-26: the host channels this screen owes an answer to, by the
  /// [SendController.flowId] each one's flow was given.
  ///
  /// Keyed by flow, not held as a single "am I armed" flag, because ONE
  /// root-scoped controller serves every send screen and a screen stays mounted
  /// (and listening) under a stacked one. A boolean answers "the machine
  /// reached a terminal"; the question is "did MY flow reach one", and the
  /// review found both directions of that confusion reachable — an idle entry
  /// booking the payment made on the entry stacked above it, and an entry
  /// retired by an in-place route update never hearing its own outcome.
  ///
  /// A channel leaves this map exactly once: when its flow's terminal arrives,
  /// or at teardown.
  final Map<int, WalletSendReporter> _owed = {};

  /// FR-26: whether a flow this screen owes an answer for actually ENTERED a
  /// spend — asked of the controller, which is the only thing that knows.
  ///
  /// This used to be a local latch set when the machine reached a
  /// `SendSubmitting`/`SendQueuing` transient. That is a DIFFERENT question,
  /// and it was wrong twice: a queue-time compose failure reaches the transient
  /// and signs nothing, and an authorizer denial reaches it and hands the user
  /// back a live Review. Both then graded as "we lost the answer" over a flow
  /// that provably created nothing.
  bool _enteredSpend(int flowId) => _sendController.hasEnteredSpend(flowId);

  @override
  void initState() {
    super.initState();
    // Request screenshot/recents protection for the whole time financial data is
    // on screen. Best-effort defence-in-depth (swallow failures); ref.read in
    // initState is allowed (no watch).
    _security = ref.read(screenSecurityProvider);
    _sendController = ref.read(sendControllerProvider.notifier);
    unawaited(_security.enable());
    // FR-26: take ownership of the report channel BEFORE anything can fail.
    // From here the entry helper stops watching it — this screen answers.
    widget.reporter?.markAttached();
    // NO setState in the seeding here — initState runs before the first build.
    _applyPrefill();
    // Stage S8 `deadline`: and read whether the request arrived already
    // revoked (the grace ran out before this mount) — before the entry reset
    // below hands the bit to the controller.
    _noteRevocation();
    _scheduleEntryReset();
  }

  /// Stage S8 `deadline`: (re)derive [_expired] for the request this screen
  /// now holds. From [initState] and [didUpdateWidget] (inside setState) —
  /// the same read at both, so a fresh mount and an in-place update agree.
  ///
  /// A channel, when present, decides for itself: a revoked one revokes the
  /// request it came with, a live one is a new push the host awaits — a new
  /// act, even for a request with the same fields. With NO channel (a bare
  /// request, or the organic form) the controller's memory of what the grace
  /// revoked decides: the R05 shape is a host re-navigating onto the send path
  /// — in place or onto a fresh mount — with the request it already holds a
  /// final negative for, and that request must not pay; a different request
  /// is a new act (row 789 stands).
  void _noteRevocation() {
    final prefill = widget.prefill;
    final reporter = widget.reporter;
    if (reporter != null) {
      _expired = reporter.isRevoked && prefill != null;
      return;
    }
    _expired = prefill != null && _sendController.isRevokedRequest(prefill);
  }

  /// FR-25: seed the (screen-owned) controllers + the live recipient status +
  /// the lock from [SendScreen.prefill] — or clear them for a null prefill. The
  /// address's text also seeds the live recipient classification (the memo gate
  /// + Review enablement read it); the amount is a bare-decimal via formatZec
  /// (the exact round-trip inverse of the field's own parser); a memo is
  /// text-only (a non-text URI memo is rejected at the fromUri seam). The
  /// post-frame resetToForm only resets the controller STATE (→ SendForm),
  /// never these text controllers; the session-flip listener in build() clears
  /// them only on an ACTUAL identity change (never the initial value), so the
  /// prefill survives entry but is correctly wiped if the wallet flips under it.
  ///
  /// Called from [initState] (bare) and [didUpdateWidget] (inside setState) —
  /// the an in-place route update (a host `replace`/`go` onto the
  /// send path; the documented [WalletSendEntry.push] always mints a fresh
  /// route) delivers a NEW prefill through didUpdateWidget, and without a
  /// re-seed the form would silently keep the OLD request's fields + lock on a
  /// money surface.
  void _applyPrefill() {
    final prefill = widget.prefill;
    _addressController.clear();
    _amountController.clear();
    _memoController.clear();
    _recipientStatus = const RecipientEmpty();
    _recipientLocked = false;
    _machineMemo = null;
    if (prefill == null) return;
    // FR-28: taken verbatim. No trim, no normalisation, no length re-derivation
    // — the bytes are the host's envelope and any edit here would corrupt it.
    // (`WalletMachineMemo` already refused empty bytes and a blank purpose at
    // construction, so a machine memo that exists is a disclosable one.)
    _machineMemo = prefill.machineMemo;
    // Robustness clamps (invariant 7 — the field formatters/maxLength run only
    // on USER input, not programmatic sets): mirror the address field's
    // 512-grapheme cap and the memo's 512-char cap so a buggy typed-request
    // host can't push an unbounded string across the sync classify FFI or into
    // the widget tree. URI-sourced values are already parser-bounded; the Rust
    // gates (ADDRESS_MAX_BYTES / MemoTooLong) stay the authoritative backstop.
    final address = prefill.address.length > 512
        ? prefill.address.substring(0, 512)
        : prefill.address;
    _addressController.text = address;
    final amountZat = prefill.amountZat;
    if (amountZat != null && amountZat > 0) {
      _amountController.text = formatZec(amountZat);
    }
    final memo = prefill.memo;
    if (memo != null && memo.isNotEmpty) {
      _memoController.text = memo.length > 512 ? memo.substring(0, 512) : memo;
    }
    final session = ref.read(walletSessionProvider);
    if (session != null) {
      _recipientStatus = classifyRecipient(session, address);
    }
    // Lock ONLY a sendable recipient. Locking an invalid / wrong-network /
    // (session-null) unclassified address would leave the field read-only AND
    // non-sendable — Review disabled with no way for the user to correct it. A
    // non-sendable locked prefill instead opens EDITABLE so its inline status
    // line explains the problem and the address can be fixed. (This gate is
    // part of the PUBLIC lockRecipient contract — documented on
    // [WalletSendRequest.lockRecipient].)
    _recipientLocked = prefill.lockRecipient && _recipientStatus.isSendable;
  }

  /// Fresh form each entry: the send controller is non-autoDispose
  /// (root-scoped), so reset any prior terminal result to a clean form.
  /// Post-frame so a provider is never mutated mid-build; re-check mounted
  /// inside (the project pattern).
  ///
  /// MED-2 (the seam-created re-attach gap): `resetToForm` deliberately
  /// NO-OPS while a step is in flight (the -F1 re-attach contract — an
  /// ORGANIC re-entry must show the running send land). But a PREFILLED entry
  /// is unambiguously a NEW send intent, and before FR-25 nothing external
  /// could stack a send screen mid-submit — so without the deferral below, a
  /// host push during SendSubmitting would render the PREVIOUS send's outcome
  /// under the new entry (the user attributes send A's "Sent" to request B).
  /// Deferral: watch the controller one-shot; the moment the in-flight step
  /// LANDS, reset to this entry's fresh form. Send A's outcome stays on the
  /// durable surfaces (the in-flight cue, activity, balance — invalidated at
  /// outcome-landing by the controller), so nothing is hidden.
  ///
  /// Stage S8 `deadline`: the reset carries the request's revocation
  /// ([_expired]) to the controller — every reset, so the bit survives the
  /// fresh flow id each one mints — and a revoked request lands the machine
  /// on [SendRequestExpired], which counts as landed exactly like a form.
  void _scheduleEntryReset() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final controller = ref.read(sendControllerProvider.notifier);
      controller.resetToForm(requestRevoked: _expired);
      final landed = switch (ref.read(sendControllerProvider)) {
        SendForm() || SendRequestExpired() => true,
        _ => false,
      };
      // FR-26: the ordinary case — the reset took, so the flow it just minted
      // is ours and its terminal is the one we report.
      if (landed) {
        _drivenFlows.add(controller.flowId);
        _claimFlow(controller.flowId);
      }
      if (landed || widget.prefill == null) return;
      // Prefilled entry over an in-flight step: defer the reset to the landing.
      _pendingEntryReset?.close();
      _pendingEntryReset = ref.listenManual(sendControllerProvider, (
        prev,
        next,
      ) {
        if (next is SendPreparing ||
            next is SendSubmitting ||
            next is SendQueuing) {
          return; // still in flight
        }
        _pendingEntryReset?.close();
        _pendingEntryReset = null;
        if (!mounted) return;
        final controller = ref.read(sendControllerProvider.notifier);
        controller.resetToForm(requestRevoked: _expired);
        // FR-26: our flow starts HERE, after the prior entry's step landed —
        // which is exactly why that landing was not ours to report.
        _drivenFlows.add(controller.flowId);
        _claimFlow(controller.flowId);
      });
    });
  }

  /// FR-26: take ownership of [flowId] on behalf of this entry's channel, so
  /// its terminal — and only its terminal — becomes this host's report.
  void _claimFlow(int flowId) {
    final reporter = widget.reporter;
    if (reporter == null || reporter.isReported) return;
    _owed[flowId] = reporter;
  }

  @override
  void didUpdateWidget(SendScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    // MED-1: a NEW request arriving on the SAME State (an in-place route
    // update — `replace`/`go` onto the send path with a different `extra`;
    // every [WalletSendEntry.push] mints a fresh route and never lands here)
    // must re-seed, or the form silently keeps the OLD request's fields + lock
    // while widget.prefill is the new one.
    // FR-26: the channel is what has to be tracked here, NOT the request.
    // `WalletSendRequest` is const-constructible, so two identical literals in
    // host code are the SAME object — keying on the prefill alone let an
    // in-place route update retire a channel without ever answering it, and
    // the entry helper's own fallback declines it too (the screen had already
    // attached). The host's await on a money surface then never completed.
    final reporterChanged = !identical(widget.reporter, oldWidget.reporter);
    final prefillChanged = !identical(widget.prefill, oldWidget.prefill);
    if (prefillChanged || reporterChanged) {
      if (reporterChanged) {
        // The retiring channel keeps its claim: if its flow is still running,
        // its true terminal is still coming and this screen is still listening
        // for it. Only a flow that never started a money step is closed out
        // now — there is nothing left for it to hear.
        _closeIdleClaims(oldWidget.reporter);
        widget.reporter?.markAttached();
        // The new channel arrived under the CURRENT identity: a flip before
        // it says nothing about this request.
        _requestIdentityGone = false;
      }
      setState(() {
        if (prefillChanged) _applyPrefill();
        // Stage S8 `deadline`: the second site the revocation is read. THIS is
        // the door an IN-PLACE bare-request re-navigation comes through — the
        // channel gone, the flow id about to be re-minted below — and the
        // request must stay unpayable through it while it is the same request
        // (the fresh-mount door reads the same memory in `initState`).
        _noteRevocation();
      });
      _scheduleEntryReset();
    }
  }

  /// FR-26: answer [reporter]'s claims that can no longer land — a flow it owns
  /// that never entered a money step has nothing further to say, so it is
  /// honestly [WalletSendNoTransaction]. A claim on a flow that IS running is
  /// left in place: its terminal is still coming, and the screen is still
  /// listening for it.
  void _closeIdleClaims(WalletSendReporter? reporter) {
    if (reporter == null) return;
    final claimed = _owed.entries
        .where((e) => identical(e.value, reporter))
        .map((e) => e.key)
        .toList();
    var hadClaim = false;
    for (final flowId in claimed) {
      hadClaim = true;
      if (_enteredSpend(flowId)) continue;
      // S13 H1, the in-place door (diff review HIGH): the host is about
      // to hear "no transaction" for this flow, so its pending closure (the
      // authorizer prompt may still be up) must refuse — or an approval now
      // pays under a claim nobody holds any more.
      _sendController.abandonFlow(flowId);
      _owed.remove(flowId);
      if (!reporter.isReported) {
        reporter.report(
          WalletSendNoTransaction(correlationId: reporter.correlationId),
        );
      }
    }
    // Never claimed a flow at all (retired before its entry reset landed):
    // nothing was ever started under it.
    if (!hadClaim && !reporter.isReported) {
      reporter.report(
        WalletSendNoTransaction(correlationId: reporter.correlationId),
      );
    }
  }

  /// FR-26: at teardown, every channel still owed an answer gets one.
  ///
  /// A flow that entered a money step is [WalletSendUnclassified], NOT "nothing
  /// happened": the sign + broadcast outlives the screen, so the honest answer
  /// is that this layer cannot grade it and the wallet holds whatever moved. A
  /// flow that never started one is [WalletSendNoTransaction] — structurally,
  /// not by inference.
  ///
  /// A no-op for a channel already answered: the first answer is the true one.
  void _reportFinal() {
    for (final entry in _owed.entries) {
      final reporter = entry.value;
      if (reporter.isReported) continue;
      reporter.report(
        _enteredSpend(entry.key)
            ? WalletSendUnclassified(correlationId: reporter.correlationId)
            : WalletSendNoTransaction(correlationId: reporter.correlationId),
      );
    }
    _owed.clear();
    // A channel this screen attached but never claimed a flow for (the user
    // left before the entry reset landed) still gets its honest answer.
    final reporter = widget.reporter;
    if (reporter != null && !reporter.isReported) {
      reporter.report(
        WalletSendNoTransaction(correlationId: reporter.correlationId),
      );
    }
  }

  @override
  void dispose() {
    // S13 §1a H1 — BEFORE the reports below. A screen can go by an exit the
    // Back hold cannot stop (a host `pop`/`go`/`replace`, a session teardown),
    // and during the host's authorizer prompt the spend closure is still
    // waiting to run. Abandoning the flows this screen ran makes that closure
    // refuse before it enters, so the "no transaction" `_reportFinal` gives
    // for a flow that has not entered is structurally true, not a guess. A
    // flow that already entered is untouched: its payment keeps going.
    for (final flowId in {..._owed.keys, ..._drivenFlows}) {
      _sendController.abandonFlow(flowId);
    }
    _reportFinal();
    _pendingEntryReset?.close();
    unawaited(_security.disable());
    _addressController.dispose();
    _amountController.dispose();
    _memoController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    // Identity-switch hygiene (wrap review — the duress journey): the
    // typed recipient/amount/memo are SCREEN-owned, so without this they
    // survive a wallet-session flip and render the OLD identity's draft
    // inside the NEW identity's form — a disclosure to a coercer in the
    // decoy flow, and a cross-identity payment draft. Clear them the moment
    // the session changes (`ref.listen` never fires on the initial value).
    ref.listen(walletSessionProvider, (_, _) {
      _addressController.clear();
      _amountController.clear();
      _memoController.clear();
      setState(() {
        _recipientStatus = const RecipientEmpty();
        // Release any FR-25 recipient lock (the address was just cleared, so a
        // still-read-only field would be an empty dead-end): the form becomes a
        // clean, editable organic send for the now-active identity.
        _recipientLocked = false;
        // FR-28: and drop the machine memo with the rest of the draft. It was
        // attached to a payment the PREVIOUS identity was asked to make; a
        // carried-over envelope would put one identity's reference on the
        // other's transaction.
        _machineMemo = null;
        // S13: the host's request went with the draft — no later flow on
        // this screen is re-claimed for it.
        _requestIdentityGone = true;
      });
    });
    // FR-26 — the host's report, delivered the MOMENT the flow's terminal
    // lands, not at the pop: the user can leave by any exit, and a payment that
    // lands as they go is still a payment.
    //
    // It reads the controller's OWN published terminal, not the rendering
    // state, for two reasons the review made concrete. The terminal
    // carries a flow id, so a screen mounted under a stacked one cannot book
    // the payment made above it. And the controller publishes even when it
    // rightly REFUSES the state write — a landing that arrives after a session
    // flip is swallowed by the identity guard, and reading that silence as an
    // absence claims that no money moved over a broadcast transaction.
    ref.listen(sendFlowOutcomeProvider, (prev, next) {
      if (next == null) return;
      final reporter = _owed.remove(next.flowId);
      if (reporter == null || reporter.isReported) return;
      reporter.report(
        walletSendReportFor(next, correlationId: reporter.correlationId),
      );
      // Duress hygiene: the host has now been told, so the
      // terminal's TXIDS have no further job — drop them instead of leaving the
      // previous identity's payment identifiers resident in a root provider.
      // Deferred to a post-frame callback, NOT called inline: every other
      // listener for this same notification must run first, or a sibling
      // screen's `ref.listen` would wake to a `null` it cannot report.
      final deliveredFlowId = next.flowId;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        ref.read(sendFlowOutcomeProvider.notifier).consume(deliveredFlowId);
      });
      // The result screen's affordances depend on whether a host has already
      // been told (see [_SendResultView.hostReported]).
      if (mounted) setState(() {});
    });
    // When a propose returns a fixable fault, make sure the user can SEE it: at
    // large text scale it lands below the fold under a disabled Review. Faults
    // surface only on a Review/Queue/Confirm tap (never per keystroke or on a
    // background sync tick — the controller watches only the session provider),
    // so this fires exactly when the user asked for a result, never fighting a
    // manual scroll.
    //
    // The `identical` guard is sufficient because every user-visible fault
    // either is a fresh instance (amount/ceiling faults) or re-lands AFTER a
    // transient (Preparing/Submitting/Queuing) that unmounts then remounts the
    // fault node — so a repeat of the same categorical reason still presents as
    // a prev==null → non-null transition and re-fires. INVARIANT: never set a
    // canonical (const) fault directly from a fault-carrying form with no
    // transient in between, or this (and the liveRegion re-announce) would go
    // silent for a same-reason repeat.
    ref.listen(sendControllerProvider, (prev, next) {
      final prevFault = prev is SendForm ? prev.fault : null;
      final nextFault = next is SendForm ? next.fault : null;
      if (nextFault == null || identical(nextFault, prevFault)) return;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        final faultContext = _faultKey.currentContext;
        if (faultContext == null) return;
        // alignment 1.0 pins the fault to the viewport's trailing edge — the
        // ListView's bottom, immediately above the pinned Review bar (its own
        // sibling below the list), so the "why is this disabled" message lands
        // right beside the disabled button with the offending field still above
        // it. On a form too short to scroll there is nothing to move, so it is a
        // no-op in practice; a live region carries the full text to screen
        // readers regardless of where it renders.
        unawaited(
          Scrollable.ensureVisible(
            faultContext,
            alignment: 1.0,
            duration: const Duration(milliseconds: 250),
            curve: Curves.easeOut,
          ),
        );
      });
    });
    // The Queue acknowledgement belongs to ONE payment (diff review, LOW):
    // coming back to the form from anywhere else — "Send another", Back from
    // the review, a fault — withdraws it, as the review's box resets per
    // proposal and Swap's per quote.
    ref.listen(sendControllerProvider, (prev, next) {
      if (next is SendForm && prev is! SendForm && _queuePublicAcknowledged) {
        setState(() => _queuePublicAcknowledged = false);
      }
    });
    // The send screen is reached only from the active wallet, but a wallet close
    // mid-screen would null the session — render an honest "go back" rather than
    // a stale form (money-safety: no send form without a live wallet).
    final session = ref.watch(walletSessionProvider);
    // #397 P1a — re-gate on the watch-only KIND (money-safety: the gate lives
    // with the money surface, not the navigation). The wallet-screen chrome
    // hides Send, but a deep-link / prefilled entry straight to /wallet/send on
    // a watch-only wallet would otherwise render a send form whose propose the
    // SDK then refuses typed. Render the honest "can't send" state instead —
    // synchronous (the kind is captured on OnboardingActive), so no send form
    // flashes first.
    final watchOnly = ref.watch(isWatchOnlyProvider);
    final sendState = ref.watch(sendControllerProvider);
    // S13 §1.2 + §1a H1/H2 — no leaving by Back while a spend is being made.
    // Back and `maybePop` are held during SendSubmitting and SendQueuing (the
    // authorize, sign and broadcast are unbounded), and ask instead: sending
    // continues, and leaving does not cancel it. A pop after the result works.
    final holding = sendState is SendSubmitting || sendState is SendQueuing;

    return PopScope<Object?>(
      canPop: !holding,
      onPopInvokedWithResult: (didPop, _) {
        if (didPop) return;
        unawaited(_confirmLeaveWhileSending());
      },
      child: Scaffold(
        appBar: AppBar(title: Text(l10n.walletSendTitle)),
        body: SafeArea(
          child: session == null
              ? const _SendUnavailable()
              : watchOnly
              ? const _SendWatchOnly()
              : _body(sendState),
        ),
      ),
    );
  }

  /// H2: the hold must never strand the user on an unbounded step. Leave pops
  /// this route directly (the hold stops Back only); the teardown then
  /// reports the honest outcome under H1.
  Future<void> _confirmLeaveWhileSending() async {
    final l10n = WalletLocalizations.of(context);
    final leave = await showWalletConfirm(
      context,
      title: l10n.walletSendLeaveTitle,
      body: l10n.walletSendLeaveBody,
      confirmLabel: l10n.walletSendLeaveConfirm,
      cancelLabel: l10n.walletSendLeaveStay,
      kind: WalletConfirmKind.neutral,
      confirmKey: const ValueKey('send-leave-confirm'),
      cancelKey: const ValueKey('send-leave-stay'),
    );
    if (!leave || !mounted) return;
    // Pop only THIS screen's route: if something was pushed above it while the
    // dialog was up (a late host prompt), popping would dismiss THAT instead —
    // which a host may read as a decline (the fold review's LOW). The
    // user deals with what is on top first. With nothing under this route (a
    // deep-link root), go to the wallet.
    final route = ModalRoute.of(context);
    if (route != null && !route.isCurrent) return;
    final navigator = Navigator.of(context);
    if (navigator.canPop()) {
      navigator.pop();
    } else {
      context.leaveToWalletRoot();
    }
  }

  Widget _body(SendState state) {
    final l10n = WalletLocalizations.of(context);
    // Stage S8 `deadline`: a screen holding a revoked request renders the
    // expired state from its OWN bit, whatever the shared machine says — the
    // controller is root-scoped and a stacked entry can leave it on a form
    // that is not this request's. The machine's own arm below is the same
    // view, reached when the controller refused a spend under the bit.
    if (_expired) return const _SendRequestExpired();
    return switch (state) {
      SendForm(:final fault) => _buildForm(fault),
      SendRequestExpired() => const _SendRequestExpired(),
      SendPreparing() => _SendBusy(label: l10n.walletSendPreparing),
      SendReview(
        :final proposal,
        :final recipient,
        :final machineMemoPurpose,
      ) =>
        _SendReviewView(
          // A new proposal is a new review: its acknowledgement starts
          // unticked (the state is keyed to the proposal it acknowledged).
          key: ObjectKey(proposal),
          proposal: proposal,
          recipient: recipient,
          machineMemoPurpose: machineMemoPurpose,
        ),
      SendSubmitting() => _SendBusy(label: l10n.walletSendSubmitting),
      SendSent(:final outcome) => _SendResultView(
        outcome: outcome,
        hostReported: widget.reporter?.isReported ?? false,
        machineMemoAttached: _machineMemo != null,
      ),
      SendQueuing() => _SendBusy(label: l10n.walletSendQueuing),
      SendQueued() => const _SendQueuedView(),
      SendOutcomeUnknown(:final queued) => _SendOutcomeUnknownView(
        queued: queued,
      ),
    };
  }

  /// The editable form. Inlined (not a child widget) so the Review / Queue
  /// actions read the controllers the screen owns.
  ///
  /// Layout: the scrollable fields sit in an [Expanded] list; the primary
  /// actions are PINNED in a bottom bar. So when a fault appears above, the
  /// Review button never jumps (no layout shift on error) and stays reachable
  /// above the keyboard.
  Widget _buildForm(SendFormFault? fault) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    // The honest available-to-send figure (spendable, the §2.5 SSOT) — last-known
    // through a transient reload error, like the wallet surface.
    final spendable = ref
        .watch(walletSnapshotProvider)
        .value
        ?.balance
        .spendableZat;
    // Offline-first affordance: show "queue for later" when the wallet can't
    // reach the network right now, or when a propose just failed because it isn't
    // synced far enough — never as the default (queuing skips the fee preview).
    // Gated on the host's custody supporting a background drain at all (#327):
    // per-send-credential custody can never sign a drained queue, so the
    // affordance is HIDDEN, not offered-then-faulted.
    final queueSupported = ref.watch(walletOfflineQueueSupportedProvider);
    // …and on the host's sync POLICY (the R1 gate): the queue drains
    // ONLY on sync passes, so under policy-off a queued payment is committed and
    // then sits there — the recipient is never paid while the surfaces around it
    // point at a drain that cannot run. Same honesty rule as `queueSupported`:
    // hidden, not offered-then-stranded. (Already-queued sends stay parked and
    // visible; the parked section states the pause.)
    //
    // The justification used to name a "next time you're online" PROMISE — copy
    // #401 R1(a) deleted, because a background pass at host custody holds no
    // signing credential and cannot keep it (#403 R9g). The gate itself is
    // unchanged and still right: what it prevents is committing a payment onto a
    // surface whose only exit is a drain the host has switched off.
    final syncPolicyOn = ref.watch(walletSyncPolicyProvider);
    // The posture-stable DISPLAY status (#399 item 3), never the raw stream:
    // FR-21's sub-second retry blips would otherwise insert/remove the queue
    // button in the pinned action bar under the user's thumb every cycle.
    final sync = ref.watch(walletDisplaySyncStatusProvider).value;
    // CONNECTIVITY stalls only (#399 item 2): the queue's promise — "a future
    // online sync drains this" — is true when the network/Tor path is what's
    // down, and FALSE for a storage-full/internal/reorg/unknown stall (the
    // wallet is online there; storageFull/internal can't even durably drain,
    // and the old any-Stalled gate told those users "waiting for a
    // connection" while their connection was fine).
    final offlineish =
        sync is SyncStatus_Offline ||
        (sync is SyncStatus_Stalled &&
            (sync.reason == StallReason.endpointUnreachable ||
                sync.reason == StallReason.torUnavailable));
    final notSynced =
        fault is SendCategoricalFault &&
        fault.reason == SendFaultReason.notSyncedYet;
    // Ironwood/NU6.3 (`ironwood-nu63-support.md` §9.1 step 9): never offer the
    // queue beside a network-upgrade refusal, INCLUDING when the wallet is also
    // offline. `offlineish` is a sync-state test that knows nothing about the
    // fault, so without this the offer reappeared exactly where the product
    // call said it must not (security review) — and "queue to send later"
    // beside a wait only an APP UPDATE ends is a promise the app cannot keep.
    final blockedByUpgrade =
        fault is SendCategoricalFault &&
        fault.reason == SendFaultReason.networkUpgradeUnsupported;
    final showQueue =
        queueSupported &&
        syncPolicyOn &&
        !blockedByUpgrade &&
        (offlineish || notSynced);

    return Column(
      children: [
        Expanded(
          child: ListView(
            padding: const EdgeInsets.all(16),
            children: [
              // The line RESERVES its space through a null-spendable window
              // (first load; the post-"Send another" snapshot refresh) so the
              // form fields never jump when the figure lands (#356-NIT — the
              // action-row reason's no-layout-shift treatment). Hidden ⇒ also
              // dropped from semantics; the placeholder figure is never shown.
              // maintainSize reserves the PLACEHOLDER's box, so the slot is
              // pinned to EXACTLY one text line (reliability MED): a long
              // amount at a 2.0× accessibility scale would otherwise wrap to
              // two where the "0 ZEC" placeholder reserved one — re-creating
              // the exact jump this reserve exists to remove. FittedBox (never
              // ellipsis — an amount must not truncate, the #329 rule) scales
              // the rare too-wide figure down INSIDE the slot; the invisible
              // one-line strut keeps the slot's height at a full line even
              // then, so the reserve is exact in every state.
              Padding(
                padding: const EdgeInsets.only(bottom: 16),
                child: Visibility(
                  visible: spendable != null,
                  maintainSize: true,
                  maintainAnimation: true,
                  maintainState: true,
                  // While the wallet is still catching up (#381 (a), the #380
                  // swap-line fold applied here): the spendable figure is the
                  // PARTIAL repopulating balance — qualify it so a low/zero
                  // "Available" (incl. the post-"Send another" refresh) is
                  // never read as final or as lost funds.
                  // …and the qualifier is DROPPED under the host's sync-off
                  // policy: "still catching up" claims the active
                  // progress the same form's fault arm just stopped claiming
                  // — one card must not contradict itself. The plain figure
                  // + the ambient sync-off story (badge, disabled-Send
                  // reason, activity note) carry the not-final caveat there.
                  child:
                      ref.watch(walletCatchUpCueProvider)
                              is WalletCatchUpNone ||
                          !syncPolicyOn
                      // SYNCED: the short line — keep the exact one-line
                      // FittedBox reserve (the figure must never truncate, the
                      // #329 rule; a rare too-wide amount scales down inside a
                      // pinned single line so the fields below never jump).
                      ? Stack(
                          alignment: AlignmentDirectional.centerStart,
                          children: [
                            ExcludeSemantics(
                              child: Opacity(
                                opacity: 0,
                                child: Text(' ', style: textTheme.bodySmall),
                              ),
                            ),
                            FittedBox(
                              fit: BoxFit.scaleDown,
                              child: Text(
                                l10n.walletSendAvailable(
                                  formatZec(spendable ?? 0),
                                ),
                                maxLines: 1,
                                style: textTheme.bodySmall?.copyWith(
                                  color: colors.textMuted,
                                ),
                              ),
                            ),
                          ],
                        )
                      // CATCHING UP: the qualifier makes the line 2-3x longer;
                      // a one-line FittedBox would shrink the FIGURE to an
                      // unreadable ~7px and — being width-bound — ignore the
                      // user's text-scale entirely (UX HIGH). Wrap it
                      // exactly like the move/swap Available lines instead: a
                      // plain Text that softwraps (the amount never truncates —
                      // it flows to the next line), sized by `maintainSize` so
                      // the fields below still don't jump when the figure
                      // lands (the qualifier dominates the height, so the
                      // null-"0" reserve matches the partial-figure render).
                      : Text(
                          l10n.walletSendAvailableCatchingUp(
                            formatZec(spendable ?? 0),
                          ),
                          style: textTheme.bodySmall?.copyWith(
                            color: colors.textMuted,
                          ),
                        ),
                ),
              ),
              // FR-25 lock a11y (fold — the mechanism VERIFIED against the
              // framework): an ancestor Semantics LABEL coalesces into the text
              // field's own merged node, so a screen reader announces the locked
              // state ON FOCUS of the field. A suffix-icon semanticLabel does
              // NOT do this (InputDecorator tags the suffix into a SIBLING merge
              // group — a separate swipe stop), and neither does helperText
              // (its own container node), and the native readOnly flag is
              // silent on TalkBack/VoiceOver — so the ancestor label is the one
              // channel that reaches the field itself. The lock icon stays
              // DECORATIVE (no semanticLabel — no duplicate stop); the visible
              // helperText carries the same line for sighted users. Pinned at
              // the semantics level (mutant-verified: the icon mechanism FAILS
              // the pin).
              // #401 R3a, CORRECTED — see the note above `_sendDecoration`. The
              // field's NAME already reaches its semantics node through
              // `labelText`; only the STATE sentence needs this ancestor channel,
              // which is what it carried before and carries again.
              Semantics(
                label: _recipientLocked ? l10n.walletSendRecipientLocked : null,
                child: TextField(
                  controller: _addressController,
                  autocorrect: false,
                  enableSuggestions: false,
                  onChanged: _onRecipientChanged,
                  // FR-25 lockRecipient (recipient-ONLY — amount/memo never
                  // lock): a host that resolved the payee itself
                  // (pay-a-contact) opens the recipient READ-ONLY so it can't
                  // be edited into a different address. read-only, NOT
                  // disabled — the field stays selectable so the user can
                  // verify + copy who they're paying, and it keeps full
                  // contrast (a greyed field reads as "broken", not "fixed").
                  // onChanged can't fire while read-only, so the seeded
                  // recipient status stands.
                  readOnly: _recipientLocked,
                  // Boundary cap (invariant 7) — a Zcash address never exceeds
                  // the SDK's ADDRESS_MAX_BYTES (512). The formatter caps the
                  // field at 512 GRAPHEMES, bounding a paste to a few KB so it
                  // can't cross the sync FFI unbounded on every keystroke; the
                  // Rust `Address::parse` byte-cap is the precise,
                  // authoritative backstop (proven hostile-input-safe by its
                  // own never-panic proptest). A length formatter, not
                  // `maxLength`, so the field shows no char counter.
                  inputFormatters: [LengthLimitingTextInputFormatter(512)],
                  decoration:
                      _sendDecoration(
                        context,
                        label: l10n.walletSendRecipientLabel,
                        hint: l10n.walletSendRecipientHint,
                      ).copyWith(
                        suffixIcon: _recipientLocked
                            ? WalletIcon(
                                WalletGlyph.locked,
                                color: colors.textMuted,
                              )
                            : null,
                        helperText: _recipientLocked
                            ? l10n.walletSendRecipientLocked
                            : null,
                      ),
                ),
              ),
              // S13 §1.2 — Paste and Scan. Never drawn where they would override
              // what a host asked for (§1a H3): a locked recipient, an attached
              // machine memo. No Scan where the camera reader is unsupported.
              if (_scanAndPasteAllowed)
                Wrap(
                  spacing: 8,
                  children: [
                    TextButton.icon(
                      key: const ValueKey('send-paste'),
                      icon: const WalletIcon(WalletGlyph.paste),
                      label: Text(l10n.walletSendPaste),
                      onPressed: () => unawaited(_paste()),
                    ),
                    if (ref.watch(addressScannerSupportedProvider))
                      TextButton.icon(
                        key: const ValueKey('send-scan'),
                        icon: const WalletIcon(WalletGlyph.scanQr),
                        label: Text(l10n.walletSendScanQr),
                        onPressed: () => unawaited(_scan()),
                      ),
                  ],
                ),
              if (_scanFault)
                Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: WalletNotice(
                    key: const ValueKey('send-scan-fault'),
                    tone: WalletNoticeTone.warning,
                    glyph: WalletGlyph.error,
                    message: l10n.walletSendFaultUriInvalid,
                  ),
                ),
              // Live recipient feedback — ALWAYS rendered (reserved height) so a
              // valid/invalid/transparent verdict never shifts the fields below.
              _RecipientStatusLine(status: _recipientStatus),
              const SizedBox(height: 8),
              TextField(
                controller: _amountController,
                keyboardType: const TextInputType.numberWithOptions(
                  decimal: true,
                ),
                // Money is integer-exact: digits and one separator (a comma reads
                // as the point) at the input layer, and size-cap before the value
                // reaches the FFI (design invariant 7 — the parser is the real
                // gate). 20 chars > the longest valid amount "21000000.00000000".
                inputFormatters: const [
                  WalletDecimalInputFormatter(maxLength: 20),
                ],
                decoration: _sendDecoration(
                  context,
                  label: l10n.walletSendAmountLabel,
                  hint: l10n.walletSendAmountHint,
                ),
                // A new amount is a new payment: the Queue acknowledgement was
                // given for the old one (diff review, LOW).
                onChanged: (_) {
                  if (_queuePublicAcknowledged) {
                    setState(() => _queuePublicAcknowledged = false);
                  }
                },
              ),
              const SizedBox(height: 16),
              // #401 R3a — the one thing this screen genuinely lacked: the memo
              // field is auto-gated OFF for a transparent recipient, and the WHY
              // lived only in a sibling Text below it. Flutter's native disabled
              // flag is silent on TalkBack/VoiceOver, so a screen-reader user met a
              // field that refuses input for no stated reason. Exactly the argument
              // that already put the locked note on the recipient field, and the
              // same mechanism: the ancestor label is PREPENDED to the decoration
              // label on the field's own node (measured), so this reads
              // "<reason>, Memo" without repeating anything.
              Semantics(
                // FR-28: the machine-memo reason takes precedence — when the
                // host owns the memo slot that is WHY the field refuses input,
                // and the transparent-recipient sentence would be a different,
                // false explanation.
                label: _machineMemo != null
                    ? l10n.walletSendMemoMachineDisabled
                    : (_recipientStatus.allowsMemo
                          ? null
                          : l10n.walletSendMemoTransparentDisabled),
                child: TextField(
                  controller: _memoController,
                  maxLines: 2,
                  // Auto-gated: only a SHIELDED recipient can receive a ZIP-302
                  // memo. For a transparent recipient the field is disabled and a
                  // note explains WHY (honest degradation, invariant 6) — the
                  // controller's text is also dropped at compose (see [_memoToSend]),
                  // so a memo typed before a transparent paste is never silently
                  // sent nor silently lost.
                  // FR-28: a payment carries ONE memo. When the host attached
                  // machine bytes the written field is disabled rather than
                  // left open to collect text the encoder would then have to
                  // refuse — the user is told why, right below.
                  enabled: _recipientStatus.allowsMemo && _machineMemo == null,
                  // Boundary cap (design invariant 7 — size-cap before the round-
                  // trip): a ZIP-302 memo is ≤512 BYTES; cap the field generously
                  // in characters so a multi-megabyte paste can't cross the FFI on
                  // every Review tap. `maxLength` installs a length-limiting
                  // formatter internally; the SDK is still the precise byte-level
                  // gate (typed `MemoTooLong`).
                  maxLength: 512,
                  decoration: _sendDecoration(
                    context,
                    label: l10n.walletSendMemoLabel,
                    hint: l10n.walletSendMemoHint,
                  ),
                ),
              ),
              // FR-28 — the disclosure ALSO lives here, and this is not
              // belt-and-braces. The offline QUEUE commits a spend straight
              // from this form: it never enters the review, so the review-only
              // disclosure left the queue path attaching opaque bytes to a
              // permanent ledger with nothing shown at all — and the default
              // authorizer is a passthrough that prompts nothing (crypto
              // audit). Whichever button the user reaches for, they have read
              // this before it commits.
              if (_machineMemo case final memo?) ...[
                const SizedBox(height: 16),
                _MachineMemoDisclosure(purpose: memo.purpose),
              ],
              if (_machineMemo != null || !_recipientStatus.allowsMemo)
                Padding(
                  padding: const EdgeInsets.only(top: 4),
                  // EXCLUDED FROM SEMANTICS (#403 R8b): the ancestor label above
                  // now speaks this exact sentence as part of the FIELD's own
                  // node, so leaving the visible copy readable too makes a screen
                  // reader say the same reason twice while walking one control.
                  // The visual note is unchanged for sighted users — this removes
                  // only the duplicate announcement.
                  child: ExcludeSemantics(
                    child: Text(
                      _machineMemo != null
                          ? l10n.walletSendMemoMachineDisabled
                          : l10n.walletSendMemoTransparentDisabled,
                      style: textTheme.bodySmall?.copyWith(
                        color: colors.textMuted,
                      ),
                    ),
                  ),
                ),
              if (fault != null) ...[
                const SizedBox(height: 16),
                // queueOffered mirrors the capability seam: when the queue
                // affordance is hidden (#327), the not-synced copy must not
                // invite it (review H1).
                SendFormFaultView(
                  key: _faultKey,
                  fault: fault,
                  queueOffered: queueSupported,
                ),
              ],
              // A queued payment never passes the review screen, so for a public
              // recipient the Queue path shows the SAME warning card the review
              // does, and takes the acknowledgement here. It sits in the
              // scrolling list, not the pinned bar, so at large text it can
              // never crowd the Queue button out (diff review, two LOWs).
              if (showQueue && _recipientStatus is RecipientTransparent) ...[
                const SizedBox(height: 16),
                const DeshieldWarning(),
                const SizedBox(height: 8),
                WalletAckCheckbox(
                  key: const ValueKey('send-queue-public-ack'),
                  value: _queuePublicAcknowledged,
                  onChanged: (v) =>
                      setState(() => _queuePublicAcknowledged = v),
                  label: l10n.walletSendPublicAckLabel,
                ),
              ],
            ],
          ),
        ),
        // Pinned action bar — the primary action holds its place when a fault
        // renders above (no jump) and stays above the keyboard.
        Container(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
          decoration: BoxDecoration(
            color: colors.bg,
            border: Border(top: BorderSide(color: colors.border)),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              WalletCta(
                child: FilledButton(
                  // Foolproof: disabled until the recipient is a valid,
                  // sendable address (the status line explains an empty /
                  // invalid / wrong-network recipient). The amount's own faults
                  // stay inline post-tap; the SDK remains the binding gate.
                  onPressed: _recipientStatus.isSendable ? _review : null,
                  child: Text(l10n.walletSendReviewButton),
                ),
              ),
              if (showQueue) ...[
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: WalletCta(
                        child: OutlinedButton(
                          // Same recipient gate — queuing a send to an invalid
                          // address is never useful — plus the public-payment
                          // acknowledgement.
                          onPressed: _queueAllowed ? _queue : null,
                          child: Text(l10n.walletSendQueueButton),
                        ),
                      ),
                    ),
                    // The hint is an explanation, not a state: behind the (i)
                    // after the action it explains (S13 §1.2, DESIGN §6.20).
                    WalletInfoButton(
                      key: const ValueKey('send-queue-info'),
                      label: l10n.walletSendQueueButton,
                      body: l10n.walletSendQueueHint,
                    ),
                  ],
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }

  /// Recompute the live recipient classification on each keystroke/paste. Local
  /// + synchronous (no network) — feedback even fully offline. A null session is
  /// a defensive race (the screen re-renders unavailable); fall back to neutral.
  void _onRecipientChanged(String value) {
    final session = ref.read(walletSessionProvider);
    setState(() {
      _scanFault = false;
      _recipientStatus = session == null
          ? const RecipientEmpty()
          : classifyRecipient(session, value);
    });
  }

  /// §1a H3: Paste and Scan never override a host's request — not over a
  /// locked recipient, not beside a host's machine memo, not on an expired
  /// request. Re-read after the camera closes: the draft can change under it.
  bool get _scanAndPasteAllowed =>
      !_recipientLocked && _machineMemo == null && !_expired;

  /// The longest scanned text this screen will parse (§1a LOW): far above any
  /// single-payment `zcash:` URI, far below anything that would cost the
  /// parser real work.
  static const int _maxScannedChars = 2048;

  /// The recipient and memo fields' own 512 caps (the address field's
  /// formatter, the memo's `maxLength`), applied to a programmatic set, which
  /// bypasses both. The SDK's byte gates stay the authoritative backstop.
  static const int _maxFieldChars = 512;

  static String _capped(String s) =>
      s.length > _maxFieldChars ? s.substring(0, _maxFieldChars) : s;

  /// Paste the clipboard into the recipient and classify it, as typing would.
  Future<void> _paste() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    if (!mounted || !_scanAndPasteAllowed) return;
    final text = data?.text?.trim();
    if (text == null || text.isEmpty) return;
    final capped = _capped(text);
    _addressController.text = capped;
    _onRecipientChanged(capped);
  }

  /// Scan a QR code into the draft. A `zcash:` URI goes through the SDK's own
  /// parser (the FR-25 decode: this wallet's network, one payment, a text
  /// memo) and REPLACES the draft; a bare address fills the recipient only.
  /// Anything that cannot be used shows one inline fault and changes nothing.
  /// The payload is never logged (§5.4), and a URI's label is never shown.
  Future<void> _scan() async {
    final raw = await ref.read(addressScannerProvider)(context);
    if (!mounted || raw == null || !_scanAndPasteAllowed) return;
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    final text = raw.trim();
    final request = _requestFromScan(session, text);
    if (request != null) {
      _applyScanned(request);
      return;
    }
    final address = _capped(text);
    final status = text.length > _maxScannedChars || _isPaymentUri(text)
        ? const RecipientInvalid()
        : classifyRecipient(session, address);
    if (status is RecipientEmpty || status is RecipientInvalid) {
      setState(() => _scanFault = true);
      return;
    }
    _addressController.text = address;
    setState(() {
      _scanFault = false;
      _recipientStatus = status;
    });
  }

  static bool _isPaymentUri(String text) =>
      text.toLowerCase().startsWith('zcash:');

  /// The request a scanned `zcash:` URI carries, or null when [text] is not
  /// a URI this form can use (not a URI, over the cap, or refused by the
  /// parser for any reason).
  WalletSendRequest? _requestFromScan(WalletSession session, String text) {
    if (!_isPaymentUri(text) || text.length > _maxScannedChars) return null;
    try {
      return session.parseSendRequest(text);
    } catch (_) {
      return null;
    }
  }

  /// §1a H3: a scanned request replaces the WHOLE draft — address, amount and
  /// memo — never a merge with what was typed, so a payment is never half one
  /// request and half another. The recipient status goes through its setter,
  /// so the Queue acknowledgement given for the old draft is withdrawn.
  void _applyScanned(WalletSendRequest request) {
    final session = ref.read(walletSessionProvider);
    if (session == null) return;
    final address = _capped(request.address);
    _addressController.text = address;
    final amountZat = request.amountZat;
    _amountController.text = amountZat != null && amountZat > 0
        ? formatZec(amountZat)
        : '';
    final memo = request.memo;
    _memoController.text = memo == null ? '' : _capped(memo);
    setState(() {
      _scanFault = false;
      _recipientStatus = classifyRecipient(session, address);
    });
  }

  /// The memo to forward — DROPPED for a transparent recipient. The memo field
  /// is disabled in that state but its controller may still hold text typed
  /// before a transparent address was pasted; forwarding it would trip the SDK's
  /// typed `memoToTransparent` rejection. Gating here keeps compose honest
  /// without destroying the user's draft (it returns if they pick a shielded
  /// recipient again).
  /// FR-28: the host's machine memo is forwarded on EVERY compose, read from
  /// screen state and never re-derived from a field — that is the whole fix.
  /// The form re-composes its URI from its own text fields on each Review or
  /// Queue tap, so bytes that lived only in the original request were dropped
  /// the moment the user edited the amount: no error, no log, a silent no-op
  /// that read as success.
  String? get _memoToSend => _machineMemo != null
      // FR-28: one memo per payment. A machine memo owns the slot, and the
      // text field is disabled while it does — so there is no typed text to
      // forward, and nothing the encoder would have to refuse.
      ? null
      : (_recipientStatus.allowsMemo ? _memoController.text : null);

  void _review() {
    _driveCurrentFlow();
    unawaited(
      ref
          .read(sendControllerProvider.notifier)
          .prepare(
            address: _addressController.text,
            amountText: _amountController.text,
            memo: _memoToSend,
            machineMemo: _machineMemo,
          ),
    );
  }

  void _queue() {
    _driveCurrentFlow();
    unawaited(
      ref
          .read(sendControllerProvider.notifier)
          .queueOffline(
            address: _addressController.text,
            amountText: _amountController.text,
            memo: _memoToSend,
            machineMemo: _machineMemo,
          ),
    );
  }

  /// Set by a session flip: this screen's host request belonged to the
  /// identity that was active when it arrived, so a later flow is never
  /// re-claimed for it (the flip already dropped the request's draft).
  bool _requestIdentityGone = false;

  /// S13 §1a H1 (the diff review's two MEDIUMs): a Review or Queue tap
  /// is this screen driving the controller's CURRENT flow — record it for the
  /// teardown abandon. The form (the only place these taps exist) renders
  /// only in `SendForm`, so nothing is in flight here.
  ///
  /// And if the host's channel is still waiting but its claim sits on an
  /// OLDER flow (an abandoned-flow restart minted a new one under this
  /// screen), the claim moves to the flow the user is actually about to pay
  /// through, and the superseded one is abandoned. One host request, one
  /// live flow: otherwise the host would hear "no transaction" for the old
  /// id while this tap paid under the new one.
  void _driveCurrentFlow() {
    var flowId = _sendController.flowId;
    // Not a flow THIS screen started (it was revealed under a screen that
    // left, it shares the controller with another live screen, or a flip or
    // an abandoned-flow restart minted the id under it): start its own, so a
    // tap never pays under — or later abandons — a flow another screen owns
    // (the fold review's LOW). The state is SendForm here, so the reset
    // takes; the form's text is the screen's and survives it.
    if (!_drivenFlows.contains(flowId)) {
      _sendController.resetToForm(requestRevoked: _expired);
      flowId = _sendController.flowId;
    }
    _drivenFlows.add(flowId);
    final reporter = widget.reporter;
    if (reporter == null ||
        reporter.isReported ||
        _requestIdentityGone ||
        _owed.containsKey(flowId)) {
      return;
    }
    final superseded = [
      for (final e in _owed.entries)
        if (identical(e.value, reporter) && !_enteredSpend(e.key)) e.key,
    ];
    if (superseded.isEmpty) return;
    for (final old in superseded) {
      _sendController.abandonFlow(old);
      _owed.remove(old);
    }
    _owed[flowId] = reporter;
  }
}

/// Shared input decoration for the send form. The hint uses the MUTED token so
/// the placeholder reads as light-gray guidance — not near-black text that looks
/// like an entered value (the maintainer report on the light theme). One builder so
/// all three fields stay visually consistent.
///
/// **`labelText` IS the accessible name, and #401 R3a first claimed otherwise.**
/// The device walk reported all three fields unnamed; that was an artifact of
/// the dump, not of this screen. MEASURED afterwards against the real semantics
/// tree: the `isTextField` node carries `labelText` as its label, and an ancestor
/// `Semantics(label:)` is PREPENDED to it rather than replacing it. `uiautomator
/// dump` cannot show it either way — Flutter routes a text field's label to
/// `AccessibilityNodeInfo` hint text, and the dump emits `text` / `content-desc`
/// and no hint at all. So the earlier "no accessible name" reading was the same
/// class of mistake as grepping `text=` for a `content-desc` label: absence in the
/// dump is not absence in the tree.
///
/// The ancestor channel is therefore reserved for STATE sentences that have no
/// other way to reach the field (the locked recipient, the memo's disabled
/// reason). Adding the field's own name there would announce it twice.
InputDecoration _sendDecoration(
  BuildContext context, {
  required String label,
  required String hint,
}) {
  final colors = WalletColors.of(context);
  return InputDecoration(
    labelText: label,
    hintText: hint,
    hintStyle: TextStyle(color: colors.textMuted),
  );
}

/// The live recipient feedback line — ALWAYS rendered at a consistent height so
/// a verdict appearing/changing never shifts the fields below it (the maintainer's
/// no-layout-shift bar; the empty state reserves a blank single line). Shows the
/// §5.1 privacy axis ("Shielded · private" / "Transparent · public") for a valid
/// recipient and an honest, plain-language line for an invalid / wrong-network
/// one — never a raw code (invariant 6). A screen-reader live region so the
/// verdict is announced as the user types/pastes.
class _RecipientStatusLine extends StatelessWidget {
  const _RecipientStatusLine({required this.status});

  final RecipientStatus status;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    // (icon, tint, message) per state. Shielded reads as good (green); a
    // transparent recipient is allowed but PUBLIC (orange heads-up, matching the
    // Review-screen de-shield treatment); invalid / wrong-network are honest
    // warnings (orange — red stays reserved for a money failure, per the palette).
    final WalletGlyph? icon;
    final Color tint;
    final String message;
    switch (status) {
      case RecipientEmpty():
        icon = null;
        tint = colors.textMuted;
        message = '';
      case RecipientShielded():
        icon = WalletGlyph.shielded;
        tint = colors.green;
        message = l10n.walletSendRecipientShielded;
      case RecipientTransparent():
        icon = WalletGlyph.transparent;
        tint = colors.orange;
        message = l10n.walletSendRecipientTransparent;
      case RecipientInvalid():
        icon = WalletGlyph.error;
        tint = colors.orange;
        message = l10n.walletSendRecipientInvalid;
      case RecipientWrongNetwork():
        icon = WalletGlyph.error;
        tint = colors.orange;
        message = l10n.walletSendRecipientWrongNetwork;
    }

    return Padding(
      key: const Key('send-recipient-status'),
      padding: const EdgeInsets.only(top: 8),
      child: Semantics(
        liveRegion: message.isNotEmpty,
        label: message.isEmpty ? null : message,
        excludeSemantics: true,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Fixed 18×18 leading slot (transparent when empty) so the row's
            // leading width is identical across states — no horizontal jump.
            SizedBox(
              width: 18,
              height: 18,
              child: icon == null
                  ? null
                  : WalletIcon(icon, size: 18, color: tint),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                message,
                style: textTheme.bodySmall?.copyWith(color: tint),
                maxLines: 2,
                // Bound the line growth (a long verdict at extreme text scale
                // ellipsizes rather than silently clipping or pushing the
                // scrollable fields below by more than one line; the pinned
                // action bar never shifts regardless).
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Honest "no live wallet" state (defensive — the screen is gated to an active
/// wallet; a wallet close mid-screen lands here, never a stale send form).
class _SendUnavailable extends StatelessWidget {
  const _SendUnavailable();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              l10n.walletSendUnavailable,
              textAlign: TextAlign.center,
              style: Theme.of(
                context,
              ).textTheme.bodyMedium?.copyWith(color: colors.textMuted),
            ),
            const SizedBox(height: 16),
            OutlinedButton(
              onPressed: () => context.leaveToWalletRoot(),
              child: Text(l10n.walletSendDone),
            ),
          ],
        ),
      ),
    );
  }
}

/// Honest "this wallet can't send" state (#397 §3.7 D3) — a watch-only wallet
/// holds a viewing key but no spending keys, so it can never sign a send. The
/// wallet-screen chrome hides Send, so this is reached only via a deep-link /
/// prefilled entry straight to /wallet/send; it re-gates on the kind so a
/// watch-only wallet never sees a send form that would fault on propose. Distinct
/// copy from [_SendUnavailable] (which is a transient "no live wallet"): this is
/// the PERMANENT fact of the wallet's custody, so it states it plainly.
class _SendWatchOnly extends StatelessWidget {
  const _SendWatchOnly();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            WalletIcon(
              WalletGlyph.watchOnly,
              size: 40,
              color: colors.textMuted,
            ),
            const SizedBox(height: 16),
            Text(
              l10n.walletSendWatchOnly,
              textAlign: TextAlign.center,
              style: Theme.of(
                context,
              ).textTheme.bodyMedium?.copyWith(color: colors.textMuted),
            ),
            const SizedBox(height: 16),
            OutlinedButton(
              onPressed: () => context.leaveToWalletRoot(),
              child: Text(l10n.walletSendDone),
            ),
          ],
        ),
      ),
    );
  }
}

/// Honest "this request expired" state (stage S8 `deadline`, R05): the entry's
/// mount grace ran out before this screen appeared, the host was told "no
/// transaction", and that answer is final for the request — so there is no
/// form, no Review, no Queue, and nothing here can pay it. The copy names the
/// limit the user hit (a phone that took more than five seconds to open the
/// send screen) and the one next step: start again from the app. Distinct
/// from [_SendUnavailable] (a transient "no live wallet") and the result views
/// (a flow that ran): nothing ran, and nothing will. A live region, like the
/// result views — it replaces a screen the user was waiting for, and a
/// non-sighted user must not be left hunting for whether the payment went out.
class _SendRequestExpired extends StatelessWidget {
  const _SendRequestExpired();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Semantics(
          container: true,
          liveRegion: true,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              WalletIcon(
                WalletGlyph.timerExpired,
                size: 48,
                color: colors.textMuted,
              ),
              const SizedBox(height: 16),
              Text(
                l10n.walletSendExpiredTitle,
                style: textTheme.headlineSmall,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 12),
              Text(
                l10n.walletSendExpiredBody,
                style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 24),
              OutlinedButton(
                onPressed: () => context.leaveToWalletRoot(),
                child: Text(l10n.walletSendDone),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// The honest "we lost the answer" terminal (S7 U1, [SendOutcomeUnknown]): the
/// spend ran and something after it threw or declined, so this screen can say
/// neither "sent" nor "nothing was sent". It points at where the truth is and
/// offers Done only — a Try again here is how one payment gets made twice.
/// Never renders an error's text. A live region, like the result views — the
/// shared [WalletOutcomeUnknownView] the shield and move sheets use too.
class _SendOutcomeUnknownView extends ConsumerWidget {
  const _SendOutcomeUnknownView({required this.queued});

  final bool queued;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: WalletOutcomeUnknownView(
          title: l10n.walletSendUnknownTitle,
          body: queued
              ? l10n.walletSendUnknownQueuedBody
              : l10n.walletSendUnknownBody,
          closeLabel: l10n.walletSendDone,
          onClose: () => _done(context, ref),
        ),
      ),
    );
  }
}

/// A neutral busy view for the transient phases (preparing / submitting /
/// queuing). One coherent screen-reader node.
class _SendBusy extends StatelessWidget {
  const _SendBusy({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Center(
      child: Semantics(
        container: true,
        excludeSemantics: true,
        // #401 R3b, the same family as the parked Send now: a busy view that a
        // screen reader never announces is a silent app for the length of an
        // unbounded prove — and the user's response to a silent money screen is
        // to go back and try again.
        liveRegion: true,
        label: label,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const CircularProgressIndicator.adaptive(),
            const SizedBox(height: 16),
            Text(
              label,
              style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}

/// The confirm screen — the numbers the user approves before any money moves,
/// plus the §5.1 de-shield disclosure (the privacy-honesty centerpiece: a
/// transparent recipient means the amount + address are PUBLIC on-chain).
class _SendReviewView extends ConsumerStatefulWidget {
  const _SendReviewView({
    super.key,
    required this.proposal,
    required this.recipient,
    this.machineMemoPurpose,
  });

  final SendProposal proposal;
  final String recipient;

  /// FR-28: the purpose of an attached machine memo, disclosed HERE — the
  /// authorization step, the moment the user is already reading and can still
  /// stop. Not a separate screen, not dismissible, not remembered: a "don't
  /// show again" would return it to a silent attachment after one tap.
  ///
  /// Read from the REVIEW STATE, not from the screen's own draft: the sentence
  /// the user reads and the sentence the host's prompt receives must be one
  /// value, or they can drift.
  final String? machineMemoPurpose;

  @override
  ConsumerState<_SendReviewView> createState() => _SendReviewViewState();
}

class _SendReviewViewState extends ConsumerState<_SendReviewView> {
  /// The public-payment acknowledgement ("add the Send
  /// acknowledgement like Swap"). Owned HERE and keyed by the proposal at the
  /// construction site, so a new proposal — a different recipient or amount
  /// after Back — always starts unticked, like Swap's box on a new quote.
  bool _publicAcknowledged = false;

  SendProposal get proposal => widget.proposal;
  String get recipient => widget.recipient;
  String? get machineMemoPurpose => widget.machineMemoPurpose;

  /// Send now waits on the acknowledgement only when something public is at
  /// stake; an ordinary shielded payment carries no extra tap.
  bool get _confirmAllowed =>
      !proposal.hasTransparentRecipient || _publicAcknowledged;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(l10n.walletSendReviewTitle, style: textTheme.headlineSmall),
        const SizedBox(height: 16),

        // The de-shield disclosure FIRST (most load-bearing) when any output is
        // transparent — never let the headline number bury the privacy loss.
        if (proposal.hasTransparentRecipient) ...[
          const DeshieldWarning(),
          const SizedBox(height: 8),
          // Directly under the warning it acknowledges, as Swap's sits under
          // its disclosure. Send now stays disabled until it is ticked.
          WalletAckCheckbox(
            key: const ValueKey('send-public-ack'),
            value: _publicAcknowledged,
            onChanged: (v) => setState(() => _publicAcknowledged = v),
            label: l10n.walletSendPublicAckLabel,
          ),
          const SizedBox(height: 8),
        ],

        // §3.2i-3 (c): the review STATES the payment's visibility class in
        // BOTH cases — the shielded arm is the informative one (today every
        // send draws from the shielded pool; the recipient address decides
        // what becomes public), the transparent arm restates the warning
        // above in one compact line so the fact survives a skimmed review.
        _PrivacyStatement(transparent: proposal.hasTransparentRecipient),
        const SizedBox(height: 16),

        // FR-28 — the machine-memo disclosure, above the numbers because it is
        // a fact about WHAT IS BEING SIGNED, not a footnote to it.
        if (machineMemoPurpose case final purpose?) ...[
          _MachineMemoDisclosure(purpose: purpose),
          const SizedBox(height: 16),
        ],

        // Who — echoed back so the user verifies the recipient they typed. A
        // group: fill, no outline (S13 §1.2).
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: colors.bgCard,
            borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                l10n.walletSendRecipientLabel,
                style: textTheme.labelMedium?.copyWith(color: colors.textMuted),
              ),
              const SizedBox(height: 4),
              // Foolproof: the recipient (the user's own typed input) is shown in
              // equal-weight monospace GROUPS so it can be verified glyph-by-glyph
              // before any money moves — the shared verification brick (the same
              // one the swap refund echo uses).
              AddressVerificationText(recipient),
              // Passive money-safety cue: paying your OWN wallet (money-safe, but
              // usually unintended). Never a gate — an understated note only.
              if (proposal.selfSend) ...[
                const SizedBox(height: 12),
                const _SelfSendNote(),
              ],
            ],
          ),
        ),
        const SizedBox(height: 16),

        // The numbers — integer zatoshis formatted exactly (never via double).
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: colors.bgCard,
            borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
          ),
          child: Column(
            children: [
              // S13 §1.2 + §1a M2 — what the recipient gets: FR-46's own figure
              // (fee and change excluded, a TEX two-step counted once), never a
              // re-derivation. No line when it is null (more than one
              // recipient). Hide balance never masks it: this is the payment
              // being approved, not the balance (S12 R1).
              if (proposal.singleRecipientZat case final gets?) ...[
                LabeledZatRow(
                  key: const ValueKey('send-review-recipient-gets'),
                  label: l10n.walletSendRecipientGetsLabel,
                  amountZat: gets,
                ),
                const SizedBox(height: 8),
              ],
              LabeledZatRow(
                label: l10n.walletSendTotalLabel,
                amountZat: proposal.totalZat,
                emphasize: true,
              ),
              const SizedBox(height: 8),
              LabeledZatRow(
                label: l10n.walletSendFeeLabel,
                amountZat: proposal.feeZat,
              ),
              // Change is informational (already net out of total); shown when
              // nonzero so the figures reconcile.
              if (proposal.changeZat > 0) ...[
                const SizedBox(height: 8),
                LabeledZatRow(
                  label: l10n.walletSendChangeLabel,
                  amountZat: proposal.changeZat,
                ),
              ],
            ],
          ),
        ),
        const SizedBox(height: 24),

        WalletCta(
          child: FilledButton(
            onPressed: _confirmAllowed
                ? () => unawaited(_confirm(context, ref))
                : null,
            child: Text(l10n.walletSendConfirmButton),
          ),
        ),
        const SizedBox(height: 8),
        TextButton(
          onPressed: () =>
              ref.read(sendControllerProvider.notifier).backToForm(),
          child: Text(l10n.walletSendBackButton),
        ),
      ],
    );
  }

  /// The confirm action. For an ORDINARY send it signs immediately — the review
  /// screen IS the single deliberate confirm, so a normal payment never carries
  /// extra friction (no alert fatigue). For a LARGE send ([proposal.largeSend] is
  /// set) it inserts EXACTLY ONE deliberate confirmation dialog first: the rare,
  /// money-critical case earns one unmistakable "are you sure", keyed off the
  /// reason and carrying the exact amount on the irreversible button.
  ///
  /// Money-safety over alert-fatigue: the dialog fires whenever the SDK set
  /// `largeSend`, NEVER suppressed on sync/settle state. Over-firing (a mid-scan
  /// in-flight-change send transiently reading near-total) is the SAFE direction —
  /// one extra tap — whereas suppressing on an "unsettled" guess could hide a
  /// genuine near-total send; that over-fire edge is already mitigated upstream by
  /// the spend-before-sync gating (#228), so we never trade a real warning for it.
  Future<void> _confirm(BuildContext context, WidgetRef ref) async {
    // Capture the notifier BEFORE the await so nothing touches `context`/`ref`
    // across the async gap (the modal dialog) — lint-clean and dispose-safe; the
    // controller's own `_disposed`/`_requireSession` guards backstop a late tap.
    final notifier = ref.read(sendControllerProvider.notifier);
    final reason = proposal.largeSend;
    if (reason != null) {
      final approved = await showLargeSendConfirm(
        context,
        totalZat: proposal.totalZat,
        reason: reason,
      );
      if (approved != true) return; // cancelled / dismissed — stay on review
    }
    // A light tap under the thumb for the money confirm (S13 §1.7).
    unawaited(HapticFeedback.lightImpact());
    unawaited(notifier.confirm());
  }
}

/// A passive "you're paying yourself" note (inc-2d money-safety). A self-send is
/// money-SAFE — just usually unintended (a mis-paste of your own address, or a
/// deliberate consolidation) — so this is an understated INFO cue: muted, never a
/// gate, and never the orange warning treatment (reserved for the de-shield
/// privacy loss). Best-effort: the SDK detects only the exact own unified address
/// (see [SendProposal.selfSend]), so its ABSENCE is not a guarantee.
class _SelfSendNote extends StatelessWidget {
  const _SelfSendNote();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Semantics(
      container: true,
      excludeSemantics: true,
      label: l10n.walletSendSelfSendNote,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          WalletIcon(WalletGlyph.wallet, size: 18, color: colors.textMuted),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              l10n.walletSendSelfSendNote,
              style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
            ),
          ),
        ],
      ),
    );
  }
}

/// FR-28 — the machine-memo disclosure: the app is attaching opaque bytes to
/// this transaction, and here is the sentence it gave for why.
///
/// THREE THINGS IT DELIBERATELY DOES NOT DO.
/// - It never renders the BYTES. Hex is meaningless to a user and alarming in
///   the wrong direction; presence and purpose are the whole payload.
/// - It is not dismissible and not remembered. Per send, every send.
/// - It does not vouch for the sentence. The wallet cannot check that the
///   purpose describes the bytes, and a hostile host can lie in it — so the
///   copy says the wallet can't check, rather than implying it did. A label is
///   ACCOUNTABILITY, not verification, and overstating that would be worse than
///   showing nothing.
///
/// It renders as a container with its own semantics node so a screen reader
/// reads the three lines as one disclosure while walking the review, rather
/// than as three unrelated fragments between the privacy line and the numbers.
class _MachineMemoDisclosure extends StatelessWidget {
  const _MachineMemoDisclosure({required this.purpose});

  /// The host's own sentence. Attacker-controllable text on a money surface.
  final String purpose;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Semantics(
      container: true,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: colors.bgCard,
          borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
          border: Border.all(color: colors.border),
        ),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            WalletIcon(WalletGlyph.label, size: 18, color: colors.textMuted),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    l10n.walletSendMachineMemoTitle,
                    style: textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: 4),
                  // The host's own sentence, verbatim. It is attacker-
                  // controllable text on a money surface, so it is rendered as
                  // plain body copy — never as a link, never as markup, and
                  // never styled to look like the wallet's own voice.
                  Text(
                    l10n.walletSendMachineMemoPurpose(purpose),
                    style: textTheme.bodySmall,
                    // Bounded HEIGHT as well as length. The constructor already
                    // flattens the string to one line, so this is the second
                    // half of the same guarantee: a host cannot grow the review
                    // and push Confirm off the page, at any text scale.
                    maxLines: 3,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    l10n.walletSendMachineMemoLimit,
                    style: textTheme.bodySmall?.copyWith(
                      color: colors.textMuted,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// The review's explicit visibility statement (§3.2i-3 (c)): one compact line
/// naming the payment's privacy class. The shielded arm gets the calm lock
/// (the informative case — "this stays private"); the transparent arm gets the
/// orange public glyph and orange words, restating the DeshieldWarning above in
/// one skimmable line. Plain-factual copy (maintainer call).
class _PrivacyStatement extends StatelessWidget {
  const _PrivacyStatement({required this.transparent});

  final bool transparent;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final text = transparent
        ? l10n.walletSendPrivacyTransparent
        : l10n.walletSendPrivacyShielded;
    // A notice line: the transparent arm keeps its words in the warning tone
    // (they ARE the warning); the shielded arm is a plain positive line.
    return Semantics(
      container: true,
      excludeSemantics: true,
      label: text,
      child: WalletNotice(
        key: const ValueKey('send-privacy-statement'),
        tone: transparent
            ? WalletNoticeTone.warning
            : WalletNoticeTone.positive,
        glyph: transparent ? WalletGlyph.transparent : WalletGlyph.locked,
        message: text,
        messageInTone: transparent,
      ),
    );
  }
}

/// The terminal result of a `send` — honest per [SendOutcome]: a broadcast
/// failure is "saved, we'll finish sending", never a lost-money error.
class _SendResultView extends ConsumerWidget {
  const _SendResultView({
    required this.outcome,
    this.hostReported = false,
    this.machineMemoAttached = false,
  });

  final SendOutcome outcome;

  /// FR-26: a host has already been told what became of this flow, and the
  /// channel is one-shot. A SECOND payment started from here would be one the
  /// host's record can never learn about — the user believes the host saw both
  /// — so the entry closes at its first terminal and offers Done only. The user
  /// can still start a fresh send from the wallet's own Send button, where no
  /// host is waiting on a report.
  final bool hostReported;

  /// FR-28: this entry carries a host machine memo, so the form's fields are
  /// still holding an envelope the host attached to ONE payment.
  final bool machineMemoAttached;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);

    final WalletGlyph icon;
    final Color tint;
    final String title;
    final String body;
    // signFailed is the only "didn't move money, try again" path; the rest are
    // success-shaped (sent / saved-for-retry / already-submitted).
    // "Try again" is suppressed for the same reason "Send another" is: the
    // report channel is one-shot, so a payment made from a retry here is one
    // the host's record can never learn about. The security review
    // reproduced exactly that — sign-failed reported, user taps Try again, the
    // retry pays, and the host still holds "nothing was created".
    var offerTryAgain = false;
    // "Send another" is offered after every success-shaped outcome EXCEPT the TEX
    // in-motion arm: that arm's body explicitly says "don't send it again" while
    // funds are mid-flight on the ephemeral, so a re-send affordance right beneath
    // it would contradict its own guidance (and tempt a confused user to re-pay
    // "to finish it"). The user can still start a fresh send via Done → Send.
    //
    // …and EXCEPT when a host has already been told the outcome (FR-26): the
    // form is still prefilled with the payee and amount, so "Send another" is a
    // two-tap repeat of the payment just made — and the one-shot report channel
    // means the host would never hear about the second one.
    //
    // …and EXCEPT when the entry carries a machine memo. The form keeps its
    // fields across "Send another", so the second payment would silently
    // re-attach the host's envelope to a different recipient — two on-chain
    // transactions bearing the same opaque correlator, linkable by anyone who
    // knows the format, from a host that asked for one payment (crypto
    // audit).
    var offerSendAnother = !hostReported && !machineMemoAttached;

    switch (outcome) {
      case SendSucceeded():
        icon = WalletGlyph.success;
        tint = colors.green;
        title = l10n.walletSendSentTitle;
        body = l10n.walletSendSentBody;
      case SendSavedForRetry(:final broadcast, :final total):
        // The same state the Shield and Move-to-transparent sheets draw as
        // savedForRetry: a host that restyles that glyph restyles all three
        // (S10 review MEDIUM — this site kept a pre-hook `cloud_off`).
        icon = WalletGlyph.savedForRetry;
        tint = colors.orange;
        title = l10n.walletSendSavedTitle;
        // BOTH bodies promise the wallet finishes this on a later sync. That
        // promise is custody-INDEPENDENT — the transaction is already SIGNED and
        // the §6.1 ReBroadcast arm re-sends the raw bytes with no seed — but it is
        // PASS-dependent, and `after_synced` is the only thing that drives it. With
        // no passes the money sits signed and unsent behind copy that says it is
        // handled, and the user re-enters the payment: qualify it (#401 R1b/R5).
        body = walletSyncPausedQualified(
          l10n,
          (broadcast > 0 && broadcast < total)
              ? l10n.walletSendPartialBody
              : l10n.walletSendSavedBody,
          syncPassesRun: ref.watch(walletSyncPassesRunProvider),
        );
      // The wallet kept the signed payment but did NOT report that it will retry
      // it (stage S8 `obligation`, row 10): "saved", no promise, and Activity
      // carries the live delivery state. Success-shaped (a transaction exists),
      // so no "try again" — a retry here would be a second payment.
      case SendKept():
        icon = WalletGlyph.saved;
        tint = colors.orange;
        title = l10n.walletSendKeptTitle;
        body = l10n.walletSendKeptBody;
      // A two-step TEX with some but not all legs accepted (either order — see
      // summarizeSendOutcome): funds are in motion on a wallet-controlled
      // one-time address. Its own honest
      // copy — NOT the saved-for-retry "the rest will complete on the next sync"
      // (which would over-promise auto-completion the moment tx1 expires into a
      // recoverable strand). Success-shaped (no "try again"); but it SUPPRESSES
      // "Send another" — the body says "don't send it again" while funds are
      // mid-flight, so the screen must not offer a re-send. The wallet-screen
      // recover surfaces own the rest (§3.2i-2).
      case SendTexInMotion():
        icon = WalletGlyph.inMotion;
        tint = colors.orange;
        title = l10n.walletSendInMotionTitle;
        body = l10n.walletSendInMotionBody;
        offerSendAnother = false;
      case SendAlreadySubmitted():
        icon = WalletGlyph.info;
        tint = colors.textMuted;
        title = l10n.walletSendAlreadyTitle;
        body = l10n.walletSendAlreadyBody;
      case SendSignFailed():
        icon = WalletGlyph.error;
        tint = colors.red;
        title = l10n.walletSendFailedTitle;
        body = l10n.walletSendFailedBody;
        offerTryAgain = true;
    }

    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        // liveRegion so a screen reader ANNOUNCES the verdict when the in-progress
        // view is replaced by it (#400 R9). This is the outcome of a money-moving
        // action and its three sibling result surfaces already announce; without it
        // a non-sighted user is left on a screen that silently changed under them
        // and has to go hunting for whether their payment went out.
        child: Semantics(
          container: true,
          liveRegion: true,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              WalletIcon(icon, size: 48, color: tint),
              const SizedBox(height: 16),
              Text(
                title,
                style: Theme.of(context).textTheme.headlineSmall,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 12),
              Text(
                body,
                style: Theme.of(
                  context,
                ).textTheme.bodyMedium?.copyWith(color: colors.textMuted),
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 24),
              if (offerTryAgain && !hostReported)
                FilledButton(
                  onPressed: () =>
                      ref.read(sendControllerProvider.notifier).backToForm(),
                  child: Text(l10n.walletSendTryAgain),
                )
              else ...[
                FilledButton(
                  onPressed: () => _done(context, ref),
                  child: Text(l10n.walletSendDone),
                ),
                if (offerSendAnother) ...[
                  const SizedBox(height: 8),
                  TextButton(
                    // "Send another" follows a money-COMMITTING outcome (sent /
                    // saved for retry / already-submitted) — the spent notes are
                    // already in the wallet DB, so refresh the available-balance
                    // hint before the next form, or a back-to-back send would be
                    // composed against a stale (too-high) figure. (The "Try again"
                    // path after a sign failure moved no money, so it stays on
                    // `backToForm` alone; the TEX in-motion arm suppresses this
                    // entirely — see above.)
                    onPressed: () {
                      ref.invalidate(walletSnapshotReadProvider);
                      // Keep the in-flight cue fresh too (#309) — the user may
                      // navigate to the wallet screen from the new form.
                      ref.invalidate(walletInFlightSendsReadProvider);
                      ref.read(sendControllerProvider.notifier).backToForm();
                    },
                    child: Text(l10n.walletSendAnother),
                  ),
                ],
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// The terminal result of an offline `queueSend` — the intent is durably stored
/// and sends on the next online sync (invariant 4 — "queued is a normal state").
class _SendQueuedView extends ConsumerWidget {
  const _SendQueuedView();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            WalletIcon(WalletGlyph.scheduled, size: 48, color: colors.cyan),
            const SizedBox(height: 16),
            Text(
              l10n.walletSendQueuedTitle,
              style: Theme.of(context).textTheme.headlineSmall,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 12),
            Text(
              l10n.walletSendQueuedBody,
              style: Theme.of(
                context,
              ).textTheme.bodyMedium?.copyWith(color: colors.textMuted),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: () => _done(context, ref),
              child: Text(l10n.walletSendDone),
            ),
          ],
        ),
      ),
    );
  }
}

/// Done after a completed/queued send: re-read the cold wallet state (the spent
/// notes / pending change land in the wallet DB immediately, so the balance is
/// honestly refreshed) and leave the send screen.
void _done(BuildContext context, WidgetRef ref) {
  ref.invalidate(walletSnapshotReadProvider);
  // Re-pull the in-flight two-step cue (#309) as the user lands back on the
  // wallet screen: after a PARTIAL two-step broadcast the durable "don't send
  // it again" note must be visible IMMEDIATELY (the next sync edge may be
  // seconds away — this window is the cue's whole reason to exist).
  ref.invalidate(walletInFlightSendsReadProvider);
  context.leaveToWalletRoot();
}
