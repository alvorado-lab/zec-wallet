// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class WalletLocalizationsEn extends WalletLocalizations {
  WalletLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Settings';

  @override
  String get walletTitle => 'Wallet';

  @override
  String get walletNotSetUpTitle => 'Wallet not set up yet';

  @override
  String get walletNotSetUpBody =>
      'Wallet setup arrives in a later build. Setup will walk you through writing down your recovery phrase before any funds can be received — so nothing is ever at risk without a backup.';

  @override
  String get walletStartupFailedTitle => 'The wallet couldn\'t start';

  @override
  String get walletStartupFailedBody =>
      'Something stopped the wallet from loading on this device. If you already have a wallet, its funds are not affected — they live on the Zcash network and can be restored with your recovery phrase. Try again; if this keeps happening, close and reopen the app.';

  @override
  String get walletBalanceLabel => 'Balance';

  @override
  String get walletHideBalance => 'Hide balance';

  @override
  String get walletShowBalance => 'Show balance';

  @override
  String get walletBalanceHiddenAmount => 'Balance hidden';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Spendable now';

  @override
  String get walletArrivingLabel => 'Arriving';

  @override
  String get walletNotSpendableYetLabel => 'Not spendable yet';

  @override
  String get walletActivityTitle => 'Activity';

  @override
  String get walletActivityEmpty => 'No activity yet';

  @override
  String get walletActivityError => 'Couldn\'t load activity';

  @override
  String get walletActivityReceived => 'Received';

  @override
  String get walletActivitySent => 'Sent';

  @override
  String get walletActivityPending => 'Pending';

  @override
  String get walletActivityQueued => 'Queued';

  @override
  String get walletActivityRetrying => 'Retrying';

  @override
  String get walletActivitySaved => 'Saved';

  @override
  String get walletActivityExpired => 'Expired';

  @override
  String get walletActivityFailed => 'Failed';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count confirmations',
      one: '1 confirmation',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count payments received',
      one: 'Payment received',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Show transaction details';

  @override
  String get walletTxDetailStatus => 'Status';

  @override
  String get walletTxDetailFee => 'Network fee';

  @override
  String get walletTxDetailDate => 'Date';

  @override
  String get walletTxDetailHeight => 'Block height';

  @override
  String get walletTxDetailMemo => 'Memo';

  @override
  String get walletTxDetailMemoAttached => 'Included';

  @override
  String get walletTxDetailTxid => 'Transaction ID';

  @override
  String get walletTxDetailCopyTxid => 'Copy transaction ID';

  @override
  String get walletTxDetailCopied => 'Transaction ID copied';

  @override
  String get walletTxDetailClose => 'Close';

  @override
  String get walletTxFundsKept => 'No funds left your wallet';

  @override
  String get walletTxExplainQueued =>
      'Saved on this device, under Saved & pending — you can send it or cancel it there.';

  @override
  String get walletTxExplainPending =>
      'Sent to the Zcash network — waiting to be confirmed in a block.';

  @override
  String get walletTxExplainRetrying =>
      'Your wallet couldn\'t send this to the Zcash network yet. It keeps the signed transaction and tries again on each sync until it goes through or expires.';

  @override
  String get walletTxExplainSaved =>
      'Your wallet has kept this signed transaction but isn\'t sending it by itself right now.';

  @override
  String get walletTxExplainConfirmed => 'Confirmed on the Zcash network.';

  @override
  String get walletTxExplainExpired =>
      'This transaction expired before the network confirmed it, so it was cancelled. The amount is still yours to spend.';

  @override
  String get walletTxExplainFailed =>
      'The network rejected this transaction, so it didn\'t go through. The amount is still yours to spend.';

  @override
  String get walletTxExplainUnknown =>
      'This transaction\'s current status can\'t be determined. It will update after the next sync.';

  @override
  String get walletMenuTooltip => 'More options';

  @override
  String get walletRescanMenuItem => 'Rescan history…';

  @override
  String get walletCheckOneTimeMenuItem => 'Check one-time addresses…';

  @override
  String get walletRescanTitle => 'Rescan your history';

  @override
  String get walletRescanBody =>
      'Missing older funds? Re-scan the blockchain from further back to recover deposits an earlier start date skipped. Your funds and recovery phrase are never at risk.';

  @override
  String get walletRescanRangeTitle => 'How far back to scan';

  @override
  String get walletRescanRangeAll =>
      'Scan your whole history — slowest, but recovers everything.';

  @override
  String get walletRescanRangeDefault =>
      'Scanning from your wallet\'s start. Restored this wallet and older funds are still missing? Pick an earlier date, or Scan all history.';

  @override
  String get walletRescanRangeResolving => 'Preparing the recommended range…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'About $blocks blocks to scan.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Scanning from $date on. Still missing older funds? Pick an earlier date, or Scan all history.';
  }

  @override
  String get walletRescanPick => 'Pick a date';

  @override
  String get walletRescanChange => 'Change date';

  @override
  String get walletRescanScanAll => 'Scan all history';

  @override
  String get walletRescanDatePick => 'Earliest date to scan';

  @override
  String get walletRescanWarning =>
      'This re-scans the blockchain. Recent dates take minutes; scanning far back can take hours. Sync runs in the background — you can keep using your wallet.';

  @override
  String get walletRescanSettlingAdvisory =>
      'A payment from this wallet is still settling. The wallet usually declines to rescan until it completes — you can try, but expect it to be refused.';

  @override
  String get walletRescanConfirm => 'Start rescan';

  @override
  String get walletRescanCancel => 'Cancel';

  @override
  String get walletRescanRunning => 'Rebuilding…';

  @override
  String get walletRescanRebuildingAll =>
      'Rebuilding your history — scanning your whole chain. Your balance and activity fill in as it catches up.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Rebuilding your history from $date — your balance and activity fill in as it catches up.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Rebuilding your history from your wallet\'s start — your balance and activity fill in as it catches up.';

  @override
  String get walletCatchUpBanner =>
      'Catching up — your balance and activity fill in as the wallet syncs. Anything you\'ve received is safe.';

  @override
  String get walletCatchUpRescanBanner =>
      'Rebuilding your history after a rescan — your balance and activity fill in as it catches up. Anything you\'ve received is safe.';

  @override
  String get walletRescanFailedNotice =>
      'Couldn\'t rescan right now — your funds are safe, though your balance and history may need a little time to catch back up. Try again in a moment.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'A payment is still settling, so rescanning is paused to protect your funds. Your wallet is unchanged — try again in a couple of hours and keep the app open and online.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'Rescanning rebuilds your history as your wallet syncs, and syncing isn\'t running right now. Your wallet is unchanged — try again once syncing is running.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'There isn\'t enough free space to rebuild your wallet history — your funds are safe, though your balance and history may need a little time to catch back up. Free up some space and try again.';

  @override
  String get walletRescanFailedDismiss => 'Dismiss';

  @override
  String get walletActivityRebuilding => 'Rebuilding your history…';

  @override
  String get walletActivityCatchingUp =>
      'Still catching up — anything you\'ve received will show up here.';

  @override
  String get walletActivitySyncNotRunning =>
      'Your balance and history will finish loading once syncing is running.';

  @override
  String get walletActivityLoadMore => 'Load more';

  @override
  String get walletPendingChangeLabel => 'Pending change';

  @override
  String get walletTransparentLabel => 'Unshielded (public)';

  @override
  String get walletTransparentNote =>
      'Not included in \"Spendable now\" — shield these funds to spend them. Until then they stay publicly visible on-chain.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'These funds are publicly visible on-chain.';

  @override
  String walletPoolShielded(String amount) {
    return 'Shielded $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Public $amount';
  }

  @override
  String get walletPoolAllShielded => 'All shielded · private';

  @override
  String get walletPoolTapHint => 'Show public funds';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount of your balance is on a one-time address (recoverable).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount of your balance is on a one-time address.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Payments totalling $amount are set aside and still completing through one-time addresses your wallet controls. Don\'t send them again.',
      one:
          '$amount is set aside for a payment your wallet is still completing through a one-time address it controls. Don\'t send it again.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Payments totalling $amount are set aside partway through one-time addresses your wallet controls. They\'re paused until your wallet is syncing again. Don\'t send them again.',
      one:
          '$amount is set aside for a payment partway through a one-time address your wallet controls. It\'s paused until your wallet is syncing again. Don\'t send it again.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Couldn\'t check whether a payment is still completing. Retrying — until then, look for a pending payment in your activity before you send again.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount of your balance is on a one-time address (still confirming).';
  }

  @override
  String get walletShieldButton => 'Shield';

  @override
  String get walletShieldSheetTitle => 'Shield public funds';

  @override
  String get walletShieldNote =>
      'This moves funds from your public, on-chain-visible balance into your private shielded balance.';

  @override
  String get walletShieldPreparing => 'Preparing…';

  @override
  String get walletShieldAmountLabel => 'Shielding';

  @override
  String get walletShieldFeeLabel => 'Network fee';

  @override
  String get walletShieldNetLabel => 'Lands shielded';

  @override
  String get walletShieldConfirmButton => 'Shield now';

  @override
  String get walletShieldSubmitting => 'Shielding…';

  @override
  String get walletShieldNothingTitle => 'Nothing to shield yet';

  @override
  String get walletShieldNothingBody =>
      'These funds are below the amount worth shielding right now — the network fee would outweigh the benefit. They\'ll be shieldable once a bit more arrives.';

  @override
  String get walletShieldDoneTitle => 'Shielding submitted';

  @override
  String get walletShieldDoneBody =>
      'Your funds are moving into your shielded balance. It will confirm on-chain shortly.';

  @override
  String get walletShieldSavedTitle => 'Saved — we\'ll finish shielding';

  @override
  String get walletShieldSavedBody =>
      'We couldn\'t reach the network just now. Your shielding is saved and your wallet will complete it on a later sync. Nothing is lost.';

  @override
  String get walletShieldAlreadyTitle => 'Already submitted';

  @override
  String get walletShieldFailedTitle => 'Couldn\'t shield right now';

  @override
  String get walletShieldStaleBody =>
      'The wallet is still syncing. Try shielding again in a moment.';

  @override
  String get walletShieldTransientBody =>
      'Couldn\'t prepare the shield just now. Try again in a moment.';

  @override
  String get walletShieldStorageFullBody =>
      'There isn\'t enough free space to shield right now. Free up some space and try again. Your funds are safe.';

  @override
  String get walletShieldClose => 'Close';

  @override
  String get walletShieldRetry => 'Try again';

  @override
  String get walletMoveMenuItem => 'Move to public…';

  @override
  String get walletMoveSheetTitle => 'Move to public';

  @override
  String get walletMoveSheetSubtitle =>
      'Send shielded ZEC to your own public address — useful for an exchange that won\'t accept a shielded deposit.';

  @override
  String get walletMoveDestinationLabel => 'Your public address';

  @override
  String walletMoveAvailable(String amount) {
    return 'Available to move: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Available to move: $amount ZEC — your balance is still catching up';
  }

  @override
  String get walletMoveDeshieldTitle => 'This move makes your funds public';

  @override
  String get walletMoveDeshieldBody =>
      'Moving to a public address takes these funds out of your shielded balance — the amount and your public address become publicly visible on the Zcash blockchain.';

  @override
  String get walletMoveWalletEnded =>
      'The wallet session ended. Close and reopen to try again.';

  @override
  String get walletMoveLoading => 'Preparing…';

  @override
  String get walletMovePreparing => 'Checking the amount…';

  @override
  String get walletMoveSubmitting => 'Moving…';

  @override
  String get walletMoveReviewButton => 'Review';

  @override
  String get walletMoveCancel => 'Cancel';

  @override
  String get walletMoveReviewTitle => 'Review move';

  @override
  String get walletMoveOwnAddressNote =>
      'You\'re moving to your own public address. You can shield these funds again later, but this move stays on the public record permanently.';

  @override
  String get walletMoveConfirmButton => 'Move to public';

  @override
  String get walletMoveBackButton => 'Back';

  @override
  String get walletMoveDoneTitle => 'Moved to public';

  @override
  String get walletMoveDoneBody =>
      'Your funds are moving to your public address. They will confirm on-chain shortly.';

  @override
  String get walletMoveSavedTitle => 'Saved — we\'ll finish the move';

  @override
  String get walletMoveSavedBody =>
      'This move is saved and your wallet will send it on a later sync. Nothing was lost.';

  @override
  String get walletMoveAlreadyTitle => 'Already submitted';

  @override
  String get walletMoveAlreadyBody =>
      'These funds were already submitted and are on their way to your public address.';

  @override
  String get walletMoveFailedTitle => 'Couldn\'t complete this move';

  @override
  String get walletMoveNothingTitle => 'Nothing to move yet';

  @override
  String get walletMoveNothingBody =>
      'You have no shielded balance available to move right now. Once funds confirm, you can move them to your public address.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Your wallet is still catching up — anything you\'ve received becomes available to move once syncing completes.';

  @override
  String get walletMoveCouldNotLoad =>
      'Couldn\'t load your public address. Try again.';

  @override
  String get walletMoveRetry => 'Try again';

  @override
  String get walletMoveClose => 'Close';

  @override
  String get walletSnapshotUnavailable =>
      'Couldn\'t read the wallet right now. It will refresh on its own.';

  @override
  String get walletBalanceStale =>
      'Couldn\'t refresh — showing your last-known balance.';

  @override
  String get walletSyncStartFailed =>
      'Couldn\'t start syncing. We\'ll keep trying.';

  @override
  String get walletSyncRetry => 'Try again';

  @override
  String get walletSyncTryNow => 'Try now';

  @override
  String get walletSyncIdle => 'Not syncing yet';

  @override
  String get walletSyncIdleDetail => 'Syncing starts automatically.';

  @override
  String get walletSyncDisabled => 'Sync off';

  @override
  String get walletSyncDisabledDetail =>
      'Turn on syncing in this app\'s settings to update your balance.';

  @override
  String get walletSyncExplainDisabled =>
      'Syncing is turned off in this app\'s settings. Your funds are safe. Your balance and activity show the last synced state and won\'t update until syncing is turned on.';

  @override
  String get walletParkedSyncPausedNote =>
      'Your wallet isn\'t syncing, so these won\'t send on their own. Use Send now to send one yourself.';

  @override
  String get walletSyncPausedMoneyNote =>
      'Paused until your wallet is syncing again.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Connecting…';

  @override
  String get walletSyncStartingDetail =>
      'Reaching the Zcash network and preparing to scan.';

  @override
  String get walletSyncConnecting => 'Connecting…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Connecting… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Scanning $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'Scanning…';

  @override
  String get walletSyncSpendableReady => 'Funds are ready to spend.';

  @override
  String get walletSyncCatchingUp =>
      'Catching up with the network — a deep initial sync can take a while. You can keep using the app while it finishes';

  @override
  String walletSyncScanRemaining(String count) {
    return '$count blocks left';
  }

  @override
  String get walletSyncUpToDate => 'Up to date';

  @override
  String get walletSyncOffline => 'Offline';

  @override
  String get walletSyncOfflineDetail =>
      'Queued sends stay saved under Saved & pending.';

  @override
  String get walletSyncUnknown => 'Syncing…';

  @override
  String get walletSyncStalled => 'Sync paused';

  @override
  String get walletStallEndpoint =>
      'Can\'t reach the Zcash network right now. We\'ll keep trying automatically — check your connection, or the server may be temporarily unavailable.';

  @override
  String get walletStallTor =>
      'Your app\'s private path isn\'t available, so the wallet isn\'t connecting. Check your app\'s network settings, or turn the private path off. Sync resumes as soon as the path is back.';

  @override
  String get walletStallStorage =>
      'Device storage is full. Free some space and sync will resume.';

  @override
  String get walletStallReorg =>
      'The chain reorganized; re-checking recent blocks.';

  @override
  String get walletStallInternal =>
      'A local problem stopped sync. If it keeps happening, restore from your recovery phrase.';

  @override
  String get walletStallEndpointMisbehaving =>
      'This server sent data that can\'t be right, so sync stopped. It isn\'t a connection problem — switch to another server. If every server is refused, rescan your history: the wallet may be keeping a bad record from an earlier server.';

  @override
  String get walletStallBirthdayInFuture =>
      'This wallet is set to start from a block this server hasn\'t reached yet. Check the starting block this wallet is set to, or try another server.';

  @override
  String get walletStallStorageUnavailable =>
      'Sync paused on this device. Retrying.';

  @override
  String get walletStallUnknown => 'Sync stopped for an unknown reason.';

  @override
  String get walletSyncBadgeHint => 'Show sync details';

  @override
  String get walletSyncSheetClose => 'Close';

  @override
  String get walletSyncSheetProgress => 'Progress';

  @override
  String get walletSyncSheetBlocksLeft => 'Blocks left';

  @override
  String get walletSyncSheetSyncedTo => 'Synced to block';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Behind by at least $blocks blocks',
      one: 'Behind by at least 1 block',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'Syncing hasn\'t started yet — it starts automatically. No action needed.';

  @override
  String get walletSyncExplainStartFailed =>
      'Syncing couldn\'t start. Your funds are safe — the wallet just isn\'t checking for new activity. Try again below, or reopen the app.';

  @override
  String get walletSyncExplainStarting =>
      'The wallet is contacting the Zcash network and preparing to scan. This usually takes a few seconds.';

  @override
  String get walletSyncExplainConnecting =>
      'Establishing a connection to the Zcash network.';

  @override
  String get walletSyncExplainScanning =>
      'The wallet is checking blockchain blocks for your funds. Your balance and activity update as new transactions are found — you can keep using the app while it finishes.';

  @override
  String get walletSyncExplainUpToDate =>
      'Fully synced with the Zcash network. Your balance and activity are current.';

  @override
  String get walletSyncExplainStalled =>
      'Syncing hit a problem and is paused. It retries automatically.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Can\'t reach the Zcash network — that\'s normal if you\'re offline, or the server may be temporarily unavailable. Your funds are safe: the balance shows the last synced state, and queued sends stay saved under Saved & pending. The connection retries on its own.';

  @override
  String get walletSyncExplainOffline =>
      'No network connection. Your funds are safe — the balance shows the last synced state, and queued sends stay saved under Saved & pending.';

  @override
  String get walletSyncExplainUnknown =>
      'The wallet is syncing. Your balance and activity update as it progresses.';

  @override
  String get walletTorOff => 'Tor off';

  @override
  String get walletTorBootstrapping => 'Private path starting…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport starting…';
  }

  @override
  String get walletTorActive => 'Tor active';

  @override
  String get walletTorActiveUnverified => 'Tor active (unverified runtime)';

  @override
  String get walletTorActiveUnattested =>
      'Private path in use (privacy not verified)';

  @override
  String get walletTorFellBack => 'Tor unavailable — using direct connection';

  @override
  String get walletTorUnavailable => 'Private path unavailable — not connected';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport unavailable — not connected';
  }

  @override
  String get walletTorUnanswered =>
      'Private path connected — nothing coming back';

  @override
  String get walletTorUnansweredUnattested =>
      'Private path connected — nothing coming back (privacy not verified)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport connected — nothing coming back';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Not private (your app\'s direct connection) — nothing coming back';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Connected over $transport — nothing coming back; connections can be linked by the proxy';
  }

  @override
  String get walletTorUnknown => 'Tor status unknown — treat as not protected';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Balance (as of block $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Balance · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Balance (as of block $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Connection';

  @override
  String get walletSyncSheetServer => 'Server';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Server, $host, opens the server picker';
  }

  @override
  String get walletSyncServerSheetTitle => 'Sync server';

  @override
  String get walletSyncServerInUse => 'In use';

  @override
  String get walletSyncServerAppDefault => 'App default';

  @override
  String get walletSyncServerCustom => 'Custom server…';

  @override
  String get walletSyncServerCustomHint => 'https://host:port';

  @override
  String get walletSyncServerCheck => 'Check server';

  @override
  String get walletSyncServerChecking => 'Checking…';

  @override
  String get walletSyncServerUse => 'Use this server';

  @override
  String get walletSyncServerSwitching => 'Switching…';

  @override
  String get walletSyncServerContinue => 'Continue';

  @override
  String get walletSyncServerCancel => 'Cancel';

  @override
  String get walletSyncServerTrustTitle => 'Trust this server?';

  @override
  String get walletSyncServerTrustNotice =>
      'You\'re trusting this server to report your balance and history and to relay your payments. It will see your IP address unless Tor is on, roughly when your wallet was created, the public addresses your wallet checks, the transactions it looks up, and the transactions you send.';

  @override
  String get walletSyncServerKeyLabel => 'Access key (optional)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Key header';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Enter the header your server expects';

  @override
  String get walletSyncServerKeyInvalid => 'This key or header can\'t be used';

  @override
  String get walletSyncServerKeySaved => 'Key saved';

  @override
  String get walletSyncServerKeyShow => 'Show';

  @override
  String get walletSyncServerKeyHide => 'Hide';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Your key identifies you to this server. It can link your payments to your wallet, even over Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Switching restarts the sync in progress. Your balance and history stay. Funds may show as arriving until the new server\'s scan catches up.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Switching reconnects to the new server. Your balance and history stay.';

  @override
  String get walletSyncServerUnreachable =>
      'Couldn\'t reach this server. Check the address — and if it\'s right, either this server isn\'t answering or your app can\'t reach it right now. Try again, or pick another server.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Couldn\'t reach this server. The wallet can\'t tell whether this server isn\'t answering or your app can\'t reach it right now. Pick another server, or try again later.';

  @override
  String get walletSyncServerWrongNetwork =>
      'This server is on a different Zcash network.';

  @override
  String get walletSyncServerInvalidUrl =>
      'That doesn\'t look like a server address. Use https://host:port.';

  @override
  String get walletSyncServerNotOffered =>
      'This server isn\'t offered by this app.';

  @override
  String get walletSyncServerBusy =>
      'The wallet is busy right now. Try again in a moment.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'The server you chose isn\'t offered by this app any more. Using $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'The remembered server choice couldn\'t be read. Using $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Couldn\'t switch — still using $host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Wallet traffic connects directly to the server. The server can see your IP address.';

  @override
  String get walletTransportExplainTor =>
      'Wallet traffic is routed through the Tor network, which hides your IP address from the server.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Your app\'s private path is starting up. Wallet traffic waits for it before connecting.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport is starting up. Wallet traffic waits for it before connecting.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Tor couldn\'t be reached, so traffic fell back to a direct connection. The server can see your IP address.';

  @override
  String get walletTransportExplainUnavailable =>
      'Your app\'s private path isn\'t available, so the wallet isn\'t connecting. Turn the private path off, or check your app\'s network settings.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport isn\'t available, so the wallet isn\'t connecting. Turn it off, or check your app\'s network settings.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'The private path took the connection, but nothing has come back for a minute. It could be the path or the wallet server — the wallet can\'t tell which. It keeps trying; if it doesn\'t clear, try another server or check your app\'s network settings.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport took the connection, but nothing has come back for a minute. It could be the path or the wallet server — the wallet can\'t tell which. It keeps trying; if it doesn\'t clear, try another server or check your app\'s network settings.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Wallet traffic connects directly to the server. The server can see your IP address. The connection was accepted, but nothing has come back for a minute. It could be the path or the wallet server — the wallet can\'t tell which. It keeps trying; if it doesn\'t clear, try another server or check your app\'s network settings.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'The privacy of this connection can\'t be verified — treat it as not private. The connection was accepted, but nothing has come back for a minute. It could be the path or the wallet server — the wallet can\'t tell which. It keeps trying; if it doesn\'t clear, try another server or check your app\'s network settings.';

  @override
  String get walletTransportExplainUnverified =>
      'The privacy of this connection can\'t be verified — treat it as not private.';

  @override
  String get walletTransportExplainHostProxy =>
      'Wallet traffic is routed through this app\'s privacy transport, which hides your IP address from the server.';

  @override
  String get walletOnboardingWelcomeTitle => 'Set up your wallet';

  @override
  String get walletOnboardingWelcomeBody =>
      'Create a new wallet to receive and hold ZEC. We\'ll generate a recovery phrase and walk you through backing it up before any funds can arrive — so nothing is ever at risk without a backup.';

  @override
  String get walletCreateButton => 'Create a new wallet';

  @override
  String get walletRestoreButton => 'Restore from a recovery phrase';

  @override
  String get walletWatchOnlyButton => 'Watch a wallet (view-only)';

  @override
  String get walletWatchOnlyTitle => 'Watch a wallet';

  @override
  String get walletWatchOnlyBody =>
      'Paste a viewing key to watch a wallet without its spending keys. You\'ll see its balance and history, but you won\'t be able to send funds. Pick the wallet\'s approximate start date so we know how far back to look.';

  @override
  String get walletWatchOnlyKeyLabel => 'Viewing key';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => 'Scan a viewing key QR code';

  @override
  String get walletWatchOnlyScanTitle => 'Scan viewing key';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Point your camera at the viewing key QR code.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Camera unavailable. Paste the key manually instead.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Paste key instead';

  @override
  String get walletWatchOnlyScanHint =>
      'Or tap the scan button to read a viewing key QR code.';

  @override
  String get walletWatchOnlyScanFilled => 'Viewing key scanned.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Wallet start date';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Scanning from $date on — funds received before then won\'t appear. Older wallet? Pick an earlier date.';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'Pick the wallet\'s start date';

  @override
  String get walletWatchOnlyBirthdayChange => 'Change date';

  @override
  String get walletWatchOnlySubmit => 'Watch this wallet';

  @override
  String get walletWatchOnlyBack => 'Back';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'That doesn\'t look like a valid viewing key. Check it and try again.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'That viewing key is for a different network. It can\'t be used here.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'A wallet already exists on this device. Go back and open it instead.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'That start date is too recent. Pick an earlier date.';

  @override
  String get walletRestoreTitle => 'Restore your wallet';

  @override
  String get walletRestoreBody =>
      'Enter your recovery phrase to restore your wallet — type or paste the words in order, separated by spaces. Standard phrases only: if your wallet used an extra passphrase (a \"25th word\"), this app can\'t restore it yet — you\'d see an empty wallet, not an error.';

  @override
  String get walletRestorePhraseHint => 'word one  word two  word three  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count words',
      one: '1 word',
      zero: 'No words yet',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'recovery phrases have 12, 15, 18, 21, or 24 words';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count words aren\'t recovery words — fix the highlighted ones',
      one: '1 word isn\'t a recovery word — fix the highlighted one',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'word $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'word $index: not a recovery word';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Remove word $index';
  }

  @override
  String get walletRestoreSubmit => 'Restore wallet';

  @override
  String get walletRestoreBack => 'Back';

  @override
  String get walletRestoreBirthdayTitle => 'How far back to scan';

  @override
  String get walletRestoreBirthdayNone =>
      'We\'ll scan your whole history — slower, but nothing is missed.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Scanning from $date on — funds received before then won\'t appear. Older wallet? Pick an earlier date, or Scan all history.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Pick a date';

  @override
  String get walletRestoreBirthdayChange => 'Change date';

  @override
  String get walletRestoreBirthdayClear => 'Scan all history';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'Word $index isn\'t a recovery word. Check your phrase for typos, then try again.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'That recovery phrase isn\'t valid. Check the words and their order, then try again.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'That phrase doesn\'t match the wallet on this device. Double-check it and try again.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'A wallet already exists on this device. Go back to open it.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'That date is too recent. Pick an earlier date, or scan everything.';

  @override
  String get walletGeneratingLabel => 'Creating your wallet…';

  @override
  String get walletOpeningLabel => 'Opening your wallet…';

  @override
  String get walletBackupTitle => 'Back up your recovery phrase';

  @override
  String get walletBackupBody =>
      'These words are the ONLY way to recover your wallet and funds. Write them down in order and keep them somewhere safe and private. Never share them or store them online — anyone with these words can take your funds.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Screenshots are turned off on this screen.';

  @override
  String get walletBackupSecureNoteOther =>
      'Make sure no one can see your screen.';

  @override
  String get walletBackupReveal => 'Reveal recovery phrase';

  @override
  String get walletBackupRevealing => 'Preparing your recovery phrase…';

  @override
  String get walletBackupRevealFailed =>
      'Couldn\'t show your recovery phrase right now. Make sure your device is unlocked, then try again.';

  @override
  String get walletBackupRetryReveal => 'Try again';

  @override
  String get walletBackupReauthFailed =>
      'Couldn\'t verify it\'s you. Please try again.';

  @override
  String get walletBackupConfirmCheckbox =>
      'I\'ve written down my recovery phrase and stored it safely.';

  @override
  String get walletBackupContinue => 'Continue';

  @override
  String get walletBackupSaveFailed =>
      'Couldn\'t save your confirmation. Please try again.';

  @override
  String get walletBackupStartOver => 'Start over';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Start over without this wallet?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'This deletes this wallet from the device and returns you to the start. Nothing can be deposited through this app before setup is finished.\n\nIf this wallet ever held funds — or was restored from a recovery phrase — only that phrase can bring it back.';

  @override
  String get walletBackupStartOverConfirm => 'Delete and start over';

  @override
  String get walletBackupStartOverKeep => 'Keep this wallet';

  @override
  String get walletBackupSectionTitle => 'Recovery phrase';

  @override
  String get walletBackupTileTitle => 'Back up your recovery phrase';

  @override
  String get walletBackupTileSubtitle =>
      'Show the words that can recover your wallet and funds.';

  @override
  String get walletBackupScreenTitle => 'Recovery phrase';

  @override
  String get walletBackupDone => 'Done';

  @override
  String get walletBackupManagedTitle => 'No separate recovery phrase';

  @override
  String get walletBackupManagedBody =>
      'This wallet was set up using your account from the app that installed it, so it has no recovery phrase of its own. Your funds are recovered together with that account — use its backup to keep them safe.';

  @override
  String get walletExportViewingKeyTitle => 'Export viewing key';

  @override
  String get walletExportViewingKeyTileTitle => 'Export viewing key';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Share a view-only copy of your wallet — it can see your history but cannot spend.';

  @override
  String get walletExportViewingKeyWarning =>
      'This key lets whoever holds it see everything this wallet has ever received and sent — and everything it will in the future. It cannot spend your funds and cannot recover your wallet. Share it only with someone you trust to see your full history, such as an accountant or your own second device. The only way to un-share it later is to move your funds to a new wallet.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'This key lets whoever holds it see everything this wallet has ever received and sent — and everything it will in the future. It cannot spend any funds and cannot recover the wallet. Share it only with someone you trust to see your full history, such as an accountant or your own second device. Once shared, it cannot be un-shared.';

  @override
  String get walletExportViewingKeyReveal => 'Show viewing key';

  @override
  String get walletExportViewingKeyRetry => 'Try again';

  @override
  String get walletExportViewingKeyRevealing => 'Preparing your viewing key…';

  @override
  String get walletExportViewingKeyFailed =>
      'Couldn\'t show your viewing key right now. Try again in a moment.';

  @override
  String get walletExportViewingKeyQrLabel => 'Viewing key QR code';

  @override
  String get walletExportViewingKeyCopy => 'Copy viewing key';

  @override
  String get walletExportViewingKeyCopied => 'Viewing key copied';

  @override
  String get walletExportViewingKeyDone => 'Done';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Screenshots are turned off on this screen.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Make sure no one can see your screen.';

  @override
  String get walletWatchOnlySectionTitle => 'About this watch-only wallet';

  @override
  String get walletWatchOnlyAboutBody =>
      'This is a watch-only wallet. It was set up from a viewing key, so it can see your balance and history but holds no spending keys — there is nothing to back up here, and it cannot send funds.';

  @override
  String get walletWatchOnlyBadge => 'Watch-only';

  @override
  String get walletOnboardingFailedTitle => 'Wallet setup couldn\'t finish';

  @override
  String get walletOnboardingRetry => 'Try again';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'Your phone\'s secure storage isn\'t responding. Unlock your device and try again. If this keeps happening, restart your phone.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'This wallet is open in another window or app, or is still finishing a previous operation. Close any other window using it — or wait a moment — then try again.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'This wallet\'s secure key is no longer available, so it can\'t be opened on this device. Your funds are safe — restore from your recovery phrase to recover them.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Restore from recovery phrase';

  @override
  String get walletOnboardingRecoverConfirmTitle => 'Restore this wallet?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Make sure you have your recovery phrase before continuing — you\'ll need it on the next screen to recover your funds. Your funds are safe on the blockchain and controlled by that phrase. This removes the unreadable wallet data from this device so it can be rebuilt.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Cancel';

  @override
  String get walletOnboardingFailedStorageFull =>
      'There isn\'t enough free space to set up your wallet. Free up some space and try again.';

  @override
  String get walletOnboardingFailedNoVault =>
      'This device has no secure key store, so the wallet can\'t protect your recovery phrase here.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Couldn\'t reach the network during setup. Check your connection and try again.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'Wallet setup didn\'t finish. Try again to complete it — nothing was lost.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Something went wrong setting up your wallet. Try again.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'This app\'s wallet setup is misconfigured, so the wallet can\'t start. Retrying won\'t help — please report this to the app\'s developer. Your funds are not affected.';

  @override
  String get walletSendButton => 'Send';

  @override
  String get walletSendSyncNotRunning =>
      'Syncing isn\'t running — your spendable balance can\'t update';

  @override
  String get walletSendWaitingForFunds =>
      'Still syncing — you can send once you have a spendable balance';

  @override
  String get walletSendNoSpendableYet => 'No spendable balance yet';

  @override
  String get walletSendSyncUnavailable => 'You can send once syncing resumes';

  @override
  String get walletSendTitle => 'Send';

  @override
  String get walletSendUnavailable =>
      'Your wallet isn\'t ready right now. Go back and try again.';

  @override
  String get walletSendWatchOnly =>
      'This is a watch-only wallet. It can show balances and receive payments, but it holds no spending keys — so it can\'t send.';

  @override
  String get walletSendExpiredTitle => 'This payment request expired';

  @override
  String get walletSendExpiredBody =>
      'The send screen took more than five seconds to open, so the app was told that nothing was sent. That answer is final: this request can\'t be paid from here. To pay, start again from the app.';

  @override
  String get walletSendFaultWatchOnly =>
      'This is a watch-only wallet — it holds no spending keys, so it can\'t send.';

  @override
  String walletSendAvailable(String amount) {
    return 'Available to send: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Available to send: $amount ZEC — your balance is still catching up';
  }

  @override
  String get walletSendRecipientLabel => 'Recipient address';

  @override
  String get walletSendRecipientHint =>
      'Zcash address (starts with u, z, or t)';

  @override
  String get walletSendRecipientLocked => 'Recipient can\'t be changed here';

  @override
  String get walletSendAmountLabel => 'Amount (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Memo (optional)';

  @override
  String get walletSendMemoHint =>
      'Only delivered to shielded (private) recipients';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Memos need a shielded recipient. This public address can\'t receive one.';

  @override
  String get walletSendMemoMachineDisabled =>
      'This payment already carries a reference from the app, so it can\'t also take a written memo.';

  @override
  String get walletSendMachineMemoTitle => 'The app is attaching a reference';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'It says this is for: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'It stays with the transaction and can\'t be removed later. The wallet can\'t check what it contains.';

  @override
  String get walletSendRecipientShielded => 'Shielded · private';

  @override
  String get walletSendRecipientTransparent => 'Public';

  @override
  String get walletSendRecipientInvalid =>
      'This doesn\'t look like a valid Zcash address.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'This address is for a different Zcash network.';

  @override
  String get walletSendReviewButton => 'Review payment';

  @override
  String get walletSendQueueButton => 'Queue to send later';

  @override
  String get walletSendQueueHint =>
      'A queued payment waits under Saved & pending, where you can send it or cancel it. Its network fee is worked out when it\'s sent.';

  @override
  String get walletSendPreparing => 'Preparing your payment…';

  @override
  String get walletSendSubmitting => 'Sending…';

  @override
  String get walletSendQueuing => 'Queuing…';

  @override
  String get walletSendReviewTitle => 'Confirm payment';

  @override
  String get walletSendTotalLabel => 'Total';

  @override
  String get walletSendFeeLabel => 'Network fee';

  @override
  String get walletSendChangeLabel => 'Change returned';

  @override
  String get walletSendDeshieldTitle => 'This payment is not private';

  @override
  String get walletSendDeshieldBody =>
      'It sends to a public address, so the amount and recipient will be publicly visible on the Zcash blockchain.';

  @override
  String get walletSendPublicAckLabel =>
      'I understand this payment will be public.';

  @override
  String get walletSendConfirmButton => 'Send now';

  @override
  String get walletSendBackButton => 'Back';

  @override
  String get walletSendSelfSendNote =>
      'You\'re sending to your own wallet. The network fee still applies.';

  @override
  String get walletSendLargeConfirmTitle => 'Send a large amount?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'This is almost your entire balance. A sent payment can\'t be reversed.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'This is a large payment. A sent payment can\'t be reversed.';

  @override
  String get walletSendLargeConfirmBoth =>
      'This is a large payment — almost your entire balance. A sent payment can\'t be reversed.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Send $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Go back';

  @override
  String get walletSendSentTitle => 'Payment sent';

  @override
  String get walletSendSentBody =>
      'Your payment has been broadcast to the network.';

  @override
  String get walletSendSavedTitle => 'Saved — we\'ll finish sending';

  @override
  String get walletSendSavedBody =>
      'Your payment couldn\'t go out just now, so it\'s saved and your wallet will send it on a later sync. Nothing is lost.';

  @override
  String get walletSendKeptTitle => 'Saved';

  @override
  String get walletSendKeptBody =>
      'Your wallet has kept this transaction and hasn\'t promised to send it by itself. Check Activity to see where it stands.';

  @override
  String get walletSendPartialBody =>
      'Part of your payment went out; your wallet will complete the rest on a later sync. Nothing is lost.';

  @override
  String get walletSendInMotionTitle => 'Payment in progress';

  @override
  String get walletSendInMotionBody =>
      'Your payment has started and is moving through a one-time address your wallet controls. Don\'t send it again. If it doesn\'t finish, you can recover the funds from your wallet screen.';

  @override
  String get walletSendAlreadyTitle => 'Already submitted';

  @override
  String get walletSendAlreadyBody =>
      'This payment was already submitted — it won\'t be sent twice.';

  @override
  String get walletSendFailedTitle => 'Couldn\'t complete payment';

  @override
  String get walletSendFailedBody =>
      'Something went wrong completing this payment and nothing was sent. You can try again.';

  @override
  String get walletSendTryAgain => 'Try again';

  @override
  String get walletSendDone => 'Done';

  @override
  String get walletSendAnother => 'Send another';

  @override
  String get walletSendQueuedTitle => 'Queued to send';

  @override
  String get walletSendQueuedBody =>
      'This payment is saved. You\'ll find it under Saved & pending, where you can send it now or cancel it.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Not enough spendable balance — you have $available ZEC and this needs $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'The Zcash network was upgraded and this app needs an update before it can send. Your funds are safe.';

  @override
  String get walletSyncUpToDateLimited =>
      'Caught up as far as this version can read';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'The Zcash network was upgraded. This version has scanned everything it can read, but newer blocks may hold funds it cannot show yet, and memos on recent payments are unavailable. Update the app to see everything.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Caught up, but this server isn\'t serving every pool';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'This server is refusing, withholding or misreporting one of Zcash\'s shielded pools. Funds received in that pool can\'t be spent through it, and the balance shown is a floor. Switch to another server to use them — this isn\'t a connection problem.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: this server refuses to serve it';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: this server is withholding part of it';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: this server is misreporting it';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: this server\'s service for it is unknown';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Caught up with this server, but it\'s behind the network';

  @override
  String get walletSyncExplainEndpointBehind =>
      'This server\'s copy of the chain stops at a block the network passed before this version of the app was built, so your balance is only current as of that block. New payments to you may not show yet, and a payment sent from here may not go through. Switch to another server to catch up — this isn\'t a connection problem.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Waiting for an app update — your funds are safe and nothing has been sent.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Waiting for a server that reports the network version — switch servers. Your funds are safe and nothing has been sent.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Waiting for a server that reports the network version. If this device\'s date and time are wrong, fix them first — then switch servers. Your funds are safe and nothing has been sent.';

  @override
  String get walletSyncUnverified =>
      'Caught up, but this server isn\'t reporting the network version';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'Sending still works for about $hours more hours — then switch servers.',
      one: 'Sending still works for about 1 more hour — then switch servers.',
      zero: 'Sending still works for less than an hour — then switch servers.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'Sending still works for about $blocks more blocks — then switch servers.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'This server hasn\'t reported the network version for $blocks blocks, so this app can\'t confirm it\'s safe to send. Switch to another server.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'This server hasn\'t reported the network version for a day, so this app can\'t confirm it\'s safe to send. If this device\'s date and time are wrong, fix them first — then switch to a server that reports the network version.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'This server has never reported the network version, so this app can\'t confirm it\'s safe to send. Switch to another server.';

  @override
  String get walletSyncExplainUnverified =>
      'This server isn\'t saying which version of the Zcash network it\'s on, so this app can\'t confirm that a payment it signs will be accepted. Your balance is current. Switch to another server — this isn\'t a connection problem.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'This server isn\'t saying which version of the Zcash network it\'s on, so this app can\'t confirm that a payment it signs will be accepted. It has also kept serving blocks this wallet then had to undo, so your balance may not be current. Switch to another server — this isn\'t a connection problem.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'This server also keeps serving blocks this wallet then has to undo — switch servers.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Your balance is still catching up — more may become available as the wallet syncs.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC is still arriving and will be spendable once the wallet catches up.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Enter an amount to send.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Enter the amount as a number, for example 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'ZEC has at most 8 decimal places.';

  @override
  String get walletSendFaultAmountNotPositive =>
      'Enter an amount greater than zero.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'That amount is larger than the total ZEC supply.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'This app currently limits sends to $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'That doesn\'t look like a valid Zcash address for this network. Check it and try again.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'This recipient can\'t receive a memo. Remove the memo, or send to a shielded (private) address.';

  @override
  String get walletSendFaultMemoTooLong =>
      'Your memo is too long. Shorten it and try again.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'That memo can\'t be sent. Remove it and try again.';

  @override
  String get walletSendFaultMemoConflict =>
      'Couldn\'t send this payment — the app attached two notes to it. Nothing was sent.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'That address is for a different network.';

  @override
  String get walletSendFaultUriInvalid =>
      'Couldn\'t build this payment. Check the address and amount.';

  @override
  String get walletSendFaultNotSynced =>
      'Your wallet isn\'t synced far enough yet. Wait for sync to catch up, or queue this to send later.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Your wallet isn\'t synced far enough yet. Wait for sync to catch up.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Your wallet isn\'t synced far enough yet, and syncing isn\'t running right now. Check the sync status on the wallet screen.';

  @override
  String get walletSendFaultAmountsExpired =>
      'The amounts expired while you were reviewing. Please review the payment again.';

  @override
  String get walletSendFaultQueueFull =>
      'Too many sends are waiting to go out. Let them send first, then try again.';

  @override
  String get walletSendFaultWalletBusy =>
      'The wallet is busy right now. Try again in a moment.';

  @override
  String get walletSendFaultStorageFull =>
      'There isn\'t enough free space to complete this send. Free up some space and try again.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Too many one-time addresses are in use right now. Some may free up as transfers confirm, but this may not clear on its own. Your funds are safe.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Couldn\'t prepare this payment. Check the details and try again.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Couldn\'t prepare this payment just now. Try again in a moment.';

  @override
  String get walletSwapButton => 'Swap';

  @override
  String get walletSwapTitle => 'Swap ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'Your wallet isn\'t ready right now. Go back and try again.';

  @override
  String get walletSwapUnavailableOff => 'Swap isn\'t available right now.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'This wallet is view-only — it can\'t swap.';

  @override
  String get walletSwapDone => 'Done';

  @override
  String get walletSwapBackToWallet => 'Back to wallet';

  @override
  String walletSwapAvailable(String amount) {
    return 'Available to swap: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Available to swap: $amount ZEC — your balance is still catching up';
  }

  @override
  String get walletSwapAssetLabel => 'Receive asset';

  @override
  String get walletSwapAmountLabel => 'Amount to swap (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Destination address';

  @override
  String get walletSwapDestinationHint =>
      'Your receiving address on the destination chain';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Your $chain receiving address';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'A $chain address — where your swapped asset is sent. Double-check the chain is right.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Scan a destination-address QR code';

  @override
  String get walletSwapTargetAssetHint => 'Select an asset to receive';

  @override
  String get walletSwapQuoteButton => 'Get quote';

  @override
  String get walletSwapQuoting => 'Getting a quote…';

  @override
  String get walletSwapExecuting => 'Starting your swap…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Still working — the swap is starting. This can take up to a minute.';

  @override
  String get walletSwapReviewTitle => 'Confirm swap';

  @override
  String get walletSwapYouSendLabel => 'You send';

  @override
  String get walletSwapYouReceiveLabel => 'You receive at least';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Network fee';

  @override
  String get walletSwapNetworkFeeValue => 'Added when the deposit is sent';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Quote valid for about $time — confirm before it expires.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Quote valid for less than a minute — confirm before it expires.';

  @override
  String get walletSwapQuoteExpired =>
      'This quote has expired. Go back and get a new one — its rate is no longer guaranteed, and sending now risks a refund.';

  @override
  String get walletCountdownUnderMinute => 'less than a minute';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds s';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '${hours}h ${minutes}m';
  }

  @override
  String get walletSwapDeshieldTitle => 'This swap is not private';

  @override
  String get walletSwapDeshieldBody =>
      'Swapping out de-shields your ZEC — the deposit is a public transaction, and the provider\'s side is public on its network.';

  @override
  String get walletSwapDiscloseTitle => 'What the swap provider will see';

  @override
  String get walletSwapDiscloseAmounts => 'The amounts on both sides';

  @override
  String get walletSwapDiscloseCrossLink =>
      'That this ZEC and the asset you receive are one swap';

  @override
  String get walletSwapDiscloseDestination => 'Your destination address';

  @override
  String get walletSwapDiscloseSource => 'Your source address';

  @override
  String get walletSwapDiscloseIp =>
      'Your IP address (unless you route through Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'Other details of this swap';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'The provider\'s own transactions are public on its network';

  @override
  String get walletSwapAckLabel =>
      'I understand the provider will see the information above.';

  @override
  String get walletSwapConfirmButton => 'Start swap';

  @override
  String get walletSwapBackButton => 'Back';

  @override
  String get walletSwapStatusPendingTitle => 'Swap started';

  @override
  String get walletSwapStatusCheckingTitle => 'Checking swap status…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Your wallet is sending the ZEC deposit to the provider. If you\'re briefly offline it\'s sent automatically when you\'re back — but the sending window is short, and if it closes first the swap simply ends and nothing is exchanged. Your ZEC stays yours, and it can take up to an hour to show as spendable again.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'Waiting for your deposit to arrive. If you haven\'t sent the funds from your other wallet yet, send them before the quote expires.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'This swap is still waiting for its deposit. The deposit instructions aren\'t available on this device anymore — if you already sent the funds, they\'ll be detected; if you haven\'t, let this swap expire and start a new one.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'The deposit window ends $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'The deposit window has passed. If the deposit wasn\'t sent in time, the swap ends and your ZEC stays in your wallet.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'The deposit window has passed. If you haven\'t sent your deposit, this swap simply ends — get a fresh quote when you\'re ready.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Swaps in progress',
      one: 'Swap in progress',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'Your ZEC is on its way to the swap provider.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'Waiting for your deposit to reach the swap provider.';

  @override
  String get walletSwapInFlightRowGeneric => 'A swap is in progress.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'The deposit window has passed — check this swap\'s status.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'This swap hasn\'t reached a confirmed outcome here yet — open it to check. Any ZEC coming back to this wallet shows up in your balance after a sync.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'This swap hasn\'t reached a confirmed outcome here yet — open it to check. Any ZEC it delivers to this wallet shows up in your balance after a sync.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Swap completed.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Swap refunded.';

  @override
  String get walletSwapRowOutcomeFailed => 'Swap didn\'t complete.';

  @override
  String get walletSwapRemove => 'Remove';

  @override
  String get walletSwapRemoveTitle => 'Remove this swap from the list?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'This only removes the swap from this list — it doesn\'t cancel the swap, and this wallet will stop watching for its refund. ZEC refunded later still belongs to this wallet; a full rescan can find it.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'This only removes the swap from this list — it doesn\'t cancel the swap, and this wallet will stop watching for its incoming ZEC. ZEC delivered later still belongs to this wallet; a full rescan can find it. If the swap is refunded instead, the refund goes back in the coin you sent, outside this wallet.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'This only removes the swap from this list — it doesn\'t cancel the swap, and this wallet will stop watching for ZEC still arriving from it. ZEC that arrives later still belongs to this wallet; a full rescan can find it.';

  @override
  String get walletSwapRemoveBodyDone =>
      'This removes the finished swap from the list.';

  @override
  String get walletSwapRemoveCancel => 'Cancel';

  @override
  String get walletSwapRemoveConfirm => 'Remove';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Started $time';
  }

  @override
  String get walletSwapViewSwap => 'View swap';

  @override
  String get walletSwapsInFlightError =>
      'Couldn\'t load your swaps in progress right now.';

  @override
  String get walletSwapsInFlightRetry => 'Try again';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Trying…';

  @override
  String get walletSwapStartAnother => 'Start another swap';

  @override
  String get walletSwapStatusUnderTitle => 'Waiting for the full deposit';

  @override
  String get walletSwapStatusUnderBody =>
      'Part of the deposit has arrived. The rest is completing, or the provider will refund.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Part of your deposit has arrived. Send the missing amount before the deadline, or the provider refunds what arrived.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Received $received; $missing still missing. The deposit window ends $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Deposit received';

  @override
  String get walletSwapStatusDetectedBody =>
      'The provider received your deposit and will process the swap.';

  @override
  String get walletSwapStatusProcessingTitle => 'Processing your swap';

  @override
  String get walletSwapStatusProcessingBody =>
      'The provider is completing your swap.';

  @override
  String get walletSwapStatusSuccessTitle => 'Swap complete';

  @override
  String get walletSwapStatusSuccessBody => 'Your swap finished successfully.';

  @override
  String get walletSwapStatusRefundedTitle => 'Swap refunded';

  @override
  String get walletSwapStatusRefundedBody =>
      'The swap didn\'t complete, so the provider sent the funds back to your refund address.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'The swap didn\'t complete, so the provider sent your ZEC back to this wallet. It arrives as unshielded funds and shows up in your balance after the wallet next syncs — this can take a little while.';

  @override
  String get walletSwapStatusFailedTitle => 'Swap failed';

  @override
  String get walletSwapStatusFailedBody =>
      'The swap couldn\'t be completed. Any deposited funds settle or refund on the provider\'s side.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Swap not found';

  @override
  String get walletSwapStatusNotFoundBody =>
      'The provider no longer has a record of this swap — it most likely expired. If a deposit was made, the provider should refund it to the refund address. The swap stays in your list, and this wallet keeps watching for its ZEC in case it still arrives — you can remove it from the list anytime.';

  @override
  String get walletSwapStatusUnknownTitle => 'Status unavailable';

  @override
  String get walletSwapStatusUnknownBody =>
      'We can\'t read this swap\'s status right now.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'Tracking unavailable';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'Swap is turned off, so we can\'t track this here. Any funds settle or refund on the provider\'s side.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'Swap is turned off here, so this swap can\'t be tracked right now. If it was refunded, the ZEC comes back to this wallet — it shows up in your balance after swap is turned back on and the wallet syncs.';

  @override
  String get walletSwapTrackingError => 'We couldn\'t track this swap.';

  @override
  String get walletSwapTrackingErrorBody =>
      'We couldn\'t open tracking for this swap. The swap itself may still be going ahead — any deposited funds settle or refund on the provider\'s side.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Enter the address where you want to receive the swapped asset.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'That destination address isn\'t valid for this asset. Check it and try again.';

  @override
  String get walletSwapFaultExpired =>
      'This quote expired. Get a fresh quote to continue.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'The provider\'s price moved outside your limit, so the swap was stopped before anything moved. Try again.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'The slippage limit is too high for a safe swap. Try again.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'The swap provider is unavailable right now. Try again in a moment.';

  @override
  String get walletSwapFaultConnection =>
      'Couldn\'t reach the swap service. Please check your internet connection and try again.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'The swap provider returned an unexpected response, so the swap was stopped. Try again.';

  @override
  String get walletSwapFaultSwapOff => 'Swap is turned off right now.';

  @override
  String get walletSwapFaultDepositFailed =>
      'We couldn\'t send your deposit, so nothing left your wallet. Get a fresh quote to try again.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'A swap is already in progress. You can start a new one after it fully settles or its quote expires — this can take a while.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'This wallet can\'t set up a refund address yet — that usually just means the first sync hasn\'t finished. Wait for the sync to complete, then try again.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'This wallet can\'t set up a receiving address for this swap yet — that usually just means the first sync hasn\'t finished. Wait for the sync to complete, then try again.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'The swap couldn\'t start in time — the connection may be slow, or the wallet was busy. Get a new quote and try again.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'The wallet is busy for a moment. Try again.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'This quote doesn\'t match the one your wallet issued, so nothing was sent. Get a fresh quote and try again.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'This swap needs about $needed ZEC including the network fee, but only $spendable ZEC is spendable right now.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'This app currently limits swaps to $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'This swap needs about $needed ZEC including the network fee, but only $spendable ZEC is spendable right now. Your balance is still catching up — more may become spendable soon.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'The wallet couldn\'t safely record this swap, so nothing moved. Try again.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'That swap request couldn\'t be processed. Get a fresh quote and try again.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Couldn\'t get a swap quote. Check the details and try again.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Your wallet isn\'t ready right now. Go back and try again.';

  @override
  String get walletSwapDirectionBuy => 'Buy ZEC';

  @override
  String get walletSwapDirectionSell => 'Sell ZEC';

  @override
  String get walletSwapRefundLabel => 'Your refund address';

  @override
  String get walletSwapRefundHint =>
      'Where your coins return if the swap fails';

  @override
  String get walletSwapRefundHelper =>
      'On the chain you\'re sending from — not a Zcash address.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Your $chain refund address';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'A $chain address — where your coins return if the swap fails. Not a Zcash address.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'About your refund address';

  @override
  String get walletSwapRefundInfoBody =>
      'If the swap can\'t complete, the provider sends your coins back to this address on the chain you paid from. Enter an address you control — the wallet can\'t check a foreign address for you, so verify it carefully.';

  @override
  String get walletSwapRefundScanTooltip => 'Scan a refund-address QR code';

  @override
  String get walletSwapScanTitle => 'Scan address';

  @override
  String get walletSwapScanInstruction =>
      'Point your camera at the address QR code.';

  @override
  String get walletSwapScanManualEntry => 'Enter manually';

  @override
  String get walletSwapScanCancel => 'Cancel';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Camera unavailable. Enter the address manually below.';

  @override
  String get walletSwapSourceAssetLabel => 'Asset to swap from';

  @override
  String get walletSwapSourceAssetHint => 'Select an asset';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Amount to send ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Amount to send';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol on $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Choose an asset to swap from';

  @override
  String get walletSwapPickerTitleReceive => 'Choose an asset to receive';

  @override
  String get walletSwapPickerStale =>
      'Couldn\'t refresh the asset list — showing the last known list.';

  @override
  String get walletSwapPickerEmpty =>
      'No assets are available to swap right now. Try again later.';

  @override
  String get walletSwapPickerSearchHint => 'Search by name or chain';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'No assets match \"$query\".';
  }

  @override
  String get walletSwapPickerError =>
      'Couldn\'t load the asset list. Check your connection and try again.';

  @override
  String get walletSwapPickerRetry => 'Try again';

  @override
  String get walletSwapSlippageLabel => 'Slippage tolerance';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Custom';

  @override
  String get walletSwapSlippageCustomLabel => 'Custom slippage';

  @override
  String get walletSwapSlippageMayFail =>
      'Very low — the swap may fail if the price moves.';

  @override
  String get walletSwapSlippageNormal => 'A safe tolerance.';

  @override
  String get walletSwapSlippageRisky =>
      'High — you could receive noticeably less than quoted.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Too high — the swap will be rejected. Lower it to 10% or less.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'You\'ll receive at least $zec ZEC — your $slippage% slippage floor. The final amount won\'t drop below this.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'You receive ZEC to your own address';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Until you shield it — one tap, nudged on arrival — the received amount is briefly public and visible on-chain. A small delivery may stay public until it accumulates.';

  @override
  String get walletSwapRefundVerifyTitle => 'Verify your refund address';

  @override
  String get walletSwapRefundVerifyBody =>
      'Check it character by character — this is where your coins return if the swap fails. The wallet can\'t verify a foreign address for you.';

  @override
  String get walletSwapRefundVerifyAck =>
      'I\'ve checked my refund address is correct.';

  @override
  String get walletSwapPayoutVerifyTitle => 'Verify your receiving address';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Check it character by character — this is where you\'ll receive $asset. The wallet can\'t verify a foreign address for you.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'I\'ve checked my receiving address is correct.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'Swap is turned off here. Any ZEC already on its way will appear in your wallet after your next sync.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Enter the amount you want to swap.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Enter your refund address on the source chain.';

  @override
  String get walletSwapDepositTitle => 'Send your payment';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Send exactly $amount $asset on $chain to the address below.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Send the exact amount. Sending less, or sending after the window closes, means the provider refunds you to your refund address.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Deposit window: $time left';
  }

  @override
  String get walletSwapDepositExpired =>
      'This deposit window has closed. Don\'t send funds now — start a new swap. If you already sent, the provider should refund to your refund address.';

  @override
  String get walletSwapDepositQrLabel => 'QR code of the deposit address';

  @override
  String get walletSwapDepositAddressLabel => 'Deposit address';

  @override
  String get walletSwapDepositCopy => 'Copy deposit address';

  @override
  String get walletSwapDepositCopied => 'Deposit address copied';

  @override
  String get walletSwapDepositMemoRequired => 'This deposit needs a memo / tag';

  @override
  String get walletSwapDepositMemoWarning =>
      'You MUST include this exact memo with your deposit. Sending without it — or with the wrong memo — can permanently lose your funds.';

  @override
  String get walletSwapDepositMemoLabel => 'Deposit memo / tag';

  @override
  String get walletSwapDepositMemoCopy => 'Copy memo';

  @override
  String get walletSwapDepositMemoCopied => 'Memo copied';

  @override
  String get walletSwapDepositSent => 'I\'ve sent the funds';

  @override
  String get walletSwapDepositBackTitle => 'Leave this screen?';

  @override
  String get walletSwapDepositBackBody =>
      'This won\'t cancel your swap — it continues in the background. But you\'ll need the deposit address to pay, so copy it first if you haven\'t.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'This won\'t cancel your swap — it continues in the background. The deposit window has closed, so don\'t send funds to the deposit address now. If you already sent, the provider should refund to your refund address.';

  @override
  String get walletSwapDepositBackStay => 'Stay';

  @override
  String get walletSwapDepositBackLeave => 'Leave';

  @override
  String get walletReceive => 'Receive';

  @override
  String get walletReceiveSubtitle =>
      'Share this address to receive ZEC. It\'s safe to share publicly.';

  @override
  String get walletReceiveCopy => 'Copy address';

  @override
  String get walletReceiveCopied => 'Address copied';

  @override
  String get walletReceiveUnavailable => 'Your wallet isn\'t ready yet.';

  @override
  String get walletReceiveError =>
      'We couldn\'t load your address. Please try again.';

  @override
  String get walletReceivePreparing => 'Preparing your address…';

  @override
  String get walletReceivePreparingHint =>
      'Your wallet prepares this address on your device — it can take a moment if the wallet is busy with other work.';

  @override
  String get walletReceiveRetry => 'Try again';

  @override
  String get walletReceiveQrLabel => 'QR code of your receive address';

  @override
  String get walletReceiveTypeShielded => 'Shielded';

  @override
  String get walletReceiveTypeTransparent => 'Public';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Share this public address to receive ZEC from a sender that can\'t pay a shielded address.';

  @override
  String get walletReceiveTransparentWarning =>
      'This is a public address: it\'s visible on-chain and links your payments if reused. Prefer your shielded address; shield these funds after receiving.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'QR code of your public receive address';

  @override
  String get walletReceiveFreshAddress => 'Use a fresh address';

  @override
  String get walletReceiveFreshCaption =>
      'Fresh address — can\'t be linked to your other addresses. Payments to it arrive in this wallet, and your earlier addresses keep working. It won\'t be shown here again — copy it now.';

  @override
  String get walletReceiveFreshError =>
      'Couldn\'t create a fresh address. Try again.';

  @override
  String get walletReceiveFreshBusy =>
      'The wallet is busy right now. Try the fresh address again in a moment.';

  @override
  String get walletReceiveShare => 'Share';

  @override
  String get walletReceiveRequestAmount => 'Request amount';

  @override
  String get walletReceiveRequestAmountLabel => 'Amount (optional)';

  @override
  String get walletReceiveFreshCopyNow =>
      'It won\'t be shown here again — copy it now.';

  @override
  String get walletSecurityMenuItem => 'Security…';

  @override
  String get securityTitle => 'Security';

  @override
  String get securityUnavailableBody =>
      'Wallet security settings are managed by this app, not by the wallet itself.';

  @override
  String get securityCustodySectionTitle => 'Key custody';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (hardware)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (hardware)';

  @override
  String get securityCustodyTierTee => 'Hardware keystore (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Software keystore';

  @override
  String get securityCustodyTierKeychain => 'Keychain (software-encrypted)';

  @override
  String get securityCustodyTierNone => 'No hardware keystore';

  @override
  String get securityCustodyTierUnknown => 'Unknown';

  @override
  String get securityCustodyHardwareKey =>
      'The key that locks this wallet is held in this device\'s secure hardware and is deleted with the wallet.';

  @override
  String get securityCustodyBestEffort =>
      'Deleting removes your keys best-effort; a brief forensic recovery window can remain until the device reclaims the storage. For full assurance, also use your device\'s Erase-All-Content.';

  @override
  String get securityCustodyProbeError =>
      'Couldn\'t read the custody status. Pull back and try again.';

  @override
  String get securityDeleteWalletButton => 'Delete wallet';

  @override
  String get securityDeleteWalletSubtitle =>
      'Delete this wallet and its key from this device. Your funds remain on-chain and are restorable from your recovery phrase.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Delete this wallet and its key from this device. It holds no spending keys, so there is nothing to back up — re-add it anytime with its viewing key.';

  @override
  String get securityDeleteDialogTitle => 'Delete this wallet?';

  @override
  String get securityDeleteDialogBody =>
      'This removes the wallet and its key from this device. Make sure you\'ve backed up your recovery phrase — it is the ONLY way to restore your funds.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'This removes the wallet and its key from this device. It holds no spending keys, so nothing needs backing up — you can re-add it later with its viewing key.';

  @override
  String get securityDeleteDialogConfirm => 'Delete';

  @override
  String get securityDeleteDialogCancel => 'Cancel';

  @override
  String get securityDeleteFailedSnack =>
      'Couldn\'t delete the wallet — your wallet is unchanged. Try again.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Finish the server switch first — it completes or stops within $seconds seconds. Then try deleting the wallet again.';
  }

  @override
  String get walletParkedTitle => 'Saved & pending';

  @override
  String get walletParkedSubtitle =>
      'These payments haven\'t been sent yet. Their amounts are still part of your balance.';

  @override
  String get walletParkedSubtitlePreparing =>
      'These payments haven\'t been sent yet. Their amounts are still part of your balance — except any your wallet is currently sending, which may already be set aside.';

  @override
  String get walletParkedCancel => 'Cancel';

  @override
  String get walletParkedPausedHint =>
      'Paused — this payment won\'t send on its own. Your funds are safe. Send it now, or cancel it.';

  @override
  String get walletParkedRetryStale =>
      'This payment isn\'t waiting anymore. Check your pending payments and activity.';

  @override
  String get walletParkedAlreadyInProgress =>
      'This payment is no longer waiting — your wallet may already be sending it. Check Saved & pending and your activity.';

  @override
  String get walletReclaimExplainer =>
      'One-time-address sends are stuck. You can reopen them — it moves a small amount between your own addresses and returns it.';

  @override
  String get walletReclaimButton => 'Reopen sending';

  @override
  String get walletReclaimInProgress => 'Reopening…';

  @override
  String get walletReclaimConfirmTitle => 'Reopen one-time-address sending?';

  @override
  String get walletReclaimConfirmBody =>
      'This moves a small amount between your own addresses to free up one-time-address sending, then returns it. It costs a couple of network fees. Once it confirms, recover the moved amount with Recover now.';

  @override
  String get walletReclaimConfirmCancel => 'Not now';

  @override
  String get walletReclaimConfirmAction => 'Reopen';

  @override
  String get walletReclaimStarted =>
      'Reopening started. Once it confirms, send the paused payment, then use Recover now to get the moved amount back.';

  @override
  String get walletReclaimNothing => 'Nothing to reopen right now.';

  @override
  String get walletReclaimNotBroadcast =>
      'Couldn\'t confirm it reached the network. It may still go through — wait a moment before trying again.';

  @override
  String get walletReclaimNeedsFunds =>
      'You need some shielded ZEC to reopen sending.';

  @override
  String get walletReclaimFailed =>
      'Couldn\'t reopen sending right now. Your funds are unchanged. Try again.';

  @override
  String get walletReclaimUnknown =>
      'Reopen finished. Check your one-time-address sends, and use Recover now to get any moved amount back.';

  @override
  String get walletParkedError =>
      'Couldn\'t load your pending payments right now.';

  @override
  String get walletParkedErrorRetry => 'Try again';

  @override
  String get walletParkedErrorRetryInProgress => 'Trying…';

  @override
  String get walletParkedCancelConfirmTitle => 'Cancel this pending payment?';

  @override
  String get walletParkedCancelConfirmBody =>
      'This discards the saved payment. It hasn\'t been sent, so nothing leaves your wallet — but this can\'t be undone.';

  @override
  String get walletParkedCancelConfirmKeep => 'Keep it';

  @override
  String get walletParkedCancelConfirmDiscard => 'Discard payment';

  @override
  String get walletParkedCancelDone => 'Pending payment cancelled.';

  @override
  String get walletParkedCancelAlreadySending =>
      'This payment may already be on its way — check your activity.';

  @override
  String get walletParkedCancelFailed =>
      'Couldn\'t cancel right now. Your payment is unchanged. Try again.';

  @override
  String get walletRecoverNow => 'Recover now';

  @override
  String get walletRecoverConfirmTitle => 'Recover to your shielded balance?';

  @override
  String get walletRecoverConfirmBody =>
      'This checks your one-time addresses and moves anything found into your private shielded balance. It\'s safe to run again any time.';

  @override
  String get walletRecoverConfirmCancel => 'Not now';

  @override
  String get walletRecoverConfirmAction => 'Recover';

  @override
  String get walletRecoverInProgress => 'Recovering…';

  @override
  String walletRecoverDone(String amount) {
    return 'Recovering $amount to your shielded balance.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'Recovering $amount — some funds still need another try.';
  }

  @override
  String get walletRecoverRetry =>
      'Some funds need another try — run recovery again.';

  @override
  String get walletRecoverTruncated =>
      'Not every one-time address was checked yet — run it again to check the rest.';

  @override
  String get walletRecoverNothing => 'Nothing to recover right now.';

  @override
  String get walletRecoverFailed =>
      'Couldn\'t recover right now. Your funds are unchanged. Try again.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount saved & pending · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Cancel the $amount payment saved $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount paused · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount preparing to send · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Your wallet is getting this payment ready — its amount may already be set aside. Your funds are safe. If it doesn\'t finish, it returns to the list on its own.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Your wallet is getting this payment ready — its amount may already be set aside. Your funds are safe, but it can only finish once your wallet is syncing again.';

  @override
  String get walletParkedSendNow => 'Send now';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Sending the $amount payment saved $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Send the $amount payment saved $time now';
  }

  @override
  String get walletParkedSendNowInProgress => 'Sending…';

  @override
  String get walletParkedAuthorizeSent => 'Sending your payment now.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Sending your payment now. If it doesn\'t go through, your wallet can only finish the payment once it\'s syncing again.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Not ready to send yet. Your payment is saved and unchanged.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Not ready to send yet. Your payment is saved and no longer paused — try Send now again later, or cancel it.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Couldn\'t send it right now. The payment is unchanged. Try again.';

  @override
  String get walletTransparentFundsMenuItem => 'Public funds…';

  @override
  String get walletTransparentFundsTitle => 'Public funds';

  @override
  String get walletTransparentFundsIntro =>
      'Public funds are publicly visible on the blockchain — the amount, the addresses, and the history of the coins.';

  @override
  String get walletExpertToggleLabel => 'Advanced: public funds';

  @override
  String get walletExpertToggleDescription =>
      'Show expert controls for holding public funds and turning automatic shielding off.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Show expert controls for holding public funds.';

  @override
  String get walletAutoShieldToggleLabel => 'Shield automatically';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'When your public balance reaches $minZec ZEC, it is moved into your shielded balance automatically. With this off, public funds stay publicly visible until you shield them yourself.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Couldn’t save the setting. Try again.';

  @override
  String get walletAutoShieldIncomplete =>
      'Automatic shielding didn’t complete — these funds are still publicly visible. You can shield them now.';

  @override
  String get walletSendPrivacyShielded =>
      'Shielded payment — the amount and recipient stay private on-chain.';

  @override
  String get walletSendPrivacyTransparent =>
      'Public payment — the amount and addresses are visible on the blockchain.';

  @override
  String get walletActivityPublicBadge => 'Publicly visible on-chain';

  @override
  String get walletShieldWalletEnded =>
      'The wallet session ended. Close and reopen to try again.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'New public funds are shielded into your private balance automatically once they reach $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'Automatic shielding is off — public funds stay publicly visible until you shield them.';

  @override
  String get walletMoveAutoShieldNote =>
      'Automatic shielding is on: after these funds arrive, they will be shielded back automatically (for another fee). To keep them public, first turn off automatic shielding under Public funds.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'After this move your public balance will be $amount ZEC — under the $floor ZEC needed to shield it back. It stays public until more arrives.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'You\'re moving to your own public address. This move stays on the public record permanently.';

  @override
  String get walletTxDetailVisibility => 'Visibility';

  @override
  String get walletTransparentFundsAutoDenied =>
      'Automatic shielding is paused for this session — it wasn’t approved. You can still shield manually.';

  @override
  String get walletDeepScanMenuItem => 'Check older swap addresses…';

  @override
  String get walletMenuSyncNotRunningHint =>
      'Syncing isn\'t running right now.';

  @override
  String get walletDeepScanTitle => 'Check older swap addresses';

  @override
  String get walletDeepScanBody =>
      'If you restored this wallet and it once used swaps a lot, money from its oldest swaps can take an extra step to find. This checks for it — anything found appears in your balance as your wallet syncs.';

  @override
  String get walletDeepScanCoverage =>
      'Your older swap addresses are checked up to here. If money from an old swap is still missing, check even deeper.';

  @override
  String get walletDeepScanCoveragePending =>
      'Still checking the current range — anything found appears in your balance. This can take a little while.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Checks for money from your wallet’s oldest swaps.';

  @override
  String get walletDeepScanCheckButton => 'Check older addresses';

  @override
  String get walletDeepScanCheckDeeperButton => 'Check even older addresses';

  @override
  String get walletDeepScanChecking => 'Checking…';

  @override
  String get walletDeepScanClose => 'Close';

  @override
  String get walletDeepScanTorHint =>
      'You’re not connected over Tor right now. For more privacy, consider waiting until Tor is active before checking.';

  @override
  String get walletDeepScanRescanBusy =>
      'You can check older swap addresses once the rescan finishes.';

  @override
  String get walletDeepScanRan =>
      'Checking older swap addresses — anything found will appear in your balance.';

  @override
  String get walletDeepScanFailed =>
      'Couldn’t start the check. Nothing changed — try again.';

  @override
  String get walletDeepScanSlow =>
      'This is taking longer than usual. If your older swap addresses were checked, anything found appears in your balance — check back shortly.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'Swap is turned off right now, so this can’t run. Try again when swap is available.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Still checking the last range — this can take up to a couple of days, but usually much less. It finishes on its own; check again later.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'We can’t confirm your connection privacy yet. For more privacy, check once Tor is active.';

  @override
  String get walletDeepScanBannerChecking =>
      'Still checking your older swap addresses — anything found appears in your balance.';

  @override
  String get walletRescanSwapPointer =>
      'Looking for money from an old swap? A rescan won’t find that — use “Check older swap addresses” instead.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Restored a wallet that used swaps?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'If this wallet had a very long swap history, money from its oldest swaps can take an extra step to find. Most wallets need nothing.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Check now';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Dismiss';

  @override
  String walletTorHostPath(String transport) {
    return 'Via your app\'s private path ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Via your app\'s private path ($transport); connections can be linked by the proxy';
  }

  @override
  String get walletTorHostOtherTransport => 'a private path';

  @override
  String get walletTorHostDirect =>
      'Not private (your app\'s direct connection)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Your saved server uses an unencrypted address, which your app\'s private path cannot carry — using $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'More about $label';
  }

  @override
  String get walletSendPaste => 'Paste';

  @override
  String get walletSendScanQr => 'Scan QR code';

  @override
  String get walletSendRecipientGetsLabel => 'Recipient gets';

  @override
  String get walletSwapDepositCopyAmount => 'Copy amount';

  @override
  String get walletSwapDepositAmountCopied => 'Amount copied';

  @override
  String get walletScanOpenSettings => 'Open settings';

  @override
  String get walletScanOpenSettingsFailed => 'Couldn\'t open settings.';

  @override
  String get walletSendLeaveTitle => 'Still sending';

  @override
  String get walletSendLeaveBody =>
      'Your payment keeps going if you leave. You\'ll see how it ended in your activity.';

  @override
  String get walletSendLeaveStay => 'Stay';

  @override
  String get walletSendLeaveConfirm => 'Leave';

  @override
  String get walletSheetLeaveBody =>
      'This keeps going if you leave. You\'ll see how it ended in your activity.';

  @override
  String get walletLoadingLabel => 'Loading';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'Check before shielding again';

  @override
  String get walletShieldUnknownBody =>
      'We couldn\'t confirm this shield. Check Activity before trying again.';

  @override
  String get walletMoveUnknownTitle => 'Check before moving again';

  @override
  String get walletMoveUnknownBody =>
      'We couldn\'t confirm this move. Check Activity before trying again.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
