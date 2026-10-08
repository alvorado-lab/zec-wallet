import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart' show WalletApiError;
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

import '../../l10n/app_localizations.dart';

/// Diagnostics: the FR-25 prefilled-send seam demo — the example's proof that a
/// HOST opens the audited send flow from its OWN entry point (a scanned QR / a
/// `zcash:` deep link / a typed pay-a-contact) via [WalletSendEntry.push] +
/// [WalletSendRequest], contributing only the entry, not a send form.
///
/// Two paths: (1) paste a ZIP-321 URI → [WalletSendRequest.fromUri] parses it
/// against the wallet's own network and REJECTS a malformed / wrong-network /
/// multi-leg / unsupported-memo URI at the seam (a snackbar, never navigation);
/// (2) a typed request — a self-send to the wallet's own address, so the demo
/// always yields a valid, Review-able form regardless of network. Nothing is
/// ever sent without the user confirming in the audited flow. Developer-facing,
/// EN-only (like the rest of the example shell).
class PrefilledSendScreen extends ConsumerStatefulWidget {
  const PrefilledSendScreen({super.key});

  @override
  ConsumerState<PrefilledSendScreen> createState() =>
      _PrefilledSendScreenState();
}

class _PrefilledSendScreenState extends ConsumerState<PrefilledSendScreen> {
  final _uriController = TextEditingController();
  bool _lockRecipient = false;

  /// Re-entrancy guard so a mashed button can't stack two send screens.
  bool _opening = false;

  /// FR-26 — what the entry reported about the last flow this screen opened.
  /// The whole point of the seam: a host that contributed only the entry can
  /// still say, honestly, what became of the payment — with no history read.
  String? _lastReport;

  /// Numbers the demo's own records, so the echoed correlation token proves it
  /// came back attached to the request that opened the flow.
  int _requests = 0;

  @override
  void dispose() {
    _uriController.dispose();
    super.dispose();
  }

  void _snack(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  /// The FR-26 report, as a developer-facing line. Deliberately exhaustive over
  /// the sealed family: a new variant must be given an honest reading here
  /// before this compiles, which is the point of it being sealed.
  ///
  /// Note what a real host would do with each — the demo only prints, but the
  /// distinctions are the ones a payment record turns on: only
  /// [WalletSendTransactionCreated] is a payment, [WalletSendQueuedOffline] is a
  /// promise with no transaction behind it yet, and
  /// [WalletSendUnclassified] is "ask the wallet", not "it failed".
  /// The motion axis, all three values. `indeterminate` is the one worth
  /// spelling out: a real host must treat it like in-motion for anything that
  /// could cause a second payment.
  String _motionNote(WalletSendMotion motion) => switch (motion) {
    WalletSendMotion.notInMotion => '',
    WalletSendMotion.inMotion => ', funds in motion — do not re-send',
    WalletSendMotion.indeterminate =>
      ', motion UNKNOWN — treat as in-flight, do not offer to pay again',
  };

  String _describe(WalletSendReport report) {
    final tag = report.correlationId ?? '(no id)';
    return switch (report) {
      WalletSendTransactionCreated(
        :final txids,
        :final broadcastCount,
        :final motion,
      ) =>
        '$tag → transaction exists: ${txids.length} tx '
            '($broadcastCount broadcast)${_motionNote(motion)} '
            '— ${txids.first}',
      // The queued id is the join key onto the wallet's parked-send surfaces —
      // a host that persists a queued record persists this with it, because
      // there is no txid and no later report to reconcile against.
      WalletSendQueuedOffline(:final queuedSendId) =>
        '$tag → queued offline (parked id: ${queuedSendId ?? "none"}); no '
            'transaction yet — do NOT record as paid',
      WalletSendAlreadySubmitted() =>
        '$tag → already submitted; not a second payment',
      WalletSendNoTransaction() => '$tag → nothing was created',
      WalletSendUnclassified() =>
        '$tag → unclassified; the wallet holds whatever moved',
    };
  }

  void _record(WalletSendReport report) {
    final line = _describe(report);
    if (!mounted) return;
    setState(() => _lastReport = line);
    _snack(line);
  }

  /// Path 1 — parse a pasted ZIP-321 URI and open the flow. A rejection is
  /// surfaced HERE (a snackbar), never as a half-filled form (the FR-25 seam
  /// contract). Synchronous up to the push — `fromUri` is a sync parse.
  Future<void> _openFromUri() async {
    if (_opening) return;
    final l10n = AppLocalizations.of(context);
    final session = ref.read(walletSessionProvider);
    if (session == null) {
      _snack(l10n.prefillDemoNoWallet);
      return;
    }
    final WalletSendRequest request;
    try {
      request = WalletSendRequest.fromUri(
        session,
        _uriController.text.trim(),
        lockRecipient: _lockRecipient,
        correlationId: 'uri-${++_requests}',
      );
    } on WalletSendRequestException catch (e) {
      _snack(l10n.prefillDemoRejected(e.fault.name));
      return;
    }
    setState(() => _opening = true);
    try {
      _record(await WalletSendEntry.push(context, request));
    } finally {
      // finally: a throwing push must not wedge `_opening` true.
      if (mounted) setState(() => _opening = false);
    }
  }

  /// Path 2 — a typed request (no URI): a self-send to the wallet's own address,
  /// so the demo is always valid + Review-able. The `currentAddress` read is
  /// async, so re-check `mounted` before navigating.
  Future<void> _openTyped() async {
    if (_opening) return;
    final l10n = AppLocalizations.of(context);
    final session = ref.read(walletSessionProvider);
    if (session == null) {
      _snack(l10n.prefillDemoNoWallet);
      return;
    }
    setState(() => _opening = true);
    try {
      // The package's own FFI wedge bound — a wedged bridge must not
      // leave the demo's buttons dead with no feedback.
      final ua = await session.currentAddress().timeout(walletFfiWedgeTimeout);
      if (!mounted) return;
      _record(
        await WalletSendEntry.push(
          context,
          WalletSendRequest(
            address: ua,
            amountZat: 100000, // 0.001 ZEC
            memo: 'typed request demo',
            lockRecipient: _lockRecipient,
            correlationId: 'typed-${++_requests}',
          ),
        ),
      );
    } on WalletApiError catch (e) {
      // Typed + payload-free: the code is static, never an address (§5.4).
      _snack('Could not read the wallet address (${e.code})');
    } catch (e) {
      _snack('Could not read the wallet address ($e)');
    } finally {
      if (mounted) setState(() => _opening = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.prefillDemoTitle)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(l10n.prefillDemoIntro),
          const SizedBox(height: 16),
          TextField(
            controller: _uriController,
            autocorrect: false,
            enableSuggestions: false,
            minLines: 1,
            maxLines: 3,
            decoration: InputDecoration(
              border: const OutlineInputBorder(),
              labelText: l10n.prefillDemoUriLabel,
              hintText: l10n.prefillDemoUriHint,
            ),
          ),
          const SizedBox(height: 8),
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: Text(l10n.prefillDemoLock),
            value: _lockRecipient,
            onChanged: (v) => setState(() => _lockRecipient = v),
          ),
          const SizedBox(height: 8),
          FilledButton(
            onPressed: _opening ? null : () => unawaited(_openFromUri()),
            child: _opening
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : Text(l10n.prefillDemoOpenFromUri),
          ),
          const SizedBox(height: 8),
          OutlinedButton(
            onPressed: _opening ? null : () => unawaited(_openTyped()),
            child: _opening
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : Text(l10n.prefillDemoOpenTyped),
          ),
          if (_lastReport case final report?) ...[
            const SizedBox(height: 24),
            const Text(
              'Last outcome report (FR-26)',
              style: TextStyle(fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 4),
            Text(report),
          ],
        ],
      ),
    );
  }
}
