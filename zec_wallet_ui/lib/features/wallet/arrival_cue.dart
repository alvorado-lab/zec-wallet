import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'wallet_providers.dart';
import 'wallet_session.dart';

/// The "Payment received" snackbar's own watermark (S13 §1.6, revision 2 M3).
///
/// The SDK delivers a live arrival at least once (its `IncomingFundsEvent`
/// doc: a host gates on its own watermark), so a resume re-delivered the last
/// event and the snackbar said "Payment received" again for money already
/// announced. This is that gate, and ONLY that gate: it suppresses a span at
/// or below one already shown in this session, nothing else.
///
/// Deliberately NOT the coin's watermark (`WalletCoinSpins.bumpArrival`): the
/// coin spins only inside its own gate (no catch-up cue, a latched tip, a
/// span past it), so gating the snackbar on it would hide real arrivals
/// during catch-up and before the first latch.
final walletArrivalCueProvider = NotifierProvider<WalletArrivalCue, int>(
  WalletArrivalCue.new,
);

/// State: the highest span shown in the current session (-1 when none).
class WalletArrivalCue extends Notifier<int> {
  /// The session the watermark belongs to. A rescan or a server switch opens
  /// a new one, so a height the previous one showed cannot silence it.
  WalletSession? _session;

  @override
  int build() => -1;

  /// Whether an arrival reaching [spanToHeight] should be announced, and if
  /// so, record it. A null span always shows (nothing to compare it to).
  bool admit(int? spanToHeight) {
    final session = ref.read(walletSessionProvider);
    if (!identical(session, _session)) {
      _session = session;
      state = -1;
    }
    if (spanToHeight == null) return true;
    if (spanToHeight <= state) return false;
    state = spanToHeight;
    return true;
  }
}
