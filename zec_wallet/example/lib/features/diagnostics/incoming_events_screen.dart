import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';
import 'package:zec_wallet_ui/zec_wallet_ui.dart';

import '../../l10n/app_localizations.dart';

/// Diagnostics: the live incoming-funds event stream (ADR-0536 #392) — the
/// example's proof that a HOST can consume [incomingFundsEventsProvider]. It
/// accumulates every event of this visit newest-first and renders the payload
/// VERBATIM: kinds, counts, height spans, and the opaque cursor are the whole
/// event — §5.4-safe by construction, so showing (or logging) them is fine by
/// design. Developer-facing (EN-only, like the rest of the example shell).
class IncomingEventsScreen extends ConsumerStatefulWidget {
  const IncomingEventsScreen({super.key});

  @override
  ConsumerState<IncomingEventsScreen> createState() =>
      _IncomingEventsScreenState();
}

class _IncomingEventsScreenState extends ConsumerState<IncomingEventsScreen> {
  /// Per-visit accumulation, newest first, CAPPED so a deep restore's
  /// per-batch replays can't grow this without bound (dev surface, not a log).
  static const _maxEvents = 200;
  final List<IncomingFundsEvent> _events = [];

  @override
  void initState() {
    super.initState();
    // Seed with the provider's current (latest-wins) event, if any — outside
    // build, so build never mutates state.
    final latest = ref.read(incomingFundsEventsProvider).value;
    if (latest != null) _events.add(latest);
  }

  void _record(IncomingFundsEvent event) {
    setState(() {
      _events.insert(0, event);
      if (_events.length > _maxEvents) _events.removeLast();
    });
  }

  @override
  Widget build(BuildContext context) {
    // Accumulate edges while this screen is open. The provider's state is the
    // LATEST event only (latest-wins coalescing is the stream's contract), so
    // a visit shows what arrives from now on — starting with the current one.
    ref.listen<AsyncValue<IncomingFundsEvent>>(incomingFundsEventsProvider, (
      prev,
      next,
    ) {
      final event = next.value;
      if (event != null && event != prev?.value) _record(event);
    });
    // A wallet-identity flip mid-visit: drop the previous identity's events
    // (heights/counts only, but mixing identities on one list is dishonest).
    ref.listen<Object?>(walletIdentityProvider, (prev, next) {
      if (prev != next) setState(_events.clear);
    });

    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.diagIncomingTitle)),
      body: _events.isEmpty
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Text(
                  l10n.diagIncomingEmpty,
                  textAlign: TextAlign.center,
                ),
              ),
            )
          : ListView.builder(
              itemCount: _events.length,
              itemBuilder: (context, index) {
                final e = _events[index];
                final span = e.spanFromHeight == null
                    ? '—'
                    : '${e.spanFromHeight}–${e.spanToHeight}';
                return ListTile(
                  leading: Icon(switch (e.kind) {
                    IncomingFundsEventKind.replay => Icons.history,
                    IncomingFundsEventKind.live => Icons.call_received,
                    IncomingFundsEventKind.memoRefresh => Icons.notes,
                    IncomingFundsEventKind.unknown => Icons.help_outline,
                  }),
                  // Dev surface: enum/field names render verbatim on purpose.
                  title: Text('${e.kind.name} · ${e.newTxCount} tx'),
                  subtitle: Text(
                    'span $span · total ${e.totalTxDetected} · '
                    // Delivered events always carry a cursor (M2 — the
                    // pump substitutes the last one); the '—' arm is defensive
                    // rendering only, never a contract a host may rely on.
                    'cursor ${e.cursor.isEmpty ? '—' : e.cursor}',
                  ),
                );
              },
            ),
    );
  }
}
