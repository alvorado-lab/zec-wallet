import 'package:zec_wallet/zec_wallet.dart';

import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';

/// The block spacing the pre-upgrade pause's wait is counted in: 75 s, the
/// target spacing from Blossom until NU7. The blocks the pause waits for are the
/// ones BEFORE an activation, so through NU7 they are all 75 s blocks. Stated
/// limit: an upgrade after NU7 would be preceded by 25 s blocks, and this would
/// read three times long until it is changed.
const int upgradePauseBlockSeconds = 75;

/// The pause's window in blocks: the core's `PROPOSAL_ANCHOR_DRIFT_MAX_BLOCKS`, the K
/// the SDK refuses sends within before an activation, surfaced here once (a test pins
/// it to the core's source). A paused state always carries the SDK's `blocksLeft`;
/// only one the SDK did not produce lacks it, and that reads as this, the longest wait.
const int upgradePauseWindowBlocks = 20;

/// The wait the pre-upgrade send pause (ADR-0580) names, in whole minutes:
/// [blocksLeft] blocks at [upgradePauseBlockSeconds], rounded UP, so the copy
/// never promises sending back sooner than it is, and never "0 minutes" (the
/// SDK's `blocksLeft` is 1 to 20, so this is 2 to 25).
int upgradePauseMinutes(int blocksLeft) {
  final minutes = (blocksLeft * upgradePauseBlockSeconds + 59) ~/ 60;
  return minutes < 1 ? 1 : minutes;
}

/// The sentence ADR-0580 fixes (2026-10-30) for a pause with [blocksLeft]
/// blocks to go: "The Zcash network upgrades in about N minutes. Sending is
/// paused until then so your payment can't get stuck."
String upgradePausedText(WalletLocalizations l10n, int blocksLeft) =>
    l10n.walletSendFaultUpgradePaused(upgradePauseMinutes(blocksLeft));

/// [upgradePausedText] when [error] is the SDK's pause
/// (`WalletErrorKind.upgradeImminent`), else `null`: the one test the surfaces
/// that catch an error and show a line (the parked "Send now", the recovery
/// sweep, the reopen) use, so none of them reads the pause as a failure.
String? upgradePausedTextFor(WalletLocalizations l10n, Object error) =>
    switch (error) {
      WalletApiError(
        kind: WalletErrorKind_UpgradeImminent(:final blocksLeft),
      ) =>
        upgradePausedText(l10n, blocksLeft),
      _ => null,
    };
