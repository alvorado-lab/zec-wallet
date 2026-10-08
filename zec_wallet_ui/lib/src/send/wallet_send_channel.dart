/// FR-26 delivery internals — the channel a `WalletSendEntry.push` opens and the
/// route argument that carries it to the send screen.
///
/// **They live under `lib/src/` for a reason.** The package barrel's `show` list
/// is documentation, not enforcement: everything under `lib/features/` is
/// deep-importable by a host, so a comment claiming these were unreachable was
/// simply false. Under `lib/src/` the analyzer says it for us — a host importing
/// this file trips `implementation_imports`.
///
/// What it protects is modest and worth stating exactly: forging a channel does
/// not let a host reach another flow's report (a channel is a per-push object
/// the pusher already holds). It would only let a host hand ITSELF a payment
/// record the wallet never made. That is still worth a lint, and it is not worth
/// overstating.
library;

import 'dart:async';

import '../../features/wallet/send/wallet_send_report.dart';
import '../../features/wallet/send/wallet_send_request.dart';

/// The one-shot channel a `WalletSendEntry.push` hands to the send screen, and
/// the future it returns to the host.
///
/// It rides on the route's `extra` (inside [WalletSendEntryArgs]) so the public
/// [WalletSendRequest] stays a pure, const-constructible value with no live
/// object hanging off it — which also matters because two identical const
/// requests are the SAME object, so a request can never identify a flow.
///
/// Exactly one report is ever delivered ([report] after the first is a no-op),
/// because a host may only be told once what happened to one payment.
class WalletSendReporter {
  WalletSendReporter(this.correlationId);

  /// Echoed into every report this channel delivers.
  final String? correlationId;

  final Completer<WalletSendReport> _completer = Completer<WalletSendReport>();

  /// Whether a send screen ever took ownership of this channel. `false` means
  /// no screen ever mounted for it — the one case the entry helper has to close
  /// out itself, because a screen that mounted always answers, at the latest
  /// from its teardown.
  bool get attached => _attached;
  bool _attached = false;

  /// Whether the single report has been delivered.
  bool get isReported => _completer.isCompleted;

  /// Whether the REQUEST behind this channel has lost its ability to spend
  /// (stage S8 `deadline`). Set by the entry helper's mount grace, before it
  /// answers the host "no transaction" for a screen that never appeared: that
  /// negative is final, so the request it names must not pay afterwards — not
  /// from the screen that eventually mounts, not from a fresh form, not from a
  /// host re-navigation with the same request.
  ///
  /// A SEPARATE bit from [isReported], and deliberately so. A channel is
  /// reported for a positive outcome too, and it is reported "no transaction"
  /// for a retired flow whose SCREEN is still live and spendable (an in-place
  /// route update); a denied authorization leaves the channel open and the
  /// flow spendable. None of those revoke anything. Only a negative that
  /// stands for the request itself does.
  bool get isRevoked => _revoked;
  bool _revoked = false;

  /// The host's future. Completes exactly once, and never with an error.
  Future<WalletSendReport> get future => _completer.future;

  void markAttached() => _attached = true;

  /// Revoke the request's ability to spend — see [isRevoked]. Called BEFORE
  /// the negative report it belongs with, so no reader can see the report
  /// without the revocation.
  void revoke() => _revoked = true;

  /// Deliver the report. A second call is a deliberate no-op: the first answer
  /// is the true one (the screen reports a flow's terminal the moment it lands,
  /// and its own teardown must not overwrite that with an abandonment).
  void report(WalletSendReport report) {
    if (_completer.isCompleted) return;
    _completer.complete(report);
  }
}

/// Route argument: the host's request plus the channel its report goes back
/// through. The router accepts a bare [WalletSendRequest] too (an in-place route
/// update, or a host that navigated the send path itself) — that flow simply
/// gets no report, which is the pre-FR-26 behaviour.
class WalletSendEntryArgs {
  const WalletSendEntryArgs(this.request, this.reporter);

  final WalletSendRequest request;
  final WalletSendReporter reporter;
}
