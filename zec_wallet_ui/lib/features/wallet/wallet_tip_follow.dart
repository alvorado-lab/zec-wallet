import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import 'wallet_display_sync_status.dart';
import 'wallet_providers.dart';

/// How far past the last healthy tip a scan may reach and still be the wallet
/// FOLLOWING the chain rather than catching up to it. The loop polls every
/// 20 s and a block lands about every 75 s, so a synced wallet's pass
/// normally scans one block, rarely two or three. A larger gap (the phone
/// slept, a server was behind) is a real catch-up the user should see.
const kWalletTipFollowBlocks = 10;

/// The displayed sync status as the wallet tab's sync bar and coin read it
/// (FR-49 S12, C2/C3; the S12 security review's MEDIUM).
///
/// A synced wallet does not sit in `UpToDate`: every pass that finds a new
/// block publishes `Scanning` for each batch it commits, one-block batches
/// included, then `UpToDate` at the new tip. Read naively, that is the bar
/// sliding in and out and the coin spinning about once per block. This names
/// such a pass once, so the bar and the coin cannot disagree about it:
/// [following] is true for a `Scanning` sample that is not rewound, reaches at
/// or above the tip of a plain `UpToDate` this session saw with nothing else
/// between, and by at most [kWalletTipFollowBlocks].
///
/// Any other state between two `UpToDate`s ends the follow — a stall,
/// offline, idle (a suspend), connecting, a rewound or deep scan, or a
/// QUALIFIED reached tip (limited, degraded, unverified, behind), whose bar is
/// shown anyway. The scan's own `spendableReady` is deliberately not read: it
/// is false on the first download tick of every pass and on a wallet holding
/// nothing spendable.
///
/// The bound is on chain-tip movement (`to` against the followed tip), not
/// on the work left in the pass: it relies on the core never running a deep
/// scan at a steady tip without stamping `rewound` (a rescan swaps the
/// session). The core keeps that only WITHIN a pass: a rewind stamps every
/// later sample of its pass, but a pass that rewinds and then fails leaves the
/// re-queued span to the next pass, which scans it unstamped. If the failure
/// is a blip the display's dwell absorbs, that re-scan reads as a follow (a
/// hidden bar over a healing reorg; presentation only). Carrying the stamp
/// across passes belongs in the core (`ProgressSink.rewound`). A future deep
/// pass at a steady tip — a history backfill, an in-session
/// rescan-from-height — must stamp something this rule can read.
final walletTipFollowProvider =
    NotifierProvider<WalletTipFollowNotifier, WalletTipFollow>(
      WalletTipFollowNotifier.new,
    );

/// See [walletTipFollowProvider].
class WalletTipFollow {
  const WalletTipFollow({this.status, this.healthyTip, this.following = false});

  /// The latest DISPLAYED status (null before the first sample).
  final SyncStatus? status;

  /// The tip of the plain `UpToDate` the wallet is following, or null when
  /// the follow has ended.
  final int? healthyTip;

  /// Whether [status] is a routine tip-follow scan.
  final bool following;
}

/// See [walletTipFollowProvider].
class WalletTipFollowNotifier extends Notifier<WalletTipFollow> {
  @override
  WalletTipFollow build() {
    // Session identity keys the follow, like the synced-tip latch (the
    // display provider below does not watch the session itself).
    ref.watch(walletSessionProvider);
    // The DISPLAY status, the one the sync bar renders — not the raw stream.
    // On a flaky link the core retries a blip within about a second; the
    // display's posture dwell never shows that Stalled, so it must not end
    // the follow either, or the bar would flash and the coin spin on every
    // blip (the fold review's MEDIUM). A stall the dwell adopts does end it.
    ref.listen<AsyncValue<SyncStatus>>(walletDisplaySyncStatusProvider, (
      _,
      next,
    ) {
      final s = next.value;
      if (s != null) state = walletTipFollowNext(state, s);
    });
    final s = ref.read(walletDisplaySyncStatusProvider).value;
    return s == null
        ? const WalletTipFollow()
        : walletTipFollowNext(const WalletTipFollow(), s);
  }
}

/// The follow's transition — pure, so the rule is tested without a stream.
WalletTipFollow walletTipFollowNext(WalletTipFollow cur, SyncStatus s) {
  if (s is SyncStatus_UpToDate) {
    return WalletTipFollow(status: s, healthyTip: s.tip);
  }
  final tip = cur.healthyTip;
  if (s is SyncStatus_Scanning &&
      !s.rewound &&
      tip != null &&
      s.to >= tip &&
      s.to - tip <= kWalletTipFollowBlocks) {
    return WalletTipFollow(status: s, healthyTip: tip, following: true);
  }
  return WalletTipFollow(status: s);
}
