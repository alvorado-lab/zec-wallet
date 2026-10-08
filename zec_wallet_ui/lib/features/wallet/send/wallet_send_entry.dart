import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../../core/router/wallet_routes.dart';
import '../../../src/send/wallet_send_channel.dart';
import 'send_controller.dart';
import 'wallet_send_report.dart';
import 'wallet_send_request.dart';

/// FR-25 — the public entry point that opens the package's send flow PREFILLED.
///
/// A host with its own address-discovery channel (a scanned QR, a `zcash:` deep
/// link, a messenger pay-a-contact, NFC) calls this instead of rebuilding a send
/// form: the ENTIRE money path — validation, review, ceiling, authorizer —
/// stays inside the audited package, and the host contributes only the entry
/// point. Prefilled ≠ pre-confirmed (see [WalletSendRequest]).
///
/// PRECONDITION — the send route is mounted (the host included `walletRoutes()`
/// in its GoRouter; the send flow is part of the wallet feature, so this holds
/// by construction), the calling `context` sits under the host's
/// `ProviderScope` (every wallet screen reads providers, so this too holds by
/// construction — [push] reads the send controller's revoked-request memory
/// through it, ADR-0558) AND a wallet is active. The send screen re-gates on a
/// live session itself, so a push with no wallet renders its honest "go back"
/// state (money-safety: the gate lives with the money surface, not the
/// navigation).
abstract final class WalletSendEntry {
  /// Push the send flow prefilled from [request], and REPORT what became of it
  /// (FR-26).
  ///
  /// The returned future completes exactly once, with the [WalletSendReport]
  /// for THIS request — a transaction exists (with its txids), nothing was
  /// created, it was queued offline, it was already submitted, or the seam
  /// cannot grade it. The host needs no history or activity read to learn the
  /// answer, and no join on recipient + amount: the future belongs to this
  /// push, and [WalletSendRequest.correlationId] rides through onto the report
  /// for a host that routes reports instead of awaiting them here.
  ///
  /// It completes when the send screen is GONE (its teardown, a frame or so
  /// after the pop) — not at the pop itself, so a payment that lands as the
  /// user is leaving is still reported as a payment. A report that says
  /// "unclassified" is not a failure; read its doc before mapping it to one.
  ///
  /// If the send screen has not appeared within a bounded grace (a host
  /// redirect that swallows the route — an app-lock, an auth gate), the entry
  /// answers [WalletSendNoTransaction] itself, and that answer is FINAL for
  /// this request: a screen that mounts later shows the request expired and
  /// offers no way to pay it, a host re-navigation onto the send path with
  /// the same request is refused, and the user starts again from the host.
  /// A host never holds a "nothing was created" for a request that then paid.
  ///
  /// This replaces the pre-FR-26 `Future<void>`, which completed on POP — a
  /// signal true of the user who paid and of the user who backed out alike. A
  /// host that composed a payment record on it put a false claim of payment in
  /// front of the payee.
  ///
  /// The [request] rides as GoRouter `extra` — in-session only, NOT a shareable
  /// deep-link URL. That is correct: a prefilled payment is an in-app action
  /// (from a scan / tap), never a bookmarkable link. `extra` does not survive a
  /// web reload, an OS deep link, or an Android process-death restore — the
  /// screen then opens EMPTY (a safe fallback, nothing sensitive is URL-encoded);
  /// the host re-issues the push from its own (re-derivable) source if needed.
  ///
  /// For a ZIP-321 URI, build [request] with [WalletSendRequest.fromUri] FIRST —
  /// it throws [WalletSendRequestException] AT THE SEAM on a malformed /
  /// wrong-network / multi-leg / unsupported-memo URI, so the host shows an
  /// honest error and never reaches here with a half-filled form:
  /// ```dart
  /// try {
  ///   final request = WalletSendRequest.fromUri(session, scannedUri);
  ///   WalletSendEntry.push(context, request);
  /// } on WalletSendRequestException catch (e) {
  ///   // show honest copy keyed off e.fault — no navigation
  /// }
  /// ```
  ///
  /// Contract notes:
  /// - `request.lockRecipient` is CONDITIONAL — applied only when the recipient
  ///   classifies sendable; see [WalletSendRequest.lockRecipient].
  /// - Navigate via THIS helper (`context.push` under the hood). The send
  ///   screen also re-seeds if a host updates the route in place
  ///   (`replace`/`go` with a different `extra`), but `push` is the supported
  ///   entry.
  /// - A push while a send is ALREADY submitting re-attaches to the running
  ///   step first (its outcome must not be swallowed) and then resets to this
  ///   request's fresh form the moment it lands — the prior send's outcome
  ///   stays on the wallet's durable surfaces (activity, the in-flight cue).
  ///   That prior outcome is NOT reported here: it belongs to the request that
  ///   opened it, not to this one.
  static Future<WalletSendReport> push(
    BuildContext context,
    WalletSendRequest request,
  ) {
    final reporter = WalletSendReporter(request.correlationId);
    // Stage S8 `deadline` (ADR-0558): the grace's revoke is REMEMBERED on the
    // root-scoped send controller, so a bare re-navigation with this request
    // is refused through every door, including a fresh mount long after this
    // call's context is gone. The controller is resolved NOW, while the
    // caller's context and its container are live; the timer below holds the
    // object itself and reads neither a context nor a container five seconds
    // old (a container disposed inside the grace — the app tearing down —
    // refuses reads; the object outlives it and the write is inert).
    final controller = ProviderScope.containerOf(
      context,
      listen: false,
    ).read(sendControllerProvider.notifier);
    unawaited(
      context.push<void>(
        WalletRoutes.send,
        extra: WalletSendEntryArgs(request, reporter),
      ),
    );
    // LIVENESS, and it deliberately does NOT hang off the navigation future.
    //
    // A send screen that mounts owns the answer and always delivers one, at the
    // latest from its teardown. The case left over is the screen that never
    // mounts at all — and go_router applies redirects BEFORE it attaches the
    // push future's completer, so a host redirect (an app-lock or auth gate,
    // ordinary on a wallet host) swallows the route and that future never
    // completes either. Chaining the fallback onto it, as this first shipped,
    // left the host awaiting a money surface forever in exactly the case the
    // fallback existed for.
    //
    // So: give the mount a bounded grace period. `attached` is set in the
    // screen's `initState`, so there is no race with a screen that did build —
    // it only ever answers for one that did not, and then the honest answer is
    // [WalletSendNoTransaction]: no screen, no controller, nothing touched.
    //
    // AND THAT ANSWER IS FINAL (stage S8 `deadline`, R05). A redirect that
    // held the route past the grace can still release it, and the screen that
    // then mounts would pay under a report the host already holds as "nothing
    // was created" — a payment on chain with no host record. So the grace does
    // not merely CONCLUDE, it REVOKES: the request loses its ability to spend
    // before the negative is delivered, the late screen renders "this request
    // expired" with no pay path, and the controller refuses the spend at both
    // of its seams before the host's authorizer is ever asked. What the honest
    // negative costs is one restart from the host for a user whose phone took
    // longer than the grace to open the send screen; the copy says so.
    //
    // Two writes, both BEFORE the negative goes out: the request into the
    // controller's revoked memory (the bare doors — an in-place re-navigation
    // and a fresh mount — read it), then the channel's own bit (the late
    // screen that mounts on THIS channel reads that).
    final grace = Timer(_mountGrace, () {
      if (reporter.attached || reporter.isReported) return;
      controller.revokeRequest(request);
      reporter.revoke();
      reporter.report(
        WalletSendNoTransaction(correlationId: request.correlationId),
      );
    });
    // Cancelled the moment the answer arrives — a timer left running past its
    // own purpose is a leak, and in a widget test it is a failure.
    unawaited(reporter.future.whenComplete(grace.cancel));
    return reporter.future;
  }

  /// How long a pushed send route has to actually mount its screen before the
  /// entry answers the host itself. Generous by design: it must clear a redirect
  /// chain and a route transition on a cold, loaded device, and the only cost of
  /// being late is a host that waits — where the cost of being early is telling
  /// a host "nothing was created" about a screen that is mid-build.
  static const _mountGrace = Duration(seconds: 5);
}
