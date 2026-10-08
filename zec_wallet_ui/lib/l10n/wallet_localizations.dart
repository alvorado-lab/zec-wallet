import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'wallet_localizations_ar.dart';
import 'wallet_localizations_de.dart';
import 'wallet_localizations_en.dart';
import 'wallet_localizations_es.dart';
import 'wallet_localizations_fi.dart';
import 'wallet_localizations_fr.dart';
import 'wallet_localizations_he.dart';
import 'wallet_localizations_it.dart';
import 'wallet_localizations_ja.dart';
import 'wallet_localizations_nb.dart';
import 'wallet_localizations_nl.dart';
import 'wallet_localizations_pl.dart';
import 'wallet_localizations_pt.dart';
import 'wallet_localizations_ru.dart';
import 'wallet_localizations_uk.dart';
import 'wallet_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of WalletLocalizations
/// returned by `WalletLocalizations.of(context)`.
///
/// Applications need to include `WalletLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/wallet_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: WalletLocalizations.localizationsDelegates,
///   supportedLocales: WalletLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the WalletLocalizations.supportedLocales
/// property.
abstract class WalletLocalizations {
  WalletLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static WalletLocalizations of(BuildContext context) {
    return Localizations.of<WalletLocalizations>(context, WalletLocalizations)!;
  }

  static const LocalizationsDelegate<WalletLocalizations> delegate =
      _WalletLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('ar'),
    Locale('de'),
    Locale('en'),
    Locale('es'),
    Locale('fi'),
    Locale('fr'),
    Locale('he'),
    Locale('it'),
    Locale('ja'),
    Locale('nb'),
    Locale('nl'),
    Locale('pl'),
    Locale('pt'),
    Locale('ru'),
    Locale('uk'),
    Locale('zh'),
  ];

  /// Wallet overflow-menu item that opens the host app's own settings page (the route the host wires; the example puts its theme, Tor and device-log choices there).
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get walletAppearanceMenuItem;

  /// Wallet screen title.
  ///
  /// In en, this message translates to:
  /// **'Wallet'**
  String get walletTitle;

  /// Heading shown when no wallet is provisioned in this build.
  ///
  /// In en, this message translates to:
  /// **'Wallet not set up yet'**
  String get walletNotSetUpTitle;

  /// Honest-degradation body for the not-set-up wallet state; states the money-safety reason setup is gated. Only for builds that genuinely ship without the wallet — a failed boot wiring renders walletStartupFailed* instead (#356-F1).
  ///
  /// In en, this message translates to:
  /// **'Wallet setup arrives in a later build. Setup will walk you through writing down your recovery phrase before any funds can be received — so nothing is ever at risk without a backup.'**
  String get walletNotSetUpBody;

  /// Heading of the boot-wiring failure screen (WalletStartupFailedScreen): the app ships the wallet but its startup work (native library load / data directory) failed.
  ///
  /// In en, this message translates to:
  /// **'The wallet couldn\'t start'**
  String get walletStartupFailedTitle;

  /// Body of the boot-wiring failure screen. Must reassure that a boot failure never means fund loss (funds are on-chain), and point at the retry.
  ///
  /// In en, this message translates to:
  /// **'Something stopped the wallet from loading on this device. If you already have a wallet, its funds are not affected — they live on the Zcash network and can be restored with your recovery phrase. Try again; if this keeps happening, close and reopen the app.'**
  String get walletStartupFailedBody;

  /// Label above the total wallet balance.
  ///
  /// In en, this message translates to:
  /// **'Balance'**
  String get walletBalanceLabel;

  /// Screen-reader label and tooltip of the eye button in the wallet header while amounts are SHOWN: pressing it replaces every amount with dots (FR-49 W-7; maintainer FD-6).
  ///
  /// In en, this message translates to:
  /// **'Hide balance'**
  String get walletHideBalance;

  /// Screen-reader label and tooltip of the eye button while amounts are HIDDEN: pressing it shows them again (FR-49 W-7; maintainer FD-6).
  ///
  /// In en, this message translates to:
  /// **'Show balance'**
  String get walletShowBalance;

  /// What a screen reader says in place of an amount the user has hidden with the eye button (the screen shows dots).
  ///
  /// In en, this message translates to:
  /// **'Balance hidden'**
  String get walletBalanceHiddenAmount;

  /// A ZEC amount with its unit; amount is pre-formatted by integer math (never a float).
  ///
  /// In en, this message translates to:
  /// **'{amount} ZEC'**
  String walletAmount(String amount);

  /// Label for the confirmed, spendable portion of the balance.
  ///
  /// In en, this message translates to:
  /// **'Spendable now'**
  String get walletSpendableLabel;

  /// New incoming money that is not yet confirmed. Used in two places: the synced balance card's foot, shown OUTSIDE the balance with a '+' before the amount (maintainer: 'show the actual amount that we have now and pending is a separate part'), and the status of an incoming activity row that is not yet mined. Never for money the user already has.
  ///
  /// In en, this message translates to:
  /// **'Arriving'**
  String get walletArrivingLabel;

  /// Balance card breakdown row while the wallet is NOT synced: money the user already has, inside the balance, that cannot be spent until the sync finishes (the wallet cannot yet build what spending it needs). Not new or incoming money — never translate as 'arriving' or 'incoming'.
  ///
  /// In en, this message translates to:
  /// **'Not spendable yet'**
  String get walletNotSpendableYetLabel;

  /// Heading above the transaction-history list on the wallet screen.
  ///
  /// In en, this message translates to:
  /// **'Activity'**
  String get walletActivityTitle;

  /// Empty-state line when the wallet has no transaction history.
  ///
  /// In en, this message translates to:
  /// **'No activity yet'**
  String get walletActivityEmpty;

  /// Honest error line when the transaction-history read fails or times out.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t load activity'**
  String get walletActivityError;

  /// Title of an incoming (positive net amount) history row.
  ///
  /// In en, this message translates to:
  /// **'Received'**
  String get walletActivityReceived;

  /// Title of an outgoing (negative net amount) history row.
  ///
  /// In en, this message translates to:
  /// **'Sent'**
  String get walletActivitySent;

  /// Status of a broadcast-but-unmined history row.
  ///
  /// In en, this message translates to:
  /// **'Pending'**
  String get walletActivityPending;

  /// Status of a send saved offline, waiting to broadcast.
  ///
  /// In en, this message translates to:
  /// **'Queued'**
  String get walletActivityQueued;

  /// Status of a wallet-created, unmined history row the wallet still OWES a broadcast (TxSummary.delivery == retryPending, stage S8 obligation): no endpoint has accepted it yet; the same signed bytes go out again on the next sync pass and after a relaunch. Replaces 'Pending' for that row in the activity list and the detail sheet.
  ///
  /// In en, this message translates to:
  /// **'Retrying'**
  String get walletActivityRetrying;

  /// Status of a wallet-created, unmined history row whose signed bytes are kept but which the wallet is NOT broadcasting on its own right now (TxSummary.delivery == persisted — a swap deposit held past its quote's window). No promise of automatic sending.
  ///
  /// In en, this message translates to:
  /// **'Saved'**
  String get walletActivitySaved;

  /// Status of an unmined transaction past its expiry — funds returned to spendable.
  ///
  /// In en, this message translates to:
  /// **'Expired'**
  String get walletActivityExpired;

  /// Status of a transaction the endpoint rejected.
  ///
  /// In en, this message translates to:
  /// **'Failed'**
  String get walletActivityFailed;

  /// Confirmation depth of a mined history row.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 confirmation} other{{count} confirmations}}'**
  String walletActivityConfirmations(int count);

  /// Transient arrival cue (SnackBar; also announced by screen readers) shown when the live incoming-funds stream reports new confirmed arrivals (maintainer decision — the minimal option). DELIBERATELY amount-free: the event payload carries no amount by design (ADR-0536); details are one tap away in the activity list.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Payment received} other{{count} payments received}}'**
  String walletPaymentReceived(int count);

  /// Screen-reader tap hint on a history row (the row opens the transaction-detail sheet).
  ///
  /// In en, this message translates to:
  /// **'Show transaction details'**
  String get walletActivityRowHint;

  /// Label of the status row in the transaction-detail sheet.
  ///
  /// In en, this message translates to:
  /// **'Status'**
  String get walletTxDetailStatus;

  /// Label of the fee row in the transaction-detail sheet (shown only when the fee is known).
  ///
  /// In en, this message translates to:
  /// **'Network fee'**
  String get walletTxDetailFee;

  /// Label of the date row in the transaction-detail sheet (absolute, locale-aware).
  ///
  /// In en, this message translates to:
  /// **'Date'**
  String get walletTxDetailDate;

  /// Label of the mined-height row in the transaction-detail sheet (shown only when mined).
  ///
  /// In en, this message translates to:
  /// **'Block height'**
  String get walletTxDetailHeight;

  /// Label of the memo row in the transaction-detail sheet (shown only when the tx carries one).
  ///
  /// In en, this message translates to:
  /// **'Memo'**
  String get walletTxDetailMemo;

  /// Value of the memo row: the tx carries an encrypted memo (content is not yet fetchable through the SDK, so only its presence is shown).
  ///
  /// In en, this message translates to:
  /// **'Included'**
  String get walletTxDetailMemoAttached;

  /// Label of the transaction-id row in the transaction-detail sheet.
  ///
  /// In en, this message translates to:
  /// **'Transaction ID'**
  String get walletTxDetailTxid;

  /// Button that copies the FULL transaction id (the row shows a shortened form).
  ///
  /// In en, this message translates to:
  /// **'Copy transaction ID'**
  String get walletTxDetailCopyTxid;

  /// Snackbar confirmation after copying the transaction id.
  ///
  /// In en, this message translates to:
  /// **'Transaction ID copied'**
  String get walletTxDetailCopied;

  /// Dismiss button of the transaction-detail sheet (sibling of walletShieldClose/walletMoveClose).
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get walletTxDetailClose;

  /// Prominent reassurance for an expired/failed transaction: the money-honest core fact (TxStatus.expired ⇒ funds returned to spendable; failed ⇒ endpoint rejected, nothing spent). Used in the detail-sheet banner and appended to the history row's screen-reader label.
  ///
  /// In en, this message translates to:
  /// **'No funds left your wallet'**
  String get walletTxFundsKept;

  /// Plain-language explanation of the Queued status in the transaction-detail sheet. Never 'waiting for a connection' (a send can be queued while ONLINE via the not-synced-yet path, and a queued row can be viewed during a non-network stall). Names NO drain schedule at all (#401 R1): at host custody the background pass holds no signing credential, so it points at the surface and the two real actions instead — the walletSendQueuedBody wording family. LATENT today: history.rs maps Queued rows out of the chain view, so this arm is unreachable; it is kept true so it cannot go live wrong.
  ///
  /// In en, this message translates to:
  /// **'Saved on this device, under Saved & pending — you can send it or cancel it there.'**
  String get walletTxExplainQueued;

  /// Plain-language explanation of the Pending status in the transaction-detail sheet (also the forward-compat Unknown arm, which renders as Pending). Shown for an unmined wallet-created row only once an endpoint ACCEPTED it (TxSummary.delivery accepted/null); a row the wallet is still retrying gets walletTxExplainRetrying instead, because 'sent to the network' is false for it.
  ///
  /// In en, this message translates to:
  /// **'Sent to the Zcash network — waiting to be confirmed in a block.'**
  String get walletTxExplainPending;

  /// Plain-language explanation for a wallet-created, unmined row the wallet still owes a broadcast (TxSummary.delivery == retryPending, stage S8 obligation). Cause-agnostic (a transport miss, a blackholed private path, a kill between signing and sending). 'On each sync' — never 'as soon as you're online' (the #399 reconnect-promptness rule). The expiry clause is true: a transaction past its expiry height is never rebroadcast and the funds free again.
  ///
  /// In en, this message translates to:
  /// **'Your wallet couldn\'t send this to the Zcash network yet. It keeps the signed transaction and tries again on each sync until it goes through or expires.'**
  String get walletTxExplainRetrying;

  /// Plain-language explanation for a wallet-created, unmined row whose bytes are kept without a retry promise (TxSummary.delivery == persisted — a swap deposit held past its quote's window). Makes no claim about automatic sending either way.
  ///
  /// In en, this message translates to:
  /// **'Your wallet has kept this signed transaction but isn\'t sending it by itself right now.'**
  String get walletTxExplainSaved;

  /// Plain-language explanation of the Confirmed status in the transaction-detail sheet (the depth rides the status row).
  ///
  /// In en, this message translates to:
  /// **'Confirmed on the Zcash network.'**
  String get walletTxExplainConfirmed;

  /// Plain-language explanation of the Expired status: cancelled, nothing withdrawn (the maintainer's 'what does expired mean' ask). The banner (walletTxFundsKept) carries the headline reassurance; this adds the mechanism.
  ///
  /// In en, this message translates to:
  /// **'This transaction expired before the network confirmed it, so it was cancelled. The amount is still yours to spend.'**
  String get walletTxExplainExpired;

  /// Plain-language explanation of the Failed status: endpoint-rejected, nothing withdrawn.
  ///
  /// In en, this message translates to:
  /// **'The network rejected this transaction, so it didn\'t go through. The amount is still yours to spend.'**
  String get walletTxExplainFailed;

  /// Plain-language explanation of the forward-compat Unknown status: neutral — must never AFFIRM a broadcast (unlike walletTxExplainPending) nor claim cancellation, since the binding cannot interpret the state (security review).
  ///
  /// In en, this message translates to:
  /// **'This transaction\'s current status can\'t be determined. It will update after the next sync.'**
  String get walletTxExplainUnknown;

  /// Tooltip / screen-reader label for the wallet app-bar overflow menu.
  ///
  /// In en, this message translates to:
  /// **'More options'**
  String get walletMenuTooltip;

  /// Wallet app-bar overflow-menu item that opens the rescan-recovery sheet.
  ///
  /// In en, this message translates to:
  /// **'Rescan history…'**
  String get walletRescanMenuItem;

  /// Wallet app-bar overflow-menu item that runs the manual one-time (ephemeral) address check/recovery — always available, covering returns the automatic windowed detect can no longer see (an old return past the detect window, or a re-used one-time address).
  ///
  /// In en, this message translates to:
  /// **'Check one-time addresses…'**
  String get walletCheckOneTimeMenuItem;

  /// Title of the rescan-recovery confirm sheet.
  ///
  /// In en, this message translates to:
  /// **'Rescan your history'**
  String get walletRescanTitle;

  /// Body of the rescan sheet: what rescan does + the money-safety reassurance.
  ///
  /// In en, this message translates to:
  /// **'Missing older funds? Re-scan the blockchain from further back to recover deposits an earlier start date skipped. Your funds and recovery phrase are never at risk.'**
  String get walletRescanBody;

  /// Heading of the date-range control in the rescan sheet.
  ///
  /// In en, this message translates to:
  /// **'How far back to scan'**
  String get walletRescanRangeTitle;

  /// Description shown when the rescan will scan the full history (no date).
  ///
  /// In en, this message translates to:
  /// **'Scan your whole history — slowest, but recovers everything.'**
  String get walletRescanRangeAll;

  /// Description of the DEFAULT rescan range when it is the wallet's own birthday (a wallet younger than a year, where scanning earlier would only cover empty pre-wallet blocks). Does NOT claim completeness of the USER's funds — a too-high restore birthday can leave real deposits below this floor — so it points the recovery user at the earlier-date / scan-all escape hatch (the same nudge the chosen-date arm carries).
  ///
  /// In en, this message translates to:
  /// **'Scanning from your wallet\'s start. Restored this wallet and older funds are still missing? Pick an earlier date, or Scan all history.'**
  String get walletRescanRangeDefault;

  /// Transient description while the wallet's scan-floor read is in flight (sub-millisecond normally; up to the FFI timeout on a wedged bridge). Pick-a-date and Scan-all stay available; only Start waits.
  ///
  /// In en, this message translates to:
  /// **'Preparing the recommended range…'**
  String get walletRescanRangeResolving;

  /// Size cue under the rescan range control — the approximate number of blocks the chosen range covers, so the duration warning has a visible magnitude. Pre-formatted compact count (e.g. "1.6M").
  ///
  /// In en, this message translates to:
  /// **'About {blocks} blocks to scan.'**
  String walletRescanEstimate(String blocks);

  /// Description shown when a rescan start date is chosen.
  ///
  /// In en, this message translates to:
  /// **'Scanning from {date} on. Still missing older funds? Pick an earlier date, or Scan all history.'**
  String walletRescanRangeChosen(String date);

  /// Button to choose a rescan start date.
  ///
  /// In en, this message translates to:
  /// **'Pick a date'**
  String get walletRescanPick;

  /// Button to change an already-chosen rescan start date.
  ///
  /// In en, this message translates to:
  /// **'Change date'**
  String get walletRescanChange;

  /// Button to clear the rescan start date and scan the full history.
  ///
  /// In en, this message translates to:
  /// **'Scan all history'**
  String get walletRescanScanAll;

  /// Help text on the rescan date picker dialog.
  ///
  /// In en, this message translates to:
  /// **'Earliest date to scan'**
  String get walletRescanDatePick;

  /// Honest expectation-setting note in the rescan sheet — the duration scales with how far back the chosen date reaches (the ~1-year default measured hours on emulator-class hardware, so 'a few minutes' over-promised).
  ///
  /// In en, this message translates to:
  /// **'This re-scans the blockchain. Recent dates take minutes; scanning far back can take hours. Sync runs in the background — you can keep using your wallet.'**
  String get walletRescanWarning;

  /// Rescan sheet advisory shown while in-flight sends exist (#364 M3): the engine's settling-send fence will likely refuse the rescan, and the refusal costs a full quiesce — say so BEFORE Start. Advisory only; the engine stays authoritative, so the copy must hedge ('usually', 'expect') and never promise refusal.
  ///
  /// In en, this message translates to:
  /// **'A payment from this wallet is still settling. The wallet usually declines to rescan until it completes — you can try, but expect it to be refused.'**
  String get walletRescanSettlingAdvisory;

  /// Confirm button that starts the rescan.
  ///
  /// In en, this message translates to:
  /// **'Start rescan'**
  String get walletRescanConfirm;

  /// Dismiss the rescan sheet without rescanning.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get walletRescanCancel;

  /// Label while the rescan rebuild FFI call is in progress.
  ///
  /// In en, this message translates to:
  /// **'Rebuilding…'**
  String get walletRescanRunning;

  /// Activity-section cue while a full-history rescan repopulates.
  ///
  /// In en, this message translates to:
  /// **'Rebuilding your history — scanning your whole chain. Your balance and activity fill in as it catches up.'**
  String get walletRescanRebuildingAll;

  /// Activity-section cue while a dated rescan repopulates.
  ///
  /// In en, this message translates to:
  /// **'Rebuilding your history from {date} — your balance and activity fill in as it catches up.'**
  String walletRescanRebuildingFrom(String date);

  /// Activity-section cue while the default rescan (from the wallet's own birthday) repopulates.
  ///
  /// In en, this message translates to:
  /// **'Rebuilding your history from your wallet\'s start — your balance and activity fill in as it catches up.'**
  String get walletRescanRebuildingDefault;

  /// Top-of-wallet reassurance banner on the durable catch-up arm (#380): the wallet has never completed a sync pass and is below the chain tip — a relaunch mid-rescan catch-up, a dismissed rescan notice over a rebuilt wallet, or a restore/create's first sync. Generic: unlike the walletRescanRebuilding* family it cannot name what the user chose (the choice does not survive a relaunch). HEDGED funds claim (review, the #356-F7 precedent): a fresh create's first sync — possibly offline, so unbounded — shares this cue, and 'your funds are safe' would assert funds that provably don't exist; 'anything you've received' is conditionally true on every arm.
  ///
  /// In en, this message translates to:
  /// **'Catching up — your balance and activity fill in as the wallet syncs. Anything you\'ve received is safe.'**
  String get walletCatchUpBanner;

  /// Top-of-wallet reassurance banner on the durable rescan arm (#377 s357b-2): the core's rescan-rebuilding breadcrumb says a rescan rebuild is still catching up, but the in-session choice (which range) did not survive the relaunch — so the copy names the RESCAN (the user's own deliberate action, the stronger 'did I lose funds?' reassurance) without naming the range the walletRescanRebuilding* family can. Same HEDGED funds claim as walletCatchUpBanner ('anything you've received' — a zero-balance wallet rescans too).
  ///
  /// In en, this message translates to:
  /// **'Rebuilding your history after a rescan — your balance and activity fill in as it catches up. Anything you\'ve received is safe.'**
  String get walletCatchUpRescanBanner;

  /// Honest cue shown when a rescan failed but the wallet was recovered by re-opening. Claims funds-safety only, never 'unchanged' (#379): a fault AFTER the rebuild's atomic rename recovers into the REBUILT lower-birthday wallet, whose balance/history repopulate via the auto-restarted sync.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t rescan right now — your funds are safe, though your balance and history may need a little time to catch back up. Try again in a moment.'**
  String get walletRescanFailedNotice;

  /// Cue shown when the SDK refused a rescan because a broadcast send has not settled yet (rebuilding under it could double-pay); resolution takes hours of online sync.
  ///
  /// In en, this message translates to:
  /// **'A payment is still settling, so rescanning is paused to protect your funds. Your wallet is unchanged — try again in a couple of hours and keep the app open and online.'**
  String get walletRescanBlockedSettlingNotice;

  /// Rescan refusal notice (#405): the commit-point fence turned the confirm away because no sync pass will run (host policy off OR a failed sync start), so the wipe would strand a zeroed wallet behind a rebuild that cannot execute. Nothing destructive ran, which is why this is the one rescan refusal entitled to say 'your wallet is unchanged' outright (#379 softened walletRescanFailedNotice because a post-rename fault leaves a REBUILT wallet). CAUSE-AGNOSTIC — the badge and the start-failed notice above it carry the cause and the way out.
  ///
  /// In en, this message translates to:
  /// **'Rescanning rebuilds your history as your wallet syncs, and syncing isn\'t running right now. Your wallet is unchanged — try again once syncing is running.'**
  String get walletRescanBlockedSyncNotRunningNotice;

  /// Cue shown when a rescan failed because the device is out of disk space; retrying without freeing space will fail again, so the copy asks for space instead of a retry. Claims funds-safety only, never 'unchanged' (#379): a DiskFull after the rebuild's atomic rename recovers into the REBUILT lower-birthday wallet.
  ///
  /// In en, this message translates to:
  /// **'There isn\'t enough free space to rebuild your wallet history — your funds are safe, though your balance and history may need a little time to catch back up. Free up some space and try again.'**
  String get walletRescanNeedsSpaceNotice;

  /// Dismiss the rescan-failed cue.
  ///
  /// In en, this message translates to:
  /// **'Dismiss'**
  String get walletRescanFailedDismiss;

  /// Short activity-section cue shown in place of the empty card while a rescan repopulates.
  ///
  /// In en, this message translates to:
  /// **'Rebuilding your history…'**
  String get walletActivityRebuilding;

  /// Short activity-section cue shown in place of the empty card on the durable catch-up arm (#380) — the wallet is provably still filling in but the rescan choice is unknown (post-relaunch / post-dismiss / a restore's first sync). HEDGED (review): a fresh create's first sync shares this cue, so it must not assert a history that doesn't exist — 'anything you've received will show up here' is true on the rebuilt, restored, AND empty-new arms.
  ///
  /// In en, this message translates to:
  /// **'Still catching up — anything you\'ve received will show up here.'**
  String get walletActivityCatchingUp;

  /// Activity-section note replacing 'No activity yet' when history exists but cannot repopulate because no sync pass will run (UX HIGH; #405 widened it from the host policy to the SSOT so a FAILED start is covered too). States the pending fill and its condition; CAUSE-AGNOSTIC — the badge above it names why sync isn't running.
  ///
  /// In en, this message translates to:
  /// **'Your balance and history will finish loading once syncing is running.'**
  String get walletActivitySyncNotRunning;

  /// Button at the bottom of the activity list that fetches the next keyset page.
  ///
  /// In en, this message translates to:
  /// **'Load more'**
  String get walletActivityLoadMore;

  /// Label for our own change in flight (normal right after a send); shown so the breakdown reconciles to the total.
  ///
  /// In en, this message translates to:
  /// **'Pending change'**
  String get walletPendingChangeLabel;

  /// Label for the transparent (unshielded) portion of the balance — privacy-relevant, shown only when nonzero.
  ///
  /// In en, this message translates to:
  /// **'Unshielded (public)'**
  String get walletTransparentLabel;

  /// Honest note under a nonzero unshielded balance: BOTH the spendability truth (transparent funds count toward the total but not the spendable figure until shielded — the 'why is my total bigger than spendable' question must be answerable from the card itself, maintainer) AND the privacy truth (publicly visible until shielded).
  ///
  /// In en, this message translates to:
  /// **'Not included in \"Spendable now\" — shield these funds to spend them. Until then they stay publicly visible on-chain.'**
  String get walletTransparentNote;

  /// The transparent-funds note for a WATCH-ONLY wallet (#397 §3.7 D5): it keeps only the privacy truth (public on-chain) and drops the 'shield these to spend' framing, which a watch-only wallet cannot follow (no spending keys).
  ///
  /// In en, this message translates to:
  /// **'These funds are publicly visible on-chain.'**
  String get walletTransparentNoteWatchOnly;

  /// Balance-card pool-clarity line (#389), private-pool segment: the shielded (private) portion of the total, shown always-on directly under the headline so 'how much of my ZEC is private?' is answerable at a glance. amount is a pre-formatted BARE ZEC figure (NO unit) — the unit rides the headline right above, and shielded + transparent sum EXACTLY to it (shielded = total − transparent), so the bare numbers can never disagree with the total. Keep it short: it shares ONE line with the transparent segment. 'Shielded' is the same privacy term used across the card; translate it as the sibling walletTransparentLabel does.
  ///
  /// In en, this message translates to:
  /// **'Shielded {amount}'**
  String walletPoolShielded(String amount);

  /// Balance-card pool-clarity line (#389), public-pool segment: the transparent (unshielded, publicly-visible-on-chain) portion, rendered in the same privacy-orange the rest of the card uses for transparent funds. amount is pre-formatted BARE ZEC (unit on the headline). Keep it short (shares one line with the shielded segment). Match the 'transparent/unshielded' wording of the sibling walletTransparentLabel.
  ///
  /// In en, this message translates to:
  /// **'Public {amount}'**
  String walletPoolTransparent(String amount);

  /// Balance-card pool-clarity line (#389) when the transparent balance is zero: a positive affirmation that EVERY coin is in the shielded (private) pool. Always-on so an all-private wallet gets explicit reassurance, not merely the absence of a transparent line. The '·' (U+00B7 middot) separates the two clauses; keep a real middot with a space on each side. 'shielded' and 'private' as elsewhere on the card.
  ///
  /// In en, this message translates to:
  /// **'All shielded · private'**
  String get walletPoolAllShielded;

  /// Screen-reader tap hint (#389) for the balance-card pool line: read after the button role as 'double tap to <hint>'. Tapping the line opens the Transparent funds sheet. Imperative verb phrase, like walletSyncBadgeHint ('Show sync details').
  ///
  /// In en, this message translates to:
  /// **'Show public funds'**
  String get walletPoolTapHint;

  /// Note under the transparent line: part of the unshielded funds sits on a wallet-controlled one-time (ephemeral) address and is reorg-final/recoverable. SUBSET of the balance, never added on top. Cause-agnostic (an exchange return as much as an expired transfer), so never 'stranded'/'bounced'. amount is pre-formatted.
  ///
  /// In en, this message translates to:
  /// **'{amount} of your balance is on a one-time address (recoverable).'**
  String walletRecoverableEphemeralNote(String amount);

  /// The recoverable-ephemeral note for a WATCH-ONLY wallet (#397 §3.7 D3): keeps the locational fact but DROPS the '(recoverable)' claim — recovery mints a self-send (a spend) a watch-only wallet cannot do, and its reclaim affordance is hidden. amount is pre-formatted.
  ///
  /// In en, this message translates to:
  /// **'{amount} of your balance is on a one-time address.'**
  String walletRecoverableEphemeralNoteWatchOnly(String amount);

  /// The durable in-flight two-step cue (#309): first leg broadcast, send not complete — money in motion through a wallet-controlled one-time address. Repeats the permanently-true 'don't re-send' after the result screen is dismissed; cause-agnostic + locational; NO auto-completion promise. Plural: each in-flight send uses its OWN one-time address, so N>=2 must not say 'a one-time address' / 'it'. amount is the pre-formatted aggregate and annotates the same pending payments the activity list shows. TERM (D2, #347): 'set aside for' / 'are set aside' replaced 'committed' — the safety term must NEVER read as blockchain-CONFIRMED, and never as STUCK; the 15 locales were already normalized to reserved/allocated, so EN now ratifies them.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{amount} is set aside for a payment your wallet is still completing through a one-time address it controls. Don\'t send it again.} other{Payments totalling {amount} are set aside and still completing through one-time addresses your wallet controls. Don\'t send them again.}}'**
  String walletInFlightNote(num count, String amount);

  /// walletInFlightNote's variant when NO background sync pass will run (re-keyed and renamed by #403 R4). The second leg forwards only on sync passes, so 'still completing' would claim progress and then negate it — this variant says 'partway through … paused' as one coherent story. Renamed from ...SyncOff and made CAUSE-AGNOSTIC for the same reason as its three re-keyed siblings (walletParkedPreparingHintSyncPaused, walletParkedAuthorizeSentSyncPaused, walletParkedSyncPausedNote): the drive is also not running after a FAILED sync start, where the host policy still reads on and 'syncing is off in this app's settings' is simply the wrong remedy — the screen's own sync notice carries the cause. Match walletSyncPausedMoneyNote's condition wording. Keep the D2 'set aside' term and the identical don't-send-again instruction; same plural rule and placeholders as the sibling.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, one{{amount} is set aside for a payment partway through a one-time address your wallet controls. It\'s paused until your wallet is syncing again. Don\'t send it again.} other{Payments totalling {amount} are set aside partway through one-time addresses your wallet controls. They\'re paused until your wallet is syncing again. Don\'t send them again.}}'**
  String walletInFlightNoteSyncPaused(num count, String amount);

  /// #308a (S2 §3.5d): shown in place of the in-flight cue (walletInFlightNote) when the in-flight-sends read FAILS, instead of the cue silently vanishing — a vanished cue reads exactly like 'nothing is mid-flight'. The read retries on its own and on every sync/resume edge, so 'Retrying' is true. It must not claim anything IS in flight (we don't know), and must keep the don't-double-pay caution by pointing at the activity list, where the payment's first leg shows as a pending transaction.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t check whether a payment is still completing. Retrying — until then, look for a pending payment in your activity before you send again.'**
  String get walletInFlightReadError;

  /// As walletRecoverableEphemeralNote, but the amount is not yet reorg-final (still confirming) — shown as pending recovery, never settled/ready. amount is pre-formatted.
  ///
  /// In en, this message translates to:
  /// **'{amount} of your balance is on a one-time address (still confirming).'**
  String walletRecoverableEphemeralConfirmingNote(String amount);

  /// Action button next to the unshielded balance — moves transparent funds into the private shielded pool (Recv-3).
  ///
  /// In en, this message translates to:
  /// **'Shield'**
  String get walletShieldButton;

  /// Title of the shield confirmation sheet.
  ///
  /// In en, this message translates to:
  /// **'Shield public funds'**
  String get walletShieldSheetTitle;

  /// Privacy-positive framing of the shield action (the inverse of a de-shield warning).
  ///
  /// In en, this message translates to:
  /// **'This moves funds from your public, on-chain-visible balance into your private shielded balance.'**
  String get walletShieldNote;

  /// Transient state while the shield proposal is computed (local, no network).
  ///
  /// In en, this message translates to:
  /// **'Preparing…'**
  String get walletShieldPreparing;

  /// Label for the gross transparent amount being shielded.
  ///
  /// In en, this message translates to:
  /// **'Shielding'**
  String get walletShieldAmountLabel;

  /// Label for the ZIP-317 fee on the shield transaction.
  ///
  /// In en, this message translates to:
  /// **'Network fee'**
  String get walletShieldFeeLabel;

  /// Label for the net amount that ends up in the shielded balance (gross minus fee).
  ///
  /// In en, this message translates to:
  /// **'Lands shielded'**
  String get walletShieldNetLabel;

  /// Confirm button that signs + broadcasts the shield transaction.
  ///
  /// In en, this message translates to:
  /// **'Shield now'**
  String get walletShieldConfirmButton;

  /// Transient state while the shield tx is signed and broadcast.
  ///
  /// In en, this message translates to:
  /// **'Shielding…'**
  String get walletShieldSubmitting;

  /// Shown when the transparent balance is below the shieldable minimum.
  ///
  /// In en, this message translates to:
  /// **'Nothing to shield yet'**
  String get walletShieldNothingTitle;

  /// Honest explanation that the transparent balance is below the shielding threshold.
  ///
  /// In en, this message translates to:
  /// **'These funds are below the amount worth shielding right now — the network fee would outweigh the benefit. They\'ll be shieldable once a bit more arrives.'**
  String get walletShieldNothingBody;

  /// Terminal success: the shield tx was broadcast.
  ///
  /// In en, this message translates to:
  /// **'Shielding submitted'**
  String get walletShieldDoneTitle;

  /// Body for a successful shield broadcast.
  ///
  /// In en, this message translates to:
  /// **'Your funds are moving into your shielded balance. It will confirm on-chain shortly.'**
  String get walletShieldDoneBody;

  /// The shield tx is persisted but not yet broadcast; it re-sends on a later sync (money-safe). NOT 'when you're online' (#401 R1b): that is the reconnect-promptness shape #399 forbids — the re-send rides a completed sync pass, which can lag a reconnect and never comes at all while sync is paused. Mirrors walletSendSavedTitle.
  ///
  /// In en, this message translates to:
  /// **'Saved — we\'ll finish shielding'**
  String get walletShieldSavedTitle;

  /// Honest body for the unbroadcast (saved-for-retry) shield — confirmation is NOT imminent; it re-sends on a later sync. Same #401 R1b posture as walletSendSavedBody (already signed ⇒ custody-independent, pass-dependent ⇒ the render site appends walletSyncPausedMoneyNote), and no 'next time you're online' reconnect promise.
  ///
  /// In en, this message translates to:
  /// **'We couldn\'t reach the network just now. Your shielding is saved and your wallet will complete it on a later sync. Nothing is lost.'**
  String get walletShieldSavedBody;

  /// The one-shot shield token was already consumed (a double-tap) — never broadcast twice.
  ///
  /// In en, this message translates to:
  /// **'Already submitted'**
  String get walletShieldAlreadyTitle;

  /// The shield could not be prepared or signed; no funds moved.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t shield right now'**
  String get walletShieldFailedTitle;

  /// The wallet isn't synced far enough to anchor the shield yet — retry after sync.
  ///
  /// In en, this message translates to:
  /// **'The wallet is still syncing. Try shielding again in a moment.'**
  String get walletShieldStaleBody;

  /// Shield fault body for the retryable prepare refusal (WalletErrorKind.proposeTransient, INC-018 (b)): a condition the wallet's own state clears — an anchor not yet recorded, an input a concurrent proposal holds, a witness the scan has not completed. The shield sibling of walletSendFaultCouldNotPrepareTransient; MUST NOT claim the wallet is 'still syncing' (that is walletShieldStaleBody — a locked input is not a sync matter) and MUST NOT be title-only (the couldNotPrepare arm's dead-end). No funds moved.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t prepare the shield just now. Try again in a moment.'**
  String get walletShieldTransientBody;

  /// Shield fault: the device is out of disk space, so preparing/persisting the shield hit DiskFull (#373 follow-up). Retrying without freeing space fails again, so the copy asks for space instead of a plain retry. Funds are untouched. Sibling of walletSendFaultStorageFull.
  ///
  /// In en, this message translates to:
  /// **'There isn\'t enough free space to shield right now. Free up some space and try again. Your funds are safe.'**
  String get walletShieldStorageFullBody;

  /// Dismiss the shield sheet after a terminal outcome.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get walletShieldClose;

  /// Re-run the shield after a recoverable failure.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletShieldRetry;

  /// Overflow-menu entry for the Send expert layer's de-shield action (the mirror of Shield).
  ///
  /// In en, this message translates to:
  /// **'Move to public…'**
  String get walletMoveMenuItem;

  /// Title of the move-to-transparent (de-shield to own address) sheet.
  ///
  /// In en, this message translates to:
  /// **'Move to public'**
  String get walletMoveSheetTitle;

  /// Explains what move-to-transparent is for (exchange deposits that reject shielded sources).
  ///
  /// In en, this message translates to:
  /// **'Send shielded ZEC to your own public address — useful for an exchange that won\'t accept a shielded deposit.'**
  String get walletMoveSheetSubtitle;

  /// Label for the read-only destination — the wallet's own transparent address.
  ///
  /// In en, this message translates to:
  /// **'Your public address'**
  String get walletMoveDestinationLabel;

  /// Spendable shielded-balance hint above the move amount field; integer-formatted (never a float).
  ///
  /// In en, this message translates to:
  /// **'Available to move: {amount} ZEC'**
  String walletMoveAvailable(String amount);

  /// Variant of walletMoveAvailable while the wallet is still catching up (#381, the #380 swap-line rule): the spendable figure is the partial repopulating balance, so a low/zero figure must not read as final.
  ///
  /// In en, this message translates to:
  /// **'Available to move: {amount} ZEC — your balance is still catching up'**
  String walletMoveAvailableCatchingUp(String amount);

  /// The §5.1 de-shield warning title, move-specific (a self-transfer, not a payment to a third party).
  ///
  /// In en, this message translates to:
  /// **'This move makes your funds public'**
  String get walletMoveDeshieldTitle;

  /// The §5.1 de-shield warning body, move-specific: the funds + the user's own t-address go public on-chain (no third-party 'recipient' framing).
  ///
  /// In en, this message translates to:
  /// **'Moving to a public address takes these funds out of your shielded balance — the amount and your public address become publicly visible on the Zcash blockchain.'**
  String get walletMoveDeshieldBody;

  /// Body for the defensive walletUnavailable terminal — the session dropped mid-sheet (rare).
  ///
  /// In en, this message translates to:
  /// **'The wallet session ended. Close and reopen to try again.'**
  String get walletMoveWalletEnded;

  /// Transient state while the wallet's own transparent address is loaded.
  ///
  /// In en, this message translates to:
  /// **'Preparing…'**
  String get walletMoveLoading;

  /// Transient state while the de-shield proposal is computed (local, no network).
  ///
  /// In en, this message translates to:
  /// **'Checking the amount…'**
  String get walletMovePreparing;

  /// Transient state while the de-shield tx is signed and broadcast.
  ///
  /// In en, this message translates to:
  /// **'Moving…'**
  String get walletMoveSubmitting;

  /// Button that proposes the de-shield and advances to the review screen.
  ///
  /// In en, this message translates to:
  /// **'Review'**
  String get walletMoveReviewButton;

  /// Dismiss the move-to-transparent sheet from the amount-entry phase.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get walletMoveCancel;

  /// Title of the de-shield confirmation/review screen.
  ///
  /// In en, this message translates to:
  /// **'Review move'**
  String get walletMoveReviewTitle;

  /// Honest, move-specific note: it's the user's own t-addr; the funds can be re-shielded, but the on-chain history of this move is permanent (no self-contradiction).
  ///
  /// In en, this message translates to:
  /// **'You\'re moving to your own public address. You can shield these funds again later, but this move stays on the public record permanently.'**
  String get walletMoveOwnAddressNote;

  /// Confirm button that signs + broadcasts the de-shield transaction.
  ///
  /// In en, this message translates to:
  /// **'Move to public'**
  String get walletMoveConfirmButton;

  /// Return from the review screen to the amount-entry form.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get walletMoveBackButton;

  /// Terminal success: the de-shield tx was broadcast.
  ///
  /// In en, this message translates to:
  /// **'Moved to public'**
  String get walletMoveDoneTitle;

  /// Body for a successful de-shield broadcast.
  ///
  /// In en, this message translates to:
  /// **'Your funds are moving to your public address. They will confirm on-chain shortly.'**
  String get walletMoveDoneBody;

  /// The de-shield tx is persisted but not yet broadcast; it re-sends on a later sync (money-safe). NOT 'when you're online' — the #399 reconnect-promptness rule, folded in by #401 R1b. Mirrors walletSendSavedTitle.
  ///
  /// In en, this message translates to:
  /// **'Saved — we\'ll finish the move'**
  String get walletMoveSavedTitle;

  /// Body for a de-shield that was persisted but not yet broadcast. Same #401 R1b posture as walletSendSavedBody: already signed ⇒ the promise holds at every custody tier; pass-dependent ⇒ the render site appends walletSyncPausedMoneyNote when no sync pass will run.
  ///
  /// In en, this message translates to:
  /// **'This move is saved and your wallet will send it on a later sync. Nothing was lost.'**
  String get walletMoveSavedBody;

  /// The one-shot proposal was already consumed (a double-tap); the funds are never sent twice.
  ///
  /// In en, this message translates to:
  /// **'Already submitted'**
  String get walletMoveAlreadyTitle;

  /// Body for an already-submitted move (precise: the prior submission moved the funds; never sent twice).
  ///
  /// In en, this message translates to:
  /// **'These funds were already submitted and are on their way to your public address.'**
  String get walletMoveAlreadyBody;

  /// Terminal failure: nothing was sent; the user can try again.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t complete this move'**
  String get walletMoveFailedTitle;

  /// Shown when there is no shielded balance available to de-shield.
  ///
  /// In en, this message translates to:
  /// **'Nothing to move yet'**
  String get walletMoveNothingTitle;

  /// Honest explanation that there is no spendable shielded balance to de-shield.
  ///
  /// In en, this message translates to:
  /// **'You have no shielded balance available to move right now. Once funds confirm, you can move them to your public address.'**
  String get walletMoveNothingBody;

  /// Variant of walletMoveNothingBody while the wallet is still catching up (#381, hardware-proven): the missing shielded balance is UNSCANNED, not unconfirmed, so the default body's 'Once funds confirm' would misattribute the cause. Hedged ('anything you've received') — it must not assert funds exist (#356-F7 precedent).
  ///
  /// In en, this message translates to:
  /// **'Your wallet is still catching up — anything you\'ve received becomes available to move once syncing completes.'**
  String get walletMoveNothingCatchingUpBody;

  /// The own-address fetch failed or timed out; retryable.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t load your public address. Try again.'**
  String get walletMoveCouldNotLoad;

  /// Re-run the move after a recoverable failure.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletMoveRetry;

  /// Close the move-to-transparent sheet from a terminal state.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get walletMoveClose;

  /// Honest, recoverable message when the cold snapshot read fails.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t read the wallet right now. It will refresh on its own.'**
  String get walletSnapshotUnavailable;

  /// Honest staleness cue shown when the last-known balance is rendered through a failed refresh.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t refresh — showing your last-known balance.'**
  String get walletBalanceStale;

  /// Honest, recoverable notice when the background sync loop's start command itself fails (rare); shown with a retry.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t start syncing. We\'ll keep trying.'**
  String get walletSyncStartFailed;

  /// Button on the sync-start-failed notice that re-attempts starting the sync loop.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletSyncRetry;

  /// Button on the sync sheet's stalled arm (#399): retry the connection immediately instead of waiting out the automatic retry schedule (restarts the sync loop, which resets its backoff).
  ///
  /// In en, this message translates to:
  /// **'Try now'**
  String get walletSyncTryNow;

  /// Sync status: wallet open, loop about to start (transient — sync auto-starts).
  ///
  /// In en, this message translates to:
  /// **'Not syncing yet'**
  String get walletSyncIdle;

  /// Next step under the idle sync status — sync runs on its own, there is no manual start.
  ///
  /// In en, this message translates to:
  /// **'Syncing starts automatically.'**
  String get walletSyncIdleDetail;

  /// Sync badge headline when the HOST app's sync policy is off (#383 R1) — syncing is deliberately not running by the embedding app's own setting. Short; state, not an error.
  ///
  /// In en, this message translates to:
  /// **'Sync off'**
  String get walletSyncDisabled;

  /// Next step under the sync-off badge — points at the HOST app's settings (the package has no sync switch of its own). Keep 'this app's settings' generic; hosts name the screen differently.
  ///
  /// In en, this message translates to:
  /// **'Turn on syncing in this app\'s settings to update your balance.'**
  String get walletSyncDisabledDetail;

  /// Sync sheet explanation for the sync-off state (#383 R1): says WHO turned it off (the app's settings, deliberately), carries the funds-safe reassurance its not-running siblings (ExplainStartFailed/ExplainOffline) carry, and is honest that the figures are the last synced state — never implies an error or that the package can turn it back on.
  ///
  /// In en, this message translates to:
  /// **'Syncing is turned off in this app\'s settings. Your funds are safe. Your balance and activity show the last synced state and won\'t update until syncing is turned on.'**
  String get walletSyncExplainDisabled;

  /// The parked-sends pause note, replacing the shared walletSyncPausedMoneyNote on THIS surface (#401 R2d + R5). Two changes. (1) It names the ESCAPE: the shared note says only 'paused', and it sits directly above a Send now button that #400 R9 deliberately left working at every custody tier — a user who reads 'paused' and stops looking has abandoned funds they could release with one tap. (2) It is CAUSE-AGNOSTIC and keyed on the sync DRIVE, not the host policy: a failed sync start freezes the queue exactly as a sync-off policy does, and 'in this app's settings' is the wrong instruction for it — the screen's own sync notice carries whichever remedy applies. RENDERED ONLY WHILE THAT ESCAPE EXISTS (#403 R2): Send now is suppressed on a mid-signature row, so a section whose rows are ALL mid-signature falls back to the affordance-free walletSyncPausedMoneyNote instead — a note naming a control that is not on screen is worse than the plain pause it replaced. 'Send now' must match walletParkedSendNow verbatim. Two full standalone sentences; never claims failure.
  ///
  /// In en, this message translates to:
  /// **'Your wallet isn\'t syncing, so these won\'t send on their own. Use Send now to send one yourself.'**
  String get walletParkedSyncPausedNote;

  /// Shared money-surface qualifier appended to the SAVED-FOR-RETRY result bodies (send / partial / shield / move) when no background sync pass will run (#401 R1b + R5). Those bodies promise the wallet finishes the send on a later sync — true at every custody tier (the transaction is already signed) but only where passes HAPPEN. Deliberately CAUSE-AGNOSTIC: the drive is not running under BOTH the host's sync-off policy AND a failed sync start, and naming one remedy would be wrong for the other — the screen already carries the cause-specific line (the sync-off settings note, or the start-failed retry notice). Plural-safe and standalone (a full sentence); never claims failure, only the pause. Appended through walletSyncPausedJoin, never by a Dart string interpolation. It has a SECOND render site since #403 R2: the parked-sends section falls back to it when every row is mid-signature, because its own note names a Send now that is suppressed on those rows — this one names no affordance, which is exactly why it fits there.
  ///
  /// In en, this message translates to:
  /// **'Paused until your wallet is syncing again.'**
  String get walletSyncPausedMoneyNote;

  /// THE JOINER, and the separator is the locale's business (#403 R6). walletSyncPausedQualified appends walletSyncPausedMoneyNote to a money body, and doing it with a Dart '$body $note' inserts a U+0020 after a fullwidth full stop (。) in ja and zh, which is wrong typography in both — this package already ADJUDICATED that exact pattern once, in, and abandoned it. TRANSLATORS: this string contains NO words. Emit the two placeholders verbatim in the order the locale's sentence flow requires, with whatever separator that language uses between two complete sentences: a single space for the space-delimited languages, and NOTHING AT ALL for ja/zh (both fragments already end in their own 。). Never add punctuation of your own — both fragments are already terminated. RTL (ar/he) keeps the single space: both fragments are strong-RTL and period-terminated, so the plain join is correct there.
  ///
  /// In en, this message translates to:
  /// **'{body} {note}'**
  String walletSyncPausedJoin(String body, String note);

  /// Sync status headline when the loop is started but hasn't reported a batch yet (the silent prep phase: reaching the server + fetching the commitment-tree roots and chain tip). Shown instead of the bare Idle 'Not syncing yet' so the wallet reads as actively connecting. NOTE: intentionally identical to walletSyncConnecting in English but a SEMANTICALLY DISTINCT state (host-side driving-Idle prep vs. the SDK's Connecting/Tor-bootstrap arm) — do not merge the two keys in translations.
  ///
  /// In en, this message translates to:
  /// **'Connecting…'**
  String get walletSyncStarting;

  /// Detail line under the connecting/starting sync status explaining the prep phase before scanning begins.
  ///
  /// In en, this message translates to:
  /// **'Reaching the Zcash network and preparing to scan.'**
  String get walletSyncStartingDetail;

  /// Sync status: connecting, no bootstrap percent available.
  ///
  /// In en, this message translates to:
  /// **'Connecting…'**
  String get walletSyncConnecting;

  /// Sync status: connecting with a Tor bootstrap percent.
  ///
  /// In en, this message translates to:
  /// **'Connecting… {percent}%'**
  String walletSyncConnectingPercent(int percent);

  /// Sync status: scanning blocks, monotonic percent complete.
  ///
  /// In en, this message translates to:
  /// **'Scanning {percent}%'**
  String walletSyncScanning(int percent);

  /// Sync status: the opaque early phase of a deep first sync where the note-fraction is still ~0 — a number-less headline (paired with an indeterminate bar + the catching-up detail) so a stuck-looking 'Scanning 0%' is never shown.
  ///
  /// In en, this message translates to:
  /// **'Scanning…'**
  String get walletSyncScanningEarly;

  /// Shown while scanning when funds are already spendable (spend-before-sync).
  ///
  /// In en, this message translates to:
  /// **'Funds are ready to spend.'**
  String get walletSyncSpendableReady;

  /// Shown under the Scanning status during the opaque early phase (percent still rounds to 0) so the wallet reads as actively working, not stuck — explains WHY the sync is slow. No trailing period: it can precede another detail line in the joined a11y label. Says 'a deep initial sync' (not 'first sync') since a restore also hits this on a fresh device.
  ///
  /// In en, this message translates to:
  /// **'Catching up with the network — a deep initial sync can take a while. You can keep using the app while it finishes'**
  String get walletSyncCatchingUp;

  /// Compact blocks-left count shown on the SAME ROW as the Scanning percent once a real percent exists (counts down). count is pre-formatted compactly, e.g. "1.6M".
  ///
  /// In en, this message translates to:
  /// **'{count} blocks left'**
  String walletSyncScanRemaining(String count);

  /// Sync status: fully synced to the chain tip.
  ///
  /// In en, this message translates to:
  /// **'Up to date'**
  String get walletSyncUpToDate;

  /// Sync status: no connectivity.
  ///
  /// In en, this message translates to:
  /// **'Offline'**
  String get walletSyncOffline;

  /// Next step under the offline sync status; queued sends are normal, not errors. Promises no drain schedule (#401 R1 — the automatic-drain family): a host-custody background pass cannot sign at all, so this names the surface the send is safe on instead. Keep 'Saved & pending' in step with walletParkedTitle.
  ///
  /// In en, this message translates to:
  /// **'Queued sends stay saved under Saved & pending.'**
  String get walletSyncOfflineDetail;

  /// Sync status: forward-compatibility arm rendered as a neutral syncing state.
  ///
  /// In en, this message translates to:
  /// **'Syncing…'**
  String get walletSyncUnknown;

  /// Sync status headline for a stalled (typed, renderable) state.
  ///
  /// In en, this message translates to:
  /// **'Sync paused'**
  String get walletSyncStalled;

  /// Stall reason: endpoint unreachable — the normal-offline (caution) arm since #399. Hedged on purpose: a refused dial can mean the SERVER is down while the user's internet is fine, so it must not assert the user's connection is the problem. The wallet retries on its own. Since P3-13 the server IS user-switchable (the sync sheet's Server row opens the picker); the copy still does not promise a switch, because a refused dial cannot say whether it is this server or the user's link that is down.
  ///
  /// In en, this message translates to:
  /// **'Can\'t reach the Zcash network right now. We\'ll keep trying automatically — check your connection, or the server may be temporarily unavailable.'**
  String get walletStallEndpoint;

  /// Stall reason: the private path is GENUINELY DOWN — a dial that failed (refused, unreachable, a dial timeout), nothing registered, a registrant that declared its transport FAILED, or a runtime the SDK cannot drive. TRANSPORT-NEUTRAL by construction (FR-30 (a), C1): this string is rendered from a StallReason, which carries no transport name — every other failing arm names the host's transport, this one cannot, so it names none. It must never say "Tor": a host that registered Shadowsocks reads it too (ADR-0547). TWO CLAIMS IT MAY NOT MAKE (FR-32 (b), stage S1 `copy`): a StallReason carries no POLICY, so a `Preferred` wallet reads this sentence too — it can neither say the private path "is required" (a setting that user may never have chosen) nor promise that "nothing was sent in the clear" (only `Required` fails closed; its dial plan has no fallback arm at all). Both were in this string until stage S1. NARROWED at stage S1 `truth`: a path that ACCEPTED the dial and then carried nothing no longer reaches this reason — it reads TorState.unanswered, whose sentence claims nothing about which side is at fault.
  ///
  /// In en, this message translates to:
  /// **'Your app\'s private path isn\'t available, so the wallet isn\'t connecting. Check your app\'s network settings, or turn the private path off. Sync resumes as soon as the path is back.'**
  String get walletStallTor;

  /// Stall reason: device storage full.
  ///
  /// In en, this message translates to:
  /// **'Device storage is full. Free some space and sync will resume.'**
  String get walletStallStorage;

  /// Stall reason: chain reorganization in progress.
  ///
  /// In en, this message translates to:
  /// **'The chain reorganized; re-checking recent blocks.'**
  String get walletStallReorg;

  /// Stall reason: a CORRUPT wallet store, or a local fault the wallet could not diagnose (StallReason.internal; R10 narrowed it) — repair/restore, never switch servers. A busy or briefly unreadable store is walletStallStorageUnavailable, which must never offer the restore.
  ///
  /// In en, this message translates to:
  /// **'A local problem stopped sync. If it keeps happening, restore from your recovery phrase.'**
  String get walletStallInternal;

  /// Stall reason: the server ANSWERED and the answer was wrong (StallReason.endpointMisbehaving — malformed or impossible data, a required pool it does not know, or a root/height that conflicts with what an EARLIER server told this wallet). The mirror image of walletStallInternal: the problem is on the SERVER, so the next step IS 'switch servers'. MUST NOT say 'check your connection' (that is walletStallEndpoint — the link works) and MUST NOT suggest restoring from the recovery phrase (nothing on the device is at fault; the seed is not involved). MAY name a RESCAN as the last resort, after other servers (T0-1d): one member of the class is a conflict with the wallet's own cached record — written from an earlier server — and the conflict alone cannot say which server lied; a rescan rebuilds that record and is the only exit when every server is refused. Uses the same 'rescan your history' vocabulary as walletRescanMenuItem. The wallet keeps retrying on its own.
  ///
  /// In en, this message translates to:
  /// **'This server sent data that can\'t be right, so sync stopped. It isn\'t a connection problem — switch to another server. If every server is refused, rescan your history: the wallet may be keeping a bad record from an earlier server.'**
  String get walletStallEndpointMisbehaving;

  /// Stall reason: the wallet's configured starting height (birthday) is above the chain tip THIS SERVER reports and above the newest height the app's bundled data vouches for (StallReason.birthdayInFuture, T0-1c-R2). The wallet cannot tell a server that is behind the chain from a starting height set above the real chain tip, so the copy MUST name BOTH next steps: check the starting block this wallet is set to, or try another server. That height has TWO producers (§4n-review row 7, §4r U-5): the birthday typed at restore AND a rescan from a chosen height (rescan_from) — so the copy says 'the starting block this wallet is set to' and MUST NOT say 'you entered when restoring' (the rescan user typed nothing at restore). MUST NOT say 'check your connection' (the server answered — that is walletStallEndpoint) and MUST NOT suggest restoring from the recovery phrase (nothing on the device is at fault — that is walletStallInternal). The wallet keeps retrying on its own; it clears when the server catches up or the starting block is lowered.
  ///
  /// In en, this message translates to:
  /// **'This wallet is set to start from a block this server hasn\'t reached yet. Check the starting block this wallet is set to, or try another server.'**
  String get walletStallBirthdayInFuture;

  /// Stall reason: a TRANSIENT local storage fault (StallReason.storageUnavailable, R10) — the wallet's database was busy past its timeout, or an I/O fault such as a locked iPhone's Data Protection. The store is intact and the wallet retries by itself; the SDK's background sync shows this only at the second local fault before a pass completes. MUST NOT suggest restoring from the recovery phrase (that is walletStallInternal, a corrupt store). MUST NOT say 'check your connection' or 'switch servers' (the network is not involved). Keep it short.
  ///
  /// In en, this message translates to:
  /// **'Sync paused on this device. Retrying.'**
  String get walletStallStorageUnavailable;

  /// Stall reason: forward-compatibility arm — still a stall, never healthy.
  ///
  /// In en, this message translates to:
  /// **'Sync stopped for an unknown reason.'**
  String get walletStallUnknown;

  /// Screen-reader tap hint on the sync badge row (the row opens the sync-detail sheet; the ⓘ icon carries the same affordance visually).
  ///
  /// In en, this message translates to:
  /// **'Show sync details'**
  String get walletSyncBadgeHint;

  /// Dismiss button of the sync-detail sheet (sibling of walletShieldClose/walletMoveClose). 'Close', not 'Done' (#356-NIT): the sheet is purely informational — 'Done' implies a completed action.
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get walletSyncSheetClose;

  /// Label of the live scan-percent row in the sync-detail sheet.
  ///
  /// In en, this message translates to:
  /// **'Progress'**
  String get walletSyncSheetProgress;

  /// Label of the live remaining-blocks row in the sync-detail sheet (exact grouped count — the sheet is WHERE the big number belongs; the badge keeps the compact form).
  ///
  /// In en, this message translates to:
  /// **'Blocks left'**
  String get walletSyncSheetBlocksLeft;

  /// Label of the tip row in the sync-detail sheet on EVERY reached-tip state (§4r U-2): plain up to date, and the qualified siblings — limited (this app version), degraded (this server's pools), unverified (this server's network claim) and behind (this server's height). The value is the exact grouped height the pass reached; on the behind state it is THIS SERVER's tip, and walletSyncSheetBehindBy follows it.
  ///
  /// In en, this message translates to:
  /// **'Synced to block'**
  String get walletSyncSheetSyncedTo;

  /// One-cell figure row in the sync-detail sheet under walletSyncSheetSyncedTo on SyncStatus.endpointBehind (§4m #5, §4r U-2): how far behind the network this server is, AT LEAST. count is newestKnown - tip — the newest height this wallet knows the chain reached (a public constant of the app, or its own last scanned height less the reorg allowance) less this server's tip — which is a LOWER BOUND on the server's lag, never the gap itself, so the copy MUST keep 'at least' in every plural case. blocks is the same number pre-formatted as an exact grouped count (e.g. "12,345" — the sheet's vocabulary; the badge keeps compact forms); count selects the plural case only. Rendered only when count >= 1. MUST NOT read as an error or say the funds are lost: the explanation beside it already names the next step (switch servers).
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Behind by at least 1 block} other{Behind by at least {blocks} blocks}}'**
  String walletSyncSheetBehindBy(int count, String blocks);

  /// Sync-detail sheet explanation: genuinely idle (loop not running, no start failure).
  ///
  /// In en, this message translates to:
  /// **'Syncing hasn\'t started yet — it starts automatically. No action needed.'**
  String get walletSyncExplainIdle;

  /// Sync-detail sheet explanation when the sync START command itself failed (#356-F8): replaces the idle arm's 'starts automatically — no action needed', which would contradict the retry notice. Shown above the sheet's Try-again button.
  ///
  /// In en, this message translates to:
  /// **'Syncing couldn\'t start. Your funds are safe — the wallet just isn\'t checking for new activity. Try again below, or reopen the app.'**
  String get walletSyncExplainStartFailed;

  /// Sync-detail sheet explanation: the loop is up but hasn't reported a batch yet (the silent prep phase).
  ///
  /// In en, this message translates to:
  /// **'The wallet is contacting the Zcash network and preparing to scan. This usually takes a few seconds.'**
  String get walletSyncExplainStarting;

  /// Sync-detail sheet explanation: the Connecting arm (incl. Tor bootstrap).
  ///
  /// In en, this message translates to:
  /// **'Establishing a connection to the Zcash network.'**
  String get walletSyncExplainConnecting;

  /// Sync-detail sheet explanation: scanning. Reassures the user the app stays usable (maintainer: 'what's going on' behind the badge).
  ///
  /// In en, this message translates to:
  /// **'The wallet is checking blockchain blocks for your funds. Your balance and activity update as new transactions are found — you can keep using the app while it finishes.'**
  String get walletSyncExplainScanning;

  /// Sync-detail sheet explanation: up to date.
  ///
  /// In en, this message translates to:
  /// **'Fully synced with the Zcash network. Your balance and activity are current.'**
  String get walletSyncExplainUpToDate;

  /// Sync-detail sheet explanation: stalled; the typed walletStall* reason renders as its own paragraph underneath. The endpointUnreachable stall gets walletSyncExplainStalledOffline instead (#399).
  ///
  /// In en, this message translates to:
  /// **'Syncing hit a problem and is paused. It retries automatically.'**
  String get walletSyncExplainStalled;

  /// Sync-detail sheet explanation for the endpointUnreachable stall only (#399): the calm normal-offline story — funds-safe reassurance, queued-sends-are-normal, automatic retry. Carries BOTH hedges itself ('if you're offline' conditional + the server-side possibility) because the sheet suppresses the walletStallEndpoint detail under this explanation (the near-identical-pair rule); the detail still rides the badge a11y label. The queued-sends clause names the SURFACE and no schedule (#401 R1 — host custody cannot drain on a background pass), and the retry sentence says CONNECTION explicitly so it can never be read as a promise about the send. Hard stalls keep walletSyncExplainStalled.
  ///
  /// In en, this message translates to:
  /// **'Can\'t reach the Zcash network — that\'s normal if you\'re offline, or the server may be temporarily unavailable. Your funds are safe: the balance shows the last synced state, and queued sends stay saved under Saved & pending. The connection retries on its own.'**
  String get walletSyncExplainStalledOffline;

  /// Sync-detail sheet explanation: offline; leads with the funds-are-safe reassurance. The queued-sends clause names the SURFACE, never a drain schedule (#401 R1 — host custody cannot drain on a background pass); keep 'Saved & pending' in step with walletParkedTitle.
  ///
  /// In en, this message translates to:
  /// **'No network connection. Your funds are safe — the balance shows the last synced state, and queued sends stay saved under Saved & pending.'**
  String get walletSyncExplainOffline;

  /// Sync-detail sheet explanation: the forward-compat arm — neutral syncing framing, never healthy or alarming.
  ///
  /// In en, this message translates to:
  /// **'The wallet is syncing. Your balance and activity update as it progresses.'**
  String get walletSyncExplainUnknown;

  /// Tor state chip: off by configuration.
  ///
  /// In en, this message translates to:
  /// **'Tor off'**
  String get walletTorOff;

  /// Transport chip: the private path is starting and wallet traffic waits for it. The NEUTRAL variant, used when the SDK has no name to show (nothing registered, or a runtime with no registry) — walletTorBootstrappingNamed carries the host's own name when there is one. Never says "Tor": the wallet names no transport the host did not name (ADR-0547, FR-30 (a)).
  ///
  /// In en, this message translates to:
  /// **'Private path starting…'**
  String get walletTorBootstrapping;

  /// Transport chip: as walletTorBootstrapping, for a REGISTERED transport whose descriptor names it. transport = the HOST'S OWN name for its transport, verbatim ("Tor", "Shadowsocks", "VLESS via Cloudflare") — the wallet never interprets or translates it.
  ///
  /// In en, this message translates to:
  /// **'{transport} starting…'**
  String walletTorBootstrappingNamed(String transport);

  /// Tor state chip: wallet traffic is riding Tor.
  ///
  /// In en, this message translates to:
  /// **'Tor active'**
  String get walletTorActive;

  /// Tor state chip: traffic is on Tor but via a runtime this binding can't attribute; qualified, not a confident protected framing.
  ///
  /// In en, this message translates to:
  /// **'Tor active (unverified runtime)'**
  String get walletTorActiveUnverified;

  /// Transport chip for TorRuntimeKind.dialer — an arbitrary byte-stream dialer a Rust host injected, which the SDK (net/dialer.rs) "never knows or names". FR-32 (a): this arm read "Tor active" in the PROTECTED tone until stage S1 `copy`, asserting onion routing for a path the SDK cannot attest (ADR-0547: the SDK has no predefined transport kinds and renders only what the host declared). It says what IS true — wallet traffic is riding the path the app supplied — and refuses the privacy claim; paired with walletTransportExplainUnverified in the caution tone. A Rust host that KNOWS what it injected says so through WalletHostTransport (label + protection), which wins over any SDK TorState.
  ///
  /// In en, this message translates to:
  /// **'Private path in use (privacy not verified)'**
  String get walletTorActiveUnattested;

  /// Tor state chip: policy preferred and Tor degraded to clearnet (visible, never silent).
  ///
  /// In en, this message translates to:
  /// **'Tor unavailable — using direct connection'**
  String get walletTorFellBack;

  /// Transport chip: zero traffic — the path is GENUINELY DOWN. NARROWED at stage S1 `truth` to a dial that FAILED (refused, unreachable, a dial timeout, a NotReady/Retired/FAILED descriptor) plus the cases where there is no path at all (nothing registered, a registrant that declared its transport FAILED — ABI v3 health, ADR-0549). A dial the transport ACCEPTED and then carried nothing is NOT this state: it reads TorState.unanswered, because the SDK cannot separate a blackholed path from a wedged server and this chip would blame the path. The NEUTRAL variant, used when the SDK has no name to show; walletTorUnavailableNamed carries the host's own name when there is one. Never says "Tor" (ADR-0547, FR-30 (a)); names no policy (FR-32 (b)).
  ///
  /// In en, this message translates to:
  /// **'Private path unavailable — not connected'**
  String get walletTorUnavailable;

  /// Transport chip: as walletTorUnavailable, for a REGISTERED transport whose descriptor names it. transport = the host's own name, verbatim.
  ///
  /// In en, this message translates to:
  /// **'{transport} unavailable — not connected'**
  String walletTorUnavailableNamed(String transport);

  /// Transport CHIP for TorState.unanswered (stage S1 `truth`, FR-36): the path took the connection and no RPC has come back over it for the maintainer's minute while the wallet was trying. It states the two attested facts — the dial was accepted, nothing has answered — and blames neither side: the SDK cannot tell a blackholed path from a wedged server and never guesses (the either/or is spelled out in the sheet, walletTransportExplainUnanswered). The NEUTRAL variant, used when the SDK has no name to show; walletTorUnansweredNamed carries the host's own name when there is one. Never says "Tor" (ADR-0547). Deliberately NOT walletTorUnavailable's wording: that one means the path is genuinely down.
  ///
  /// In en, this message translates to:
  /// **'Private path connected — nothing coming back'**
  String get walletTorUnanswered;

  /// Transport chip for TorState.unanswered on the runtimes that declared NOTHING to branch on — TorRuntimeKind.dialer (an arbitrary byte-stream dialer a Rust host injected, which net/dialer.rs says the SDK "never knows or names") and TorRuntimeKind.unknown. It is walletTorUnanswered plus the refusal that walletTorActiveUnattested already makes for the SAME runtime while traffic is flowing. WHY IT EXISTS (crypto audit HIGH + the product pass, found independently): the unanswered arm sent these two runtimes to the bare walletTorUnanswered, so a path the SDK cannot attest read "Private path in use (privacy not verified)" while it carried and the STRONGER "Private path connected" once it went quiet — a user checking privacy at the moment the path stopped carrying read a bigger claim than while it was working. That is FR-44's defect class, sign-identical, on a different runtime; FR-32 (a) is the rule it breaks. Paired with walletTransportExplainUnansweredUnverified, caution tone. ExternalSocks5 keeps the unqualified walletTorUnanswered (the host named Tor by choosing that variant), as does the attested empty-name hostDialer. STILL OPEN, deliberately: whether the noun "private path" over-claims even with the parenthetical — that question is the same for this string and for walletTorActiveUnattested, and they must change together or not at all.
  ///
  /// In en, this message translates to:
  /// **'Private path connected — nothing coming back (privacy not verified)'**
  String get walletTorUnansweredUnattested;

  /// Transport chip: as walletTorUnanswered, for a REGISTERED transport whose descriptor names it — and ONLY for a path the host declared HIDDEN and isolating, since it is the variant that carries no privacy qualifier. transport = the HOST'S OWN name for its transport, verbatim — the wallet never interprets or translates it (ADR-0547).
  ///
  /// In en, this message translates to:
  /// **'{transport} connected — nothing coming back'**
  String walletTorUnansweredNamed(String transport);

  /// Transport chip for TorState.unanswered on a registered dialer whose descriptor declared its path EXPOSED (ADR-0547 exposure = exposed: the server sees the device's address). FR-44: the unanswered arm inherited Active's payload and not its honesty — it read the plain "connected — nothing coming back" sentence for an exposed path, so a state change silently dropped a privacy loss the Active chip had been disclosing (walletTorHostDirect). The name is discarded here exactly as it is on walletTorHostDirect: what matters is that the path is not private, whatever the host called it and whatever the isolation says. Paired with walletTransportExplainUnansweredDirect, caution tone.
  ///
  /// In en, this message translates to:
  /// **'Not private (your app\'s direct connection) — nothing coming back'**
  String get walletTorUnansweredDirect;

  /// Transport chip for TorState.unanswered on a registered dialer that is NOT the plainly-private case: the host declared isolation unsupported (or did not declare it), so the wallet's connections can be linked to each other at the proxy — or the host did not declare whether the path hides the device's address (exposure unknown), where the weaker sentence is the honest one (the §3.3 privacy rule: never inherit a benign framing for something the binding cannot attest). The twin of walletTorHostPathLinkable on the Active family. transport = the host's own name, verbatim, or walletTorHostOtherTransport when the SDK has none to show — the sentence leads with the state, not the name, so the unattributed fragment reads inside it. Caution tone.
  ///
  /// In en, this message translates to:
  /// **'Connected over {transport} — nothing coming back; connections can be linked by the proxy'**
  String walletTorUnansweredLinkable(String transport);

  /// Tor state chip: forward-compatibility arm; privacy rule renders it as not protected.
  ///
  /// In en, this message translates to:
  /// **'Tor status unknown — treat as not protected'**
  String get walletTorUnknown;

  /// Balance card header when the chain height the balance reflects is known — the as-of block rides IN the header (maintainer), never a separate floating row. The height arrives PRE-GROUPED (exactBlockCount — same format as the sheet's figure rows).
  ///
  /// In en, this message translates to:
  /// **'Balance (as of block {height})'**
  String walletBalanceHeaderAsOf(String height);

  /// Balance card caption (maintainer): the time the balance is as of — just the time when today, date and time otherwise. One short line on a small phone; the block height lives in the sync sheet.
  ///
  /// In en, this message translates to:
  /// **'Balance · {time}'**
  String walletBalanceHeaderAt(String time);

  /// Balance card header when BOTH the as-of height and its stamp time are known — one line, never a second row (maintainer: no per-state layout shift). Height arrives pre-grouped; time pre-formatted.
  ///
  /// In en, this message translates to:
  /// **'Balance (as of block {height}, {time})'**
  String walletBalanceHeaderAsOfAt(String height, String time);

  /// Sync sheet: section header for HOW the wallet talks to the network (transport privacy + server).
  ///
  /// In en, this message translates to:
  /// **'Connection'**
  String get walletSyncSheetConnection;

  /// Sync sheet Connection section: row label for the lightwalletd host the wallet connects to. Since P3-13 the row is a BUTTON when a session exists — it opens the sync-server picker (walletSyncServerRowSemantics is its a11y label).
  ///
  /// In en, this message translates to:
  /// **'Server'**
  String get walletSyncSheetServer;

  /// Screen-reader label of the sync sheet's Server row when it opens the picker (P3-13). host is the lightwalletd HOST in use (never a full URL).
  ///
  /// In en, this message translates to:
  /// **'Server, {host}, opens the server picker'**
  String walletSyncServerRowSemantics(String host);

  /// Title of the sync-server picker sheet (P3-13) and of the switch notice dialog.
  ///
  /// In en, this message translates to:
  /// **'Sync server'**
  String get walletSyncServerSheetTitle;

  /// Picker: the trailing marker on the server row the wallet currently dials.
  ///
  /// In en, this message translates to:
  /// **'In use'**
  String get walletSyncServerInUse;

  /// Picker: the row label for the host app's default server when it is not among the offered entries (choosing it forgets the remembered choice).
  ///
  /// In en, this message translates to:
  /// **'App default'**
  String get walletSyncServerAppDefault;

  /// Picker: the expander that reveals the custom-address field (the maintainer's expert-user path — a regular user never opens it).
  ///
  /// In en, this message translates to:
  /// **'Custom server…'**
  String get walletSyncServerCustom;

  /// Picker: the custom-address field's hint — the shape the SDK's door accepts (https; http only for a local development server).
  ///
  /// In en, this message translates to:
  /// **'https://host:port'**
  String get walletSyncServerCustomHint;

  /// Picker: the action that probes the typed custom server (one round trip under the wallet's own Tor policy) WITHOUT switching.
  ///
  /// In en, this message translates to:
  /// **'Check server'**
  String get walletSyncServerCheck;

  /// Picker: the Check button's label while the probe is in flight (15 s budget).
  ///
  /// In en, this message translates to:
  /// **'Checking…'**
  String get walletSyncServerChecking;

  /// Picker: the action that switches onto a custom server the probe verified; also the trust dialog's confirm action.
  ///
  /// In en, this message translates to:
  /// **'Use this server'**
  String get walletSyncServerUse;

  /// Picker: the progress row while the switch runs (the sync loop stops and joins, the choice is written, the session is rebuilt over the same data).
  ///
  /// In en, this message translates to:
  /// **'Switching…'**
  String get walletSyncServerSwitching;

  /// The switch notice dialog's confirm action.
  ///
  /// In en, this message translates to:
  /// **'Continue'**
  String get walletSyncServerContinue;

  /// The switch and trust notice dialogs' dismiss action — nothing changes.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get walletSyncServerCancel;

  /// Title of the trust notice shown before a CUSTOM server is used for the first time (maintainer ruling 1: validate access, then tell the user they are trusting that server).
  ///
  /// In en, this message translates to:
  /// **'Trust this server?'**
  String get walletSyncServerTrustTitle;

  /// The trust notice's body: what a sync server LEARNS and what it is TRUSTED with. Shown BEFORE the first probe of a custom server (the probe itself discloses the IP — the crypto audit's MEDIUM moved it from the switch to the Check step), never again for the same host in one picker session. Names the privacy consequence the sync guards cannot judge (IP unless Tor, the birthday range, the transparent addresses the wallet polls — the most wallet-identifying item, even under Tor — the txids it fetches for memo enhancement, the broadcasts; the security review widened it from three items to five) and the honesty consequence they do (balance and history as reported). MUST NOT claim the server can move funds — it cannot (no keys).
  ///
  /// In en, this message translates to:
  /// **'You\'re trusting this server to report your balance and history and to relay your payments. It will see your IP address unless Tor is on, roughly when your wallet was created, the public addresses your wallet checks, the transactions it looks up, and the transactions you send.'**
  String get walletSyncServerTrustNotice;

  /// Picker (ADR-0568): the label of the optional key field under a custom server's address — the key the user's own server needs. Obscured; never logged.
  ///
  /// In en, this message translates to:
  /// **'Access key (optional)'**
  String get walletSyncServerKeyLabel;

  /// Picker (ADR-0568): the label of the field for the header name the key goes in (e.g. x-api-key) — shown once a key is typed.
  ///
  /// In en, this message translates to:
  /// **'Key header'**
  String get walletSyncServerKeyHeaderLabel;

  /// Picker (ADR-0568): inline copy when a key is typed without its header name.
  ///
  /// In en, this message translates to:
  /// **'Enter the header your server expects'**
  String get walletSyncServerKeyHeaderNeeded;

  /// Picker refusal copy for WalletErrorKind.invalidEndpointAuth (ADR-0568): the SDK refused the key or its header (a header the transport owns, too long, not printable, padded, or a key for an http:// server). Never the address's copy — the kind tells them apart.
  ///
  /// In en, this message translates to:
  /// **'This key or header can\'t be used'**
  String get walletSyncServerKeyInvalid;

  /// Picker (ADR-0568): shown beside the in-use custom server's host when the wallet holds a key for it. The key itself is never shown or returned.
  ///
  /// In en, this message translates to:
  /// **'Key saved'**
  String get walletSyncServerKeySaved;

  /// Picker (ADR-0568): the key field's toggle — show the typed key.
  ///
  /// In en, this message translates to:
  /// **'Show'**
  String get walletSyncServerKeyShow;

  /// Picker (ADR-0568): the key field's toggle — hide the typed key.
  ///
  /// In en, this message translates to:
  /// **'Hide'**
  String get walletSyncServerKeyHide;

  /// Trust notice addition (ADR-0568), shown when the user gives a key: a personal key lets the server tie every request, including payments sent on a fresh Tor circuit, to one account. MUST say it links payments to the wallet even over Tor.
  ///
  /// In en, this message translates to:
  /// **'Your key identifies you to this server. It can link your payments to your wallet, even over Tor.'**
  String get walletSyncServerTrustNoticeKey;

  /// The in-flight notice body while CONNECTING or SCANNING (before ANY switch): the cost (the pass in progress restarts on the new server), the reassurance (no rescan — balance, history and queued sends are untouched), and the honest caveat (the card may read pending until the new server's scan catches up — fold of the walk). At an up-to-date status the sheet shows walletSyncServerSwitchNoticeAtTip instead.
  ///
  /// In en, this message translates to:
  /// **'Switching restarts the sync in progress. Your balance and history stay. Funds may show as arriving until the new server\'s scan catches up.'**
  String get walletSyncServerSwitchNotice;

  /// The in-flight notice body at an UP-TO-DATE status (nothing is in progress): the switch reconnects; balance and history stay. fold of the walk's observation 2.
  ///
  /// In en, this message translates to:
  /// **'Switching reconnects to the new server. Your balance and history stay.'**
  String get walletSyncServerSwitchNoticeAtTip;

  /// Picker refusal copy for WalletErrorKind.syncServerUnreachable when the user TYPED this address (the custom-URL field): the probe could not dial, timed out, or was refused — a gated server rejecting the key lands here too. Hedged like walletStallEndpoint: it MUST NOT say 'check your connection' alone — the server may be the down side. Nothing changed: the wallet stays on its current server. WHY IT NO LONGER STOPS AT 'check the address' (stage S1 `copy`, from the `truth` re-adjudication): probe_oracle (wallet.rs) maps everything that is not a FAILED private dial onto this kind, and since stage S1 a private path that ACCEPTS the dial and then carries nothing no longer reports TorUnavailable — deliberately, because blaming the path for what cannot be separated from a wedged server is the over-claim the stage removed. So a censored path and a wedged server arrive here as the same error and the SDK cannot tell them apart; the address stays the first step (this reader typed it) but it is no longer the ONLY one. Its sibling walletSyncServerUnreachableOffered serves the reader who typed nothing.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t reach this server. Check the address — and if it\'s right, either this server isn\'t answering or your app can\'t reach it right now. Try again, or pick another server.'**
  String get walletSyncServerUnreachable;

  /// Picker refusal copy for WalletErrorKind.syncServerUnreachable when the address came from the APP'S OWN LIST (a predefined or default choice) rather than from the user — stage S1 `copy`. Same error, different reader: 'check the address' is dead advice for someone who typed nothing, and it is the sentence that would meet a `Required` wallet whose private path is being censored, where the address is the one thing that is certainly fine. The either/or is the point and must survive translation — the SDK cannot separate a wedged server from a path that accepts connections and carries nothing, and it does not guess; it names both causes and the two steps the reader can actually take. Names no transport (ADR-0547) and no policy (FR-32 (b)).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t reach this server. The wallet can\'t tell whether this server isn\'t answering or your app can\'t reach it right now. Pick another server, or try again later.'**
  String get walletSyncServerUnreachableOffered;

  /// Picker refusal copy for WalletErrorKind.networkMismatch: the server answered and claims another network (testnet under a mainnet wallet). Not recoverable for THAT server; nothing changed.
  ///
  /// In en, this message translates to:
  /// **'This server is on a different Zcash network.'**
  String get walletSyncServerWrongNetwork;

  /// Picker refusal copy for WalletErrorKind.invalidEndpoint on a custom address (not https, a username or password in it, a path, too long, no host). ONE copy for every reason: the SDK's reason string is a static code the UI never echoes (the FFI rule — no matching on error text).
  ///
  /// In en, this message translates to:
  /// **'That doesn\'t look like a server address. Use https://host:port.'**
  String get walletSyncServerInvalidUrl;

  /// Picker refusal copy for WalletErrorKind.syncServerNotOffered — a host bug (the picker renders only offered entries), kept honest rather than silent.
  ///
  /// In en, this message translates to:
  /// **'This server isn\'t offered by this app.'**
  String get walletSyncServerNotOffered;

  /// Picker refusal copy for WalletErrorKind.walletBusy in another phase (a rescan or a close in flight): retryable, nothing changed.
  ///
  /// In en, this message translates to:
  /// **'The wallet is busy right now. Try again in a moment.'**
  String get walletSyncServerBusy;

  /// Banner on the sync sheet's Server row and the picker when the REMEMBERED choice names a server this app version no longer offers (SyncServerFallback.choiceNotOffered): the default is in use, said, never silent. host = the server now in use.
  ///
  /// In en, this message translates to:
  /// **'The server you chose isn\'t offered by this app any more. Using {host}.'**
  String walletSyncServerFallbackNotOffered(String host);

  /// Banner for SyncServerFallback.choiceUnreadable (a malformed remembered choice): the wallet is usable on the default; pick again to replace it. host = the server now in use.
  ///
  /// In en, this message translates to:
  /// **'The remembered server choice couldn\'t be read. Using {host}.'**
  String walletSyncServerFallbackUnreadable(String host);

  /// Picker notice after a switch failed PAST the point of no return and the wallet was recovered by re-opening (the rescan's recover-by-reopen). Names the server actually in use after the re-open (the new one if the choice landed, the previous one otherwise). Funds and history are untouched either way.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t switch — still using {host}.'**
  String walletSyncServerSwitchFailedRecovered(String host);

  /// Sync sheet Connection explanation: no privacy transport — a direct (clearnet) connection.
  ///
  /// In en, this message translates to:
  /// **'Wallet traffic connects directly to the server. The server can see your IP address.'**
  String get walletTransportExplainDirect;

  /// Sync sheet Connection explanation: verified built-in Tor is active.
  ///
  /// In en, this message translates to:
  /// **'Wallet traffic is routed through the Tor network, which hides your IP address from the server.'**
  String get walletTransportExplainTor;

  /// Sync sheet Connection explanation: the private path is starting. The NEUTRAL variant (no name to show); walletTransportExplainBootstrappingNamed names the host's transport when the descriptor does.
  ///
  /// In en, this message translates to:
  /// **'Your app\'s private path is starting up. Wallet traffic waits for it before connecting.'**
  String get walletTransportExplainBootstrapping;

  /// Sync sheet Connection explanation: as walletTransportExplainBootstrapping, naming the host's own transport verbatim.
  ///
  /// In en, this message translates to:
  /// **'{transport} is starting up. Wallet traffic waits for it before connecting.'**
  String walletTransportExplainBootstrappingNamed(String transport);

  /// Sync sheet Connection explanation: Tor-preferred policy fell back to clearnet.
  ///
  /// In en, this message translates to:
  /// **'Tor couldn\'t be reached, so traffic fell back to a direct connection. The server can see your IP address.'**
  String get walletTransportExplainFellBack;

  /// Sync sheet Connection explanation: zero traffic — the path is unreachable, unregistered or declared FAILED. Carries the NEXT STEP (ADR-0549 D3) — a failed path is no longer a wait, so the sentence must not read as one. The NEUTRAL variant (no name to show). It says "IS REQUIRED" NOWHERE (FR-32 (b), stage S1 `copy`): TorState.Unavailable carries no policy and a `Preferred` wallet reaches it too — a registrant that declared its transport FAILED is a frequent producer — so the sentence states what is true (the path is not available and nothing is connecting) without asserting a setting the user may not have chosen. It makes no claim about clearnet either: since stage S1 a `Preferred` wallet leaves a FAILED path after the minute, so "nothing was sent in the clear" would be a promise this state cannot keep.
  ///
  /// In en, this message translates to:
  /// **'Your app\'s private path isn\'t available, so the wallet isn\'t connecting. Turn the private path off, or check your app\'s network settings.'**
  String get walletTransportExplainUnavailable;

  /// Sync sheet Connection explanation: as walletTransportExplainUnavailable, naming the host's own transport verbatim.
  ///
  /// In en, this message translates to:
  /// **'{transport} isn\'t available, so the wallet isn\'t connecting. Turn it off, or check your app\'s network settings.'**
  String walletTransportExplainUnavailableNamed(String transport);

  /// Sync sheet Connection explanation for TorState.unanswered (stage S1 `truth`, FR-36). The either/or is the POINT and must survive translation: the SDK reports what its own RPCs saw over a connection whose arm it knows, and it never says WHY a ready path is not carrying — a blackholed transport and a wedged server look identical from here. TWO next steps for the two causes (the walletStallBirthdayInFuture discipline): another server for a wedged server, the app's network settings for the path. MUST NOT promise that nothing left in the clear — a `Preferred` wallet reads this too, and its next dial leaves for clearnet. The NEUTRAL variant (no name to show).
  ///
  /// In en, this message translates to:
  /// **'The private path took the connection, but nothing has come back for a minute. It could be the path or the wallet server — the wallet can\'t tell which. It keeps trying; if it doesn\'t clear, try another server or check your app\'s network settings.'**
  String get walletTransportExplainUnanswered;

  /// Sync sheet Connection explanation: as walletTransportExplainUnanswered, naming the host's own transport verbatim. Only the FIRST clause takes the name — the either/or still says "the path", because naming the host's transport a second time would read as an accusation of it.
  ///
  /// In en, this message translates to:
  /// **'{transport} took the connection, but nothing has come back for a minute. It could be the path or the wallet server — the wallet can\'t tell which. It keeps trying; if it doesn\'t clear, try another server or check your app\'s network settings.'**
  String walletTransportExplainUnansweredNamed(String transport);

  /// Sync sheet Connection explanation for TorState.unanswered on an EXPOSED registered path (FR-44). Two facts, in this order: the privacy verdict the descriptor decides (walletTransportExplainDirect's sentence, verbatim — the server sees the device's address) and then the unanswered either/or with its two next steps, which survive here unchanged because the state means the same thing whatever the exposure. The subject is "the connection", never "the private path": naming this path private is the FR-44 defect.
  ///
  /// In en, this message translates to:
  /// **'Wallet traffic connects directly to the server. The server can see your IP address. The connection was accepted, but nothing has come back for a minute. It could be the path or the wallet server — the wallet can\'t tell which. It keeps trying; if it doesn\'t clear, try another server or check your app\'s network settings.'**
  String get walletTransportExplainUnansweredDirect;

  /// Sync sheet Connection explanation for TorState.unanswered on a registered path whose EXPOSURE the host did not declare, or declared with a value this binding cannot read (FR-44). walletTransportExplainUnverified's verdict, verbatim, then the unanswered either/or with its two next steps. The Active family renders walletTransportExplainUnverified alone on the same payload; this is the same verdict with the state's own sentence after it.
  ///
  /// In en, this message translates to:
  /// **'The privacy of this connection can\'t be verified — treat it as not private. The connection was accepted, but nothing has come back for a minute. It could be the path or the wallet server — the wallet can\'t tell which. It keeps trying; if it doesn\'t clear, try another server or check your app\'s network settings.'**
  String get walletTransportExplainUnansweredUnverified;

  /// Sync sheet Connection explanation: an unverifiable/unknown transport state (privacy rule: never claim protection).
  ///
  /// In en, this message translates to:
  /// **'The privacy of this connection can\'t be verified — treat it as not private.'**
  String get walletTransportExplainUnverified;

  /// Sync sheet Connection explanation: generic copy for a HOST-provided protective transport (e.g. xray/vless/VPN) when the host supplies no detail of its own.
  ///
  /// In en, this message translates to:
  /// **'Wallet traffic is routed through this app\'s privacy transport, which hides your IP address from the server.'**
  String get walletTransportExplainHostProxy;

  /// Heading on the wallet onboarding welcome screen.
  ///
  /// In en, this message translates to:
  /// **'Set up your wallet'**
  String get walletOnboardingWelcomeTitle;

  /// Body on the wallet onboarding welcome screen; states the money-safety reason backup comes first.
  ///
  /// In en, this message translates to:
  /// **'Create a new wallet to receive and hold ZEC. We\'ll generate a recovery phrase and walk you through backing it up before any funds can arrive — so nothing is ever at risk without a backup.'**
  String get walletOnboardingWelcomeBody;

  /// Primary action on the welcome screen: start creating a brand-new wallet.
  ///
  /// In en, this message translates to:
  /// **'Create a new wallet'**
  String get walletCreateButton;

  /// Secondary action on the welcome screen: restore an existing wallet from its recovery phrase.
  ///
  /// In en, this message translates to:
  /// **'Restore from a recovery phrase'**
  String get walletRestoreButton;

  /// Welcome-screen action (#397) to import a watch-only wallet from a viewing key — it can see balance and history but cannot spend.
  ///
  /// In en, this message translates to:
  /// **'Watch a wallet (view-only)'**
  String get walletWatchOnlyButton;

  /// Heading on the watch-only import screen.
  ///
  /// In en, this message translates to:
  /// **'Watch a wallet'**
  String get walletWatchOnlyTitle;

  /// Intro on the watch-only import screen: explains a viewing key gives view-only access and that a start date is needed.
  ///
  /// In en, this message translates to:
  /// **'Paste a viewing key to watch a wallet without its spending keys. You\'ll see its balance and history, but you won\'t be able to send funds. Pick the wallet\'s approximate start date so we know how far back to look.'**
  String get walletWatchOnlyBody;

  /// Label for the text field where the user pastes the unified viewing key.
  ///
  /// In en, this message translates to:
  /// **'Viewing key'**
  String get walletWatchOnlyKeyLabel;

  /// Placeholder hint showing the expected viewing-key prefix.
  ///
  /// In en, this message translates to:
  /// **'uview1…'**
  String get walletWatchOnlyKeyHint;

  /// Tooltip/a11y label of the camera-scan icon button on the viewing-key field (mobile only).
  ///
  /// In en, this message translates to:
  /// **'Scan a viewing key QR code'**
  String get walletWatchOnlyScanTooltip;

  /// App-bar title of the full-screen QR reader when scanning a viewing key.
  ///
  /// In en, this message translates to:
  /// **'Scan viewing key'**
  String get walletWatchOnlyScanTitle;

  /// Aiming hint overlaid on the camera preview when scanning a viewing key. Mirrors walletSwapScanInstruction.
  ///
  /// In en, this message translates to:
  /// **'Point your camera at the viewing key QR code.'**
  String get walletWatchOnlyScanInstruction;

  /// Shown on the QR reader when the camera cannot start (permission denied / no camera); the manual-entry escape button is always present. Mirrors walletSwapScanCameraUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Camera unavailable. Paste the key manually instead.'**
  String get walletWatchOnlyScanCameraUnavailable;

  /// The always-present escape button on the viewing-key QR reader — returns to the import form to PASTE the key. The viewing-key counterpart of walletSwapScanManualEntry ('Enter manually'); a viewing key is pasted, not typed, so this must say paste, matching walletWatchOnlyScanCameraUnavailable.
  ///
  /// In en, this message translates to:
  /// **'Paste key instead'**
  String get walletWatchOnlyScanManualEntry;

  /// A muted one-line hint under the viewing-key input field, shown only on camera platforms (Android/iOS). Points the user at the scan icon so the QR-scan affordance is discoverable when the body copy is paste-first. Keep it short.
  ///
  /// In en, this message translates to:
  /// **'Or tap the scan button to read a viewing key QR code.'**
  String get walletWatchOnlyScanHint;

  /// Screen-reader announcement (not visible on screen) made after a QR scan fills the viewing-key field, so a non-sighted user knows the scan landed. NEVER contains the key itself. Keep it short and past-tense.
  ///
  /// In en, this message translates to:
  /// **'Viewing key scanned.'**
  String get walletWatchOnlyScanFilled;

  /// Heading of the required creation-date control on the watch-only import screen.
  ///
  /// In en, this message translates to:
  /// **'Wallet start date'**
  String get walletWatchOnlyBirthdayTitle;

  /// The chosen watch-only start month/year, carrying the honest warning that older funds won't appear (a watch-only import always scans from a floor — no full-scan arm — so the auditor importing an older wallet must not silently see an understated balance). Mirrors walletRestoreBirthdayChosen without the 'Scan all history' clause.
  ///
  /// In en, this message translates to:
  /// **'Scanning from {date} on — funds received before then won\'t appear. Older wallet? Pick an earlier date.'**
  String walletWatchOnlyBirthdayChosen(String date);

  /// Date-picker help text on the watch-only import screen.
  ///
  /// In en, this message translates to:
  /// **'Pick the wallet\'s start date'**
  String get walletWatchOnlyBirthdayPick;

  /// Button to change the chosen watch-only start date.
  ///
  /// In en, this message translates to:
  /// **'Change date'**
  String get walletWatchOnlyBirthdayChange;

  /// Confirm action that imports the watch-only wallet from the pasted viewing key.
  ///
  /// In en, this message translates to:
  /// **'Watch this wallet'**
  String get walletWatchOnlySubmit;

  /// Return from the watch-only import screen to the welcome screen.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get walletWatchOnlyBack;

  /// Inline fault when the entered string is not a valid unified viewing key. MODE-NEUTRAL wording: the key may have been pasted OR scanned from a QR, so it must not say 'paste again'.
  ///
  /// In en, this message translates to:
  /// **'That doesn\'t look like a valid viewing key. Check it and try again.'**
  String get walletWatchOnlyFaultInvalidKey;

  /// Inline fault when a well-formed viewing key is for the wrong network (mainnet vs testnet).
  ///
  /// In en, this message translates to:
  /// **'That viewing key is for a different network. It can\'t be used here.'**
  String get walletWatchOnlyFaultNetworkMismatch;

  /// Inline fault when a wallet already exists and a watch-only import was attempted over it.
  ///
  /// In en, this message translates to:
  /// **'A wallet already exists on this device. Go back and open it instead.'**
  String get walletWatchOnlyFaultAlreadyExists;

  /// Inline fault when the chosen watch-only start date is above the chain tip.
  ///
  /// In en, this message translates to:
  /// **'That start date is too recent. Pick an earlier date.'**
  String get walletWatchOnlyFaultBirthdayTooRecent;

  /// Heading on the restore (recovery-phrase entry) screen.
  ///
  /// In en, this message translates to:
  /// **'Restore your wallet'**
  String get walletRestoreTitle;

  /// Body on the restore screen explaining what to enter, with an honest note that passphrase-protected ('25th word') wallets can't be imported (a wrong/absent passphrase silently restores a different, empty wallet).
  ///
  /// In en, this message translates to:
  /// **'Enter your recovery phrase to restore your wallet — type or paste the words in order, separated by spaces. Standard phrases only: if your wallet used an extra passphrase (a \"25th word\"), this app can\'t restore it yet — you\'d see an empty wallet, not an error.'**
  String get walletRestoreBody;

  /// Placeholder hint inside the recovery-phrase field, showing words are space-separated.
  ///
  /// In en, this message translates to:
  /// **'word one  word two  word three  …'**
  String get walletRestorePhraseHint;

  /// Live count of recovery words entered.
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =0{No words yet} =1{1 word} other{{count} words}}'**
  String walletRestoreWordCount(int count);

  /// Hint shown beside the word count when it isn't yet a valid phrase length.
  ///
  /// In en, this message translates to:
  /// **'recovery phrases have 12, 15, 18, 21, or 24 words'**
  String get walletRestoreLengthHint;

  /// Live cue when one or more entered words aren't in the BIP39 list (shown as error-coloured pills).
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{1 word isn\'t a recovery word — fix the highlighted one} other{{count} words aren\'t recovery words — fix the highlighted ones}}'**
  String walletRestoreSomeWordsInvalid(int count);

  /// Screen-reader label for a recovery-word pill (valid word).
  ///
  /// In en, this message translates to:
  /// **'word {index}: {word}'**
  String walletRestorePillSemantics(int index, String word);

  /// Screen-reader label for a recovery-word pill that isn't a BIP39 word. Deliberately OMITS the typed value: an invalid token carries no verification value to read back, and keeping it out of the OS accessibility tree avoids echoing a mistyped recovery word to assistive/automation services (security HARDENING).
  ///
  /// In en, this message translates to:
  /// **'word {index}: not a recovery word'**
  String walletRestorePillSemanticsInvalid(int index);

  /// Screen-reader label for the × that removes a recovery-word pill.
  ///
  /// In en, this message translates to:
  /// **'Remove word {index}'**
  String walletRestoreRemoveWord(int index);

  /// Primary action on the restore screen: validate the phrase and restore the wallet.
  ///
  /// In en, this message translates to:
  /// **'Restore wallet'**
  String get walletRestoreSubmit;

  /// Secondary action on the restore screen: return to the welcome screen.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get walletRestoreBack;

  /// Heading of the restore starting-point (wallet-creation date) control.
  ///
  /// In en, this message translates to:
  /// **'How far back to scan'**
  String get walletRestoreBirthdayTitle;

  /// Shown when the user chose a full scan (no date): money-safe but slower.
  ///
  /// In en, this message translates to:
  /// **'We\'ll scan your whole history — slower, but nothing is missed.'**
  String get walletRestoreBirthdayNone;

  /// Shown when a creation date is set (the ~6-months-ago default). States the exclusion as a FACT and names both escape hatches (earlier date / full scan).
  ///
  /// In en, this message translates to:
  /// **'Scanning from {date} on — funds received before then won\'t appear. Older wallet? Pick an earlier date, or Scan all history.'**
  String walletRestoreBirthdayChosen(String date);

  /// Action to choose the approximate wallet-creation date.
  ///
  /// In en, this message translates to:
  /// **'Pick a date'**
  String get walletRestoreBirthdayPick;

  /// Action to change an already-chosen creation date.
  ///
  /// In en, this message translates to:
  /// **'Change date'**
  String get walletRestoreBirthdayChange;

  /// Action to clear the chosen creation date and scan the whole chain instead (money-safe, slower).
  ///
  /// In en, this message translates to:
  /// **'Scan all history'**
  String get walletRestoreBirthdayClear;

  /// Inline restore error when one word isn't in the BIP39 list; index is 1-based.
  ///
  /// In en, this message translates to:
  /// **'Word {index} isn\'t a recovery word. Check your phrase for typos, then try again.'**
  String walletRestoreFaultInvalidWord(int index);

  /// Inline restore error when the phrase fails validation with no single offending word (e.g. a checksum failure).
  ///
  /// In en, this message translates to:
  /// **'That recovery phrase isn\'t valid. Check the words and their order, then try again.'**
  String get walletRestoreFaultInvalidPhrase;

  /// Inline restore error when a different phrase is supplied over an interrupted-create remnant.
  ///
  /// In en, this message translates to:
  /// **'That phrase doesn\'t match the wallet on this device. Double-check it and try again.'**
  String get walletRestoreFaultSeedMismatch;

  /// Inline restore error when a completed wallet is already provisioned here.
  ///
  /// In en, this message translates to:
  /// **'A wallet already exists on this device. Go back to open it.'**
  String get walletRestoreFaultAlreadyExists;

  /// Inline restore error when the chosen creation date is past the chain tip.
  ///
  /// In en, this message translates to:
  /// **'That date is too recent. Pick an earlier date, or scan everything.'**
  String get walletRestoreFaultBirthdayTooRecent;

  /// Busy label while the wallet seed is generated and sealed (a local step, no network).
  ///
  /// In en, this message translates to:
  /// **'Creating your wallet…'**
  String get walletGeneratingLabel;

  /// Busy label for the boot probe / open phase — usually brief, but the open lawfully waits out a transiently-held wallet lock for up to ~27.5s, so the wait must be labeled, not a mute spinner.
  ///
  /// In en, this message translates to:
  /// **'Opening your wallet…'**
  String get walletOpeningLabel;

  /// Heading on the recovery-phrase backup screen.
  ///
  /// In en, this message translates to:
  /// **'Back up your recovery phrase'**
  String get walletBackupTitle;

  /// Explanation on the backup screen of what the recovery phrase is and the rules for handling it.
  ///
  /// In en, this message translates to:
  /// **'These words are the ONLY way to recover your wallet and funds. Write them down in order and keep them somewhere safe and private. Never share them or store them online — anyone with these words can take your funds.'**
  String get walletBackupBody;

  /// Security note on Android, where the screen is marked secure (no screenshots).
  ///
  /// In en, this message translates to:
  /// **'Screenshots are turned off on this screen.'**
  String get walletBackupSecureNoteAndroid;

  /// Security note on platforms with no screenshot-block; advises a private setting.
  ///
  /// In en, this message translates to:
  /// **'Make sure no one can see your screen.'**
  String get walletBackupSecureNoteOther;

  /// Deliberate-action button to reveal the recovery words (shoulder-surfing mitigation: words are hidden until tapped).
  ///
  /// In en, this message translates to:
  /// **'Reveal recovery phrase'**
  String get walletBackupReveal;

  /// Busy label while the recovery words are read from the wallet.
  ///
  /// In en, this message translates to:
  /// **'Preparing your recovery phrase…'**
  String get walletBackupRevealing;

  /// Honest, plain-language error when the recovery words can't be read (e.g. device locked); no error code.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t show your recovery phrase right now. Make sure your device is unlocked, then try again.'**
  String get walletBackupRevealFailed;

  /// Retry action after a failed recovery-phrase reveal.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletBackupRetryReveal;

  /// Honest error when the host re-authentication step failed (e.g. a wrong passphrase) before the recovery words were shown; distinct from a device-locked read failure, so it does not tell the user to unlock their device.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t verify it\'s you. Please try again.'**
  String get walletBackupReauthFailed;

  /// Deliberate confirmation the user toggles before the wallet can become deposit-ready (the money-safety gate).
  ///
  /// In en, this message translates to:
  /// **'I\'ve written down my recovery phrase and stored it safely.'**
  String get walletBackupConfirmCheckbox;

  /// Action that confirms the backup and opens the wallet for deposits; enabled only once the confirmation box is checked.
  ///
  /// In en, this message translates to:
  /// **'Continue'**
  String get walletBackupContinue;

  /// Honest error when persisting the backup confirmation fails; the wallet stays not-yet-ready (the gate stays closed).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t save your confirmation. Please try again.'**
  String get walletBackupSaveFailed;

  /// Understated escape action on the forced-backup screen (#356-F2): deletes the not-yet-backed-up wallet (after a confirm dialog) and returns to the create/restore start, so a user who mis-tapped Create is never cornered into falsely confirming a backup.
  ///
  /// In en, this message translates to:
  /// **'Start over'**
  String get walletBackupStartOver;

  /// Title of the Start-over confirmation dialog on the forced-backup screen.
  ///
  /// In en, this message translates to:
  /// **'Start over without this wallet?'**
  String get walletBackupStartOverConfirmTitle;

  /// Body of the Start-over confirmation dialog. Must stay honest for EVERY way of reaching forced backup: a fresh create (nothing deposited via this app), the rare restore stranded here by a confirm-persist failure, AND the residual lost-flag edge on a wallet that was briefly active (review) — hence the hedged 'if this wallet ever held funds' phrase-warning instead of an absolute 'never received anything' claim.
  ///
  /// In en, this message translates to:
  /// **'This deletes this wallet from the device and returns you to the start. Nothing can be deposited through this app before setup is finished.\n\nIf this wallet ever held funds — or was restored from a recovery phrase — only that phrase can bring it back.'**
  String get walletBackupStartOverConfirmBody;

  /// Destructive confirm action of the Start-over dialog (rendered in the destructive color).
  ///
  /// In en, this message translates to:
  /// **'Delete and start over'**
  String get walletBackupStartOverConfirm;

  /// Safe cancel action of the Start-over dialog — says what it keeps rather than a bare 'Cancel', so a mis-tap defaults to no loss.
  ///
  /// In en, this message translates to:
  /// **'Keep this wallet'**
  String get walletBackupStartOverKeep;

  /// Section header on the Security settings screen for the recovery-phrase backup entry.
  ///
  /// In en, this message translates to:
  /// **'Recovery phrase'**
  String get walletBackupSectionTitle;

  /// Title of the settings tile that opens the post-onboarding recovery-phrase backup screen.
  ///
  /// In en, this message translates to:
  /// **'Back up your recovery phrase'**
  String get walletBackupTileTitle;

  /// Subtitle of the settings tile that opens the recovery-phrase backup screen.
  ///
  /// In en, this message translates to:
  /// **'Show the words that can recover your wallet and funds.'**
  String get walletBackupTileSubtitle;

  /// App-bar title of the post-onboarding recovery-phrase backup screen.
  ///
  /// In en, this message translates to:
  /// **'Recovery phrase'**
  String get walletBackupScreenTitle;

  /// Action that closes the recovery-phrase backup screen after the words have been viewed (view-only; nothing is saved).
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get walletBackupDone;

  /// Heading of the managed-by-host state, shown when the wallet was set up from a host-supplied seed and has no recovery phrase of its own.
  ///
  /// In en, this message translates to:
  /// **'No separate recovery phrase'**
  String get walletBackupManagedTitle;

  /// Explanation shown when the wallet has no local recovery phrase (a host-managed / raw seed); tells the user their recovery lives with the installing app's account.
  ///
  /// In en, this message translates to:
  /// **'This wallet was set up using your account from the app that installed it, so it has no recovery phrase of its own. Your funds are recovered together with that account — use its backup to keep them safe.'**
  String get walletBackupManagedBody;

  /// App-bar title of the UFVK export screen (#397): the sanctioned surface that reveals the wallet's unified full viewing key for a watch-only or accounting use.
  ///
  /// In en, this message translates to:
  /// **'Export viewing key'**
  String get walletExportViewingKeyTitle;

  /// Settings → Security list-tile that opens the viewing-key export screen.
  ///
  /// In en, this message translates to:
  /// **'Export viewing key'**
  String get walletExportViewingKeyTileTitle;

  /// Subtitle of the export-viewing-key settings tile, summarizing that the exported key is view-only.
  ///
  /// In en, this message translates to:
  /// **'Share a view-only copy of your wallet — it can see your history but cannot spend.'**
  String get walletExportViewingKeyTileSubtitle;

  /// The #397 D9 warning copy shown on the export-viewing-key screen. Must state all four facts: (1) it reveals all history in and out, past and future; (2) it cannot spend or recover; (3) share only with someone trusted; (4) the only un-share is moving funds to a new wallet. No softening.
  ///
  /// In en, this message translates to:
  /// **'This key lets whoever holds it see everything this wallet has ever received and sent — and everything it will in the future. It cannot spend your funds and cannot recover your wallet. Share it only with someone you trust to see your full history, such as an accountant or your own second device. The only way to un-share it later is to move your funds to a new wallet.'**
  String get walletExportViewingKeyWarning;

  /// The #397 D9 warning for a WATCH-ONLY wallet exporting its own viewing key (UX-M3): identical to walletExportViewingKeyWarning but its final clause states the honest 'once shared it cannot be un-shared', since a watch-only wallet cannot 'move your funds to a new wallet' (no spending keys). Keep the same four facts, no softening.
  ///
  /// In en, this message translates to:
  /// **'This key lets whoever holds it see everything this wallet has ever received and sent — and everything it will in the future. It cannot spend any funds and cannot recover the wallet. Share it only with someone you trust to see your full history, such as an accountant or your own second device. Once shared, it cannot be un-shared.'**
  String get walletExportViewingKeyWarningWatchOnly;

  /// Button that triggers re-authentication and then reveals the viewing key on the export screen.
  ///
  /// In en, this message translates to:
  /// **'Show viewing key'**
  String get walletExportViewingKeyReveal;

  /// Retry action after a failed re-auth or a failed viewing-key read on the export screen.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletExportViewingKeyRetry;

  /// Loading line shown while the viewing key is being read for export.
  ///
  /// In en, this message translates to:
  /// **'Preparing your viewing key…'**
  String get walletExportViewingKeyRevealing;

  /// Honest error line shown when reading the viewing key for export failed transiently.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t show your viewing key right now. Try again in a moment.'**
  String get walletExportViewingKeyFailed;

  /// Accessibility label for the QR tile encoding the exported viewing key.
  ///
  /// In en, this message translates to:
  /// **'Viewing key QR code'**
  String get walletExportViewingKeyQrLabel;

  /// Button that copies the exported viewing key to the clipboard.
  ///
  /// In en, this message translates to:
  /// **'Copy viewing key'**
  String get walletExportViewingKeyCopy;

  /// Confirmation shown after the viewing key is copied to the clipboard.
  ///
  /// In en, this message translates to:
  /// **'Viewing key copied'**
  String get walletExportViewingKeyCopied;

  /// Action that closes the export-viewing-key screen.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get walletExportViewingKeyDone;

  /// Note shown on the export-viewing-key screen when screenshot blocking is active.
  ///
  /// In en, this message translates to:
  /// **'Screenshots are turned off on this screen.'**
  String get walletExportViewingKeySecureNoteAndroid;

  /// Note shown on the export-viewing-key screen when screenshot blocking is not available (non-Android).
  ///
  /// In en, this message translates to:
  /// **'Make sure no one can see your screen.'**
  String get walletExportViewingKeySecureNoteOther;

  /// Security-screen section header shown instead of the backup section for a watch-only wallet.
  ///
  /// In en, this message translates to:
  /// **'About this watch-only wallet'**
  String get walletWatchOnlySectionTitle;

  /// Security-screen explanation for a watch-only wallet: it has no spending keys and nothing to back up.
  ///
  /// In en, this message translates to:
  /// **'This is a watch-only wallet. It was set up from a viewing key, so it can see your balance and history but holds no spending keys — there is nothing to back up here, and it cannot send funds.'**
  String get walletWatchOnlyAboutBody;

  /// Short header badge marking a watch-only wallet (no spending keys).
  ///
  /// In en, this message translates to:
  /// **'Watch-only'**
  String get walletWatchOnlyBadge;

  /// Heading on the onboarding failure screen.
  ///
  /// In en, this message translates to:
  /// **'Wallet setup couldn\'t finish'**
  String get walletOnboardingFailedTitle;

  /// Retry action shown for recoverable onboarding failures.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletOnboardingRetry;

  /// Onboarding failure: device locked, or the key store did not answer within the SDK's bound (retryable; a wedge clears only on restart).
  ///
  /// In en, this message translates to:
  /// **'Your phone\'s secure storage isn\'t responding. Unlock your device and try again. If this keeps happening, restart your phone.'**
  String get walletOnboardingFailedDeviceLocked;

  /// Onboarding failure: the wallet's single-instance lock is held (retryable). Honest for BOTH causes — a real second window AND a transient internal straggler finishing up (the device-proven case, where no other window exists).
  ///
  /// In en, this message translates to:
  /// **'This wallet is open in another window or app, or is still finishing a previous operation. Close any other window using it — or wait a moment — then try again.'**
  String get walletOnboardingFailedAlreadyOpen;

  /// Onboarding failure: damaged seal / destroyed key / corrupt storage — restore is the path, not retry. Funds are recoverable from the phrase.
  ///
  /// In en, this message translates to:
  /// **'This wallet\'s secure key is no longer available, so it can\'t be opened on this device. Your funds are safe — restore from your recovery phrase to recover them.'**
  String get walletOnboardingFailedNeedsRecovery;

  /// Primary action on the needsRecovery failure screen: clears the unreadable wallet and routes to the restore flow (#251 escape hatch).
  ///
  /// In en, this message translates to:
  /// **'Restore from recovery phrase'**
  String get walletOnboardingFailedRestoreAction;

  /// Title of the confirm dialog before the deliberate force-clear + restore.
  ///
  /// In en, this message translates to:
  /// **'Restore this wallet?'**
  String get walletOnboardingRecoverConfirmTitle;

  /// Body of the confirm dialog: WARN the user they must have their recovery phrase in hand, then reassure funds are safe (phrase-controlled), before the irreversible force-clear.
  ///
  /// In en, this message translates to:
  /// **'Make sure you have your recovery phrase before continuing — you\'ll need it on the next screen to recover your funds. Your funds are safe on the blockchain and controlled by that phrase. This removes the unreadable wallet data from this device so it can be rebuilt.'**
  String get walletOnboardingRecoverConfirmBody;

  /// Cancel action in the restore-confirm dialog.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get walletOnboardingRecoverConfirmCancel;

  /// Onboarding failure: out of disk space (retryable).
  ///
  /// In en, this message translates to:
  /// **'There isn\'t enough free space to set up your wallet. Free up some space and try again.'**
  String get walletOnboardingFailedStorageFull;

  /// Onboarding failure: no device key vault at all (fail-closed; permanent for this device).
  ///
  /// In en, this message translates to:
  /// **'This device has no secure key store, so the wallet can\'t protect your recovery phrase here.'**
  String get walletOnboardingFailedNoVault;

  /// Onboarding failure: a network condition during setup (retryable).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t reach the network during setup. Check your connection and try again.'**
  String get walletOnboardingFailedNetwork;

  /// Onboarding failure: an interrupted create (remnant); retrying re-probes and resumes the repair via create (open cannot repair it). Retryable, reassuring (the sealed seed is intact).
  ///
  /// In en, this message translates to:
  /// **'Wallet setup didn\'t finish. Try again to complete it — nothing was lost.'**
  String get walletOnboardingFailedInterruptedSetup;

  /// Onboarding failure: anything else / a forward-compat kind (retryable; the stable code rides logs).
  ///
  /// In en, this message translates to:
  /// **'Something went wrong setting up your wallet. Try again.'**
  String get walletOnboardingFailedUnknown;

  /// Onboarding failure: the host app supplied an invalid wallet configuration (e.g. a rejected data directory). A developer error, not a device/user condition — NON-retryable, and deliberately without an action button (a Retry here would loop the same failure; security review N1).
  ///
  /// In en, this message translates to:
  /// **'This app\'s wallet setup is misconfigured, so the wallet can\'t start. Retrying won\'t help — please report this to the app\'s developer. Your funds are not affected.'**
  String get walletOnboardingFailedConfiguration;

  /// Entry button on the active wallet surface that opens the send flow.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get walletSendButton;

  /// Honest reason under a disabled Send when no sync pass will run (#405 → the SSOT). Wins over any retained status arm: nothing is syncing, stalling, or catching up. CAUSE-AGNOSTIC — the badge names the cause. No trailing period (it renders as a reason line).
  ///
  /// In en, this message translates to:
  /// **'Syncing isn\'t running — your spendable balance can\'t update'**
  String get walletSendSyncNotRunning;

  /// Honest reason shown under a disabled Send button while sync is still catching up and nothing is spendable yet (spend-before-sync). Neutral wording (#356-F7): a zero-fund wallet has no funds for sync to 'reach', so the copy must not imply funds are known to exist.
  ///
  /// In en, this message translates to:
  /// **'Still syncing — you can send once you have a spendable balance'**
  String get walletSendWaitingForFunds;

  /// Honest reason shown under a disabled Send button when the wallet is fully synced but has no spendable funds.
  ///
  /// In en, this message translates to:
  /// **'No spendable balance yet'**
  String get walletSendNoSpendableYet;

  /// Honest reason shown under a disabled Send button when sync is stalled or offline (so it won't progress until connectivity/the fault is resolved) and nothing is spendable.
  ///
  /// In en, this message translates to:
  /// **'You can send once syncing resumes'**
  String get walletSendSyncUnavailable;

  /// App-bar title of the send screen.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get walletSendTitle;

  /// Honest state when the send screen has no live wallet session (defensive).
  ///
  /// In en, this message translates to:
  /// **'Your wallet isn\'t ready right now. Go back and try again.'**
  String get walletSendUnavailable;

  /// Honest full-screen state when the send screen is reached on a watch-only wallet (no spending keys). Permanent fact, not transient.
  ///
  /// In en, this message translates to:
  /// **'This is a watch-only wallet. It can show balances and receive payments, but it holds no spending keys — so it can\'t send.'**
  String get walletSendWatchOnly;

  /// Full-screen title when a send screen opened by the app's own entry point (WalletSendEntry.push) appears AFTER the entry's mount grace ran out (stage S8 deadline, R05). The app was already told that nothing was sent, and that answer is final for this request: the screen offers no form and no way to pay it. Terminal, not transient — the user starts again from the app.
  ///
  /// In en, this message translates to:
  /// **'This payment request expired'**
  String get walletSendExpiredTitle;

  /// Body for walletSendExpiredTitle. States the limit the user hit (the five-second mount grace — keep the number in step with WalletSendEntry's grace if it is ever re-priced), why the wallet refuses (the app already holds a final "nothing was sent" for this request, and paying under it would put a payment on chain with no record of it in the app), and the ONE next step. Never promises the wallet will pay it later; nothing was signed.
  ///
  /// In en, this message translates to:
  /// **'The send screen took more than five seconds to open, so the app was told that nothing was sent. That answer is final: this request can\'t be paid from here. To pay, start again from the app.'**
  String get walletSendExpiredBody;

  /// Inline send-form fault when a watch-only wallet somehow reaches propose/send (defense-in-depth; the SDK refuses the spend).
  ///
  /// In en, this message translates to:
  /// **'This is a watch-only wallet — it holds no spending keys, so it can\'t send.'**
  String get walletSendFaultWatchOnly;

  /// Spendable-balance hint above the send form; amount is integer-formatted (never a float).
  ///
  /// In en, this message translates to:
  /// **'Available to send: {amount} ZEC'**
  String walletSendAvailable(String amount);

  /// Variant of walletSendAvailable while the wallet is still catching up (#381, the #380 swap-line rule): the spendable figure is the partial repopulating balance, so a low/zero figure (incl. the post-'Send another' refresh) must not read as final.
  ///
  /// In en, this message translates to:
  /// **'Available to send: {amount} ZEC — your balance is still catching up'**
  String walletSendAvailableCatchingUp(String amount);

  /// Label for the recipient address field / confirm line.
  ///
  /// In en, this message translates to:
  /// **'Recipient address'**
  String get walletSendRecipientLabel;

  /// Placeholder for the recipient address field.
  ///
  /// In en, this message translates to:
  /// **'Zcash address (starts with u, z, or t)'**
  String get walletSendRecipientHint;

  /// Helper text AND screen-reader label for the recipient field when it is opened read-only (locked) by a prefilled request (from a scanned payment code or the app), so the address can't be edited. Sentence case, no period. Generic — the lock applies whether or not the prefill came from a payment URI.
  ///
  /// In en, this message translates to:
  /// **'Recipient can\'t be changed here'**
  String get walletSendRecipientLocked;

  /// Label for the amount field.
  ///
  /// In en, this message translates to:
  /// **'Amount (ZEC)'**
  String get walletSendAmountLabel;

  /// Placeholder for the amount field.
  ///
  /// In en, this message translates to:
  /// **'0.00'**
  String get walletSendAmountHint;

  /// Label for the optional memo field.
  ///
  /// In en, this message translates to:
  /// **'Memo (optional)'**
  String get walletSendMemoLabel;

  /// Helper under the memo field — memos can't go to transparent addresses.
  ///
  /// In en, this message translates to:
  /// **'Only delivered to shielded (private) recipients'**
  String get walletSendMemoHint;

  /// Note shown under the memo field when it is disabled because the recipient is a transparent (public) address that cannot receive a memo.
  ///
  /// In en, this message translates to:
  /// **'Memos need a shielded recipient. This public address can\'t receive one.'**
  String get walletSendMemoTransparentDisabled;

  /// Note under the disabled memo field when the host attached opaque machine-memo bytes (FR-28). One memo per payment, so the written memo field is unavailable.
  ///
  /// In en, this message translates to:
  /// **'This payment already carries a reference from the app, so it can\'t also take a written memo.'**
  String get walletSendMemoMachineDisabled;

  /// Heading of the per-send disclosure shown at the authorization step when the host attached opaque machine-memo bytes. Never shows the bytes themselves.
  ///
  /// In en, this message translates to:
  /// **'The app is attaching a reference'**
  String get walletSendMachineMemoTitle;

  /// The host-supplied purpose sentence, rendered verbatim in the machine-memo disclosure.
  ///
  /// In en, this message translates to:
  /// **'It says this is for: {purpose}'**
  String walletSendMachineMemoPurpose(String purpose);

  /// The honest limit under the machine-memo disclosure: a purpose label is accountability, not verification. The wallet cannot confirm the sentence describes the bytes.
  ///
  /// In en, this message translates to:
  /// **'It stays with the transaction and can\'t be removed later. The wallet can\'t check what it contains.'**
  String get walletSendMachineMemoLimit;

  /// Live recipient-field status: the entered address is a shielded address, so the payment is private.
  ///
  /// In en, this message translates to:
  /// **'Shielded · private'**
  String get walletSendRecipientShielded;

  /// Live recipient-field status: the entered address is a transparent address, so the payment is publicly visible on-chain.
  ///
  /// In en, this message translates to:
  /// **'Public'**
  String get walletSendRecipientTransparent;

  /// Live recipient-field status: the entered text is not a parseable Zcash address.
  ///
  /// In en, this message translates to:
  /// **'This doesn\'t look like a valid Zcash address.'**
  String get walletSendRecipientInvalid;

  /// Live recipient-field status: the address is well-formed but for the wrong network (e.g. a testnet address in a mainnet wallet).
  ///
  /// In en, this message translates to:
  /// **'This address is for a different Zcash network.'**
  String get walletSendRecipientWrongNetwork;

  /// Primary form action: prepare the send and show the confirm screen.
  ///
  /// In en, this message translates to:
  /// **'Review payment'**
  String get walletSendReviewButton;

  /// Offline-first secondary action: durably queue the send for the next online sync.
  ///
  /// In en, this message translates to:
  /// **'Queue to send later'**
  String get walletSendQueueButton;

  /// Honest note under the queue action (#399, retold by #401 R1). NEVER asserts the user is offline (the wallet may be unsynced, or the SERVER may be the unreachable side). It also PROMISES NO SCHEDULE: the previous 'prepared and sent automatically the next time your wallet syncs online' is FALSE at host custody, where the background pass holds no signing credential at all — and that is exactly the host the queue was widened for (FR-23-b). Same rule and same wording family as walletSendQueuedBody, which the user meets ONE TAP LATER: name the surface the payment lives on and the two things they can do there. Keeps the fee-preview honesty (queuing skips it; the fee is computed at signing).
  ///
  /// In en, this message translates to:
  /// **'A queued payment waits under Saved & pending, where you can send it or cancel it. Its network fee is worked out when it\'s sent.'**
  String get walletSendQueueHint;

  /// Busy label while proposing (local note-selection + fee).
  ///
  /// In en, this message translates to:
  /// **'Preparing your payment…'**
  String get walletSendPreparing;

  /// Busy label while signing + broadcasting.
  ///
  /// In en, this message translates to:
  /// **'Sending…'**
  String get walletSendSubmitting;

  /// Busy label while durably persisting the queued send intent.
  ///
  /// In en, this message translates to:
  /// **'Queuing…'**
  String get walletSendQueuing;

  /// Heading on the confirm screen.
  ///
  /// In en, this message translates to:
  /// **'Confirm payment'**
  String get walletSendReviewTitle;

  /// Confirm line: total debited (recipient amount + fee).
  ///
  /// In en, this message translates to:
  /// **'Total'**
  String get walletSendTotalLabel;

  /// Confirm line: the ZIP-317 fee.
  ///
  /// In en, this message translates to:
  /// **'Network fee'**
  String get walletSendFeeLabel;

  /// Confirm line: change returned to the wallet (informational).
  ///
  /// In en, this message translates to:
  /// **'Change returned'**
  String get walletSendChangeLabel;

  /// §5.1 de-shield disclosure heading: a transparent recipient makes the payment public.
  ///
  /// In en, this message translates to:
  /// **'This payment is not private'**
  String get walletSendDeshieldTitle;

  /// §5.1 de-shield disclosure body — honest about the public, linkable de-shield.
  ///
  /// In en, this message translates to:
  /// **'It sends to a public address, so the amount and recipient will be publicly visible on the Zcash blockchain.'**
  String get walletSendDeshieldBody;

  /// Checkbox under the 'not private' warning on a payment to a transparent address, on the review and beside Queue; Send now / Queue stay disabled until it is ticked (maintainer: 'add the Send acknowledgement like Swap', this wording).
  ///
  /// In en, this message translates to:
  /// **'I understand this payment will be public.'**
  String get walletSendPublicAckLabel;

  /// Confirm-screen action that signs + broadcasts.
  ///
  /// In en, this message translates to:
  /// **'Send now'**
  String get walletSendConfirmButton;

  /// Confirm-screen action that returns to the editable form.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get walletSendBackButton;

  /// Passive info note on the confirm screen when the recipient is the wallet's own address — money-safe, just usually unintended. Never a blocker.
  ///
  /// In en, this message translates to:
  /// **'You\'re sending to your own wallet. The network fee still applies.'**
  String get walletSendSelfSendNote;

  /// Title of the deliberate large-amount confirmation dialog shown before signing when the proposal trips the money-safety check.
  ///
  /// In en, this message translates to:
  /// **'Send a large amount?'**
  String get walletSendLargeConfirmTitle;

  /// Large-send dialog body when the amount is a high fraction of the available balance (nearTotalBalance).
  ///
  /// In en, this message translates to:
  /// **'This is almost your entire balance. A sent payment can\'t be reversed.'**
  String get walletSendLargeConfirmNearTotal;

  /// Large-send dialog body when the amount is over the absolute large-amount threshold (overAbsoluteThreshold).
  ///
  /// In en, this message translates to:
  /// **'This is a large payment. A sent payment can\'t be reversed.'**
  String get walletSendLargeConfirmOverThreshold;

  /// Large-send dialog body when both the relative and absolute large-amount triggers fired (both, and the forward-compat unknown arm).
  ///
  /// In en, this message translates to:
  /// **'This is a large payment — almost your entire balance. A sent payment can\'t be reversed.'**
  String get walletSendLargeConfirmBoth;

  /// The irreversible confirm action in the large-send dialog; carries the exact amount so it is unmistakable at the moment of confirming.
  ///
  /// In en, this message translates to:
  /// **'Send {amount}'**
  String walletSendLargeConfirmAction(String amount);

  /// The cancel action in the large-send dialog — returns to the confirm screen without sending.
  ///
  /// In en, this message translates to:
  /// **'Go back'**
  String get walletSendLargeConfirmCancel;

  /// Result: every transaction broadcast successfully.
  ///
  /// In en, this message translates to:
  /// **'Payment sent'**
  String get walletSendSentTitle;

  /// Result body for a fully-broadcast send.
  ///
  /// In en, this message translates to:
  /// **'Your payment has been broadcast to the network.'**
  String get walletSendSentBody;

  /// Result: nothing was accepted this attempt (transport miss or mempool reject); the payment is persisted and will retry.
  ///
  /// In en, this message translates to:
  /// **'Saved — we\'ll finish sending'**
  String get walletSendSavedTitle;

  /// Result body when no transaction was accepted this attempt — cause-agnostic: a transport miss OR a mempool reject (which can be the already-known race shape with money actually in motion; the wallet-screen in-flight cue owns that window). Money-safe; auto-retry. The automatic promise is TRUE at every custody tier (#401 R1b): this transaction is already SIGNED, and the §6.1 ReBroadcast arm re-sends the raw bytes with no seed — unlike a QUEUED intent, which needs a credential the background pass may not have. It is PASS-dependent though, so the render site appends walletSyncPausedMoneyNote when no pass will run. 'On a later sync' — never 'as soon as you're online' (the #399 reconnect-promptness rule).
  ///
  /// In en, this message translates to:
  /// **'Your payment couldn\'t go out just now, so it\'s saved and your wallet will send it on a later sync. Nothing is lost.'**
  String get walletSendSavedBody;

  /// Result: a transaction was signed and kept, but the wallet did NOT report that it will retry it on its own (stage S8 obligation, row 10 — the core's per-transaction delivery state was not retry-pending, or could not be read). No promise of automatic sending; Activity shows the live state. Shared by the send, shield and move result surfaces.
  ///
  /// In en, this message translates to:
  /// **'Saved'**
  String get walletSendKeptTitle;

  /// Result body for walletSendKeptTitle. Deliberately makes NO claim either way about automatic sending: it is shown both when the wallet reported a held state (a swap deposit past its quote) and when the reading could not be taken, and must be true in both. Points at Activity, where TxSummary.delivery is rendered.
  ///
  /// In en, this message translates to:
  /// **'Your wallet has kept this transaction and hasn\'t promised to send it by itself. Check Activity to see where it stands.'**
  String get walletSendKeptBody;

  /// Result body when some (not all) pool-crossing transactions broadcast. Same #401 R1b posture as walletSendSavedBody: already signed, so the completion promise holds at every custody tier, and the render site appends walletSyncPausedMoneyNote when no sync pass will run.
  ///
  /// In en, this message translates to:
  /// **'Part of your payment went out; your wallet will complete the rest on a later sync. Nothing is lost.'**
  String get walletSendPartialBody;

  /// Result title for a TEX two-step send with some but not all legs accepted (either order) — funds are in motion on a wallet-controlled one-time address.
  ///
  /// In en, this message translates to:
  /// **'Payment in progress'**
  String get walletSendInMotionTitle;

  /// Honest in-motion body for a partial TEX two-step send. Must NOT promise auto-completion (the forwarding step can expire into a recoverable strand); only the permanently-true 'don't re-send' plus pointing at the shipped wallet-screen recovery.
  ///
  /// In en, this message translates to:
  /// **'Your payment has started and is moving through a one-time address your wallet controls. Don\'t send it again. If it doesn\'t finish, you can recover the funds from your wallet screen.'**
  String get walletSendInMotionBody;

  /// Result: the one-shot proposal was already consumed (double-tap).
  ///
  /// In en, this message translates to:
  /// **'Already submitted'**
  String get walletSendAlreadyTitle;

  /// Result body for a re-consumed proposal token (no double-spend).
  ///
  /// In en, this message translates to:
  /// **'This payment was already submitted — it won\'t be sent twice.'**
  String get walletSendAlreadyBody;

  /// Result: signing/build failed; no money moved.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t complete payment'**
  String get walletSendFailedTitle;

  /// Result body for a sign/build failure (re-propose needed).
  ///
  /// In en, this message translates to:
  /// **'Something went wrong completing this payment and nothing was sent. You can try again.'**
  String get walletSendFailedBody;

  /// Action on a failed-send result that returns to a fresh form.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletSendTryAgain;

  /// Leave the send screen after a completed/queued send.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get walletSendDone;

  /// Return to a fresh form after a completed send.
  ///
  /// In en, this message translates to:
  /// **'Send another'**
  String get walletSendAnother;

  /// Result: the offline send intent was durably stored.
  ///
  /// In en, this message translates to:
  /// **'Queued to send'**
  String get walletSendQueuedTitle;

  /// Result body for a queued (offline-first) send. It must be true at EVERY custody tier (#400 R9): the previous 'we'll send this automatically the next time your wallet syncs online' is FALSE wherever the wallet holds no signing credential of its own — a host that authorizes each spend individually cannot sign on an unattended background pass at all, and that is exactly the host the offline queue was widened for (FR-23-b). So promise no schedule; name the surface the payment now lives on (keep the wording in step with walletParkedTitle) and the two things the user can actually do there. Never 'as soon as you're online' (the retry schedule can lag a reconnect), and never anything that invites re-entering the payment (a double pay).
  ///
  /// In en, this message translates to:
  /// **'This payment is saved. You\'ll find it under Saved & pending, where you can send it now or cancel it.'**
  String get walletSendQueuedBody;

  /// Form fault: insufficient funds. Figures are integer-formatted; never logged (§5.4).
  ///
  /// In en, this message translates to:
  /// **'Not enough spendable balance — you have {available} ZEC and this needs {required} ZEC.'**
  String walletSendFaultInsufficient(String available, String required);

  /// Send fault body under the shared walletSendFailedTitle, for RW-SYNC-002 (networkUpgradeUnsupported): the network is running consensus rules this app version cannot build a valid transaction for, so signing is refused before any proving time is spent. MUST NOT blame the user, MUST NOT imply funds are at risk (they are untouched), MUST NOT promise a timeline we do not control, and MUST NOT promise that receiving still works (INC-016: a build that predates the upgrade can be blind to incoming payments as well). Never shown for a wallet that is merely behind on sync — that is walletSendFaultInsufficientCatchingUp.
  ///
  /// In en, this message translates to:
  /// **'The Zcash network was upgraded and this app needs an update before it can send. Your funds are safe.'**
  String get walletSendFaultNetworkUpgrade;

  /// Sync-status headline for SyncStatus.upToDateLimited: the wallet scanned to the chain tip but this app version could not fully interpret every block, because the network runs consensus rules it does not implement. MUST NOT read as a plain 'Up to date' — the balance shown alongside is a floor, not a total.
  ///
  /// In en, this message translates to:
  /// **'Caught up as far as this version can read'**
  String get walletSyncUpToDateLimited;

  /// Sync-detail sheet explanation paired with walletSyncUpToDateLimited. Names both consequences honestly — possibly-invisible funds and unavailable memos — because either alone would understate it.
  ///
  /// In en, this message translates to:
  /// **'The Zcash network was upgraded. This version has scanned everything it can read, but newer blocks may hold funds it cannot show yet, and memos on recent payments are unavailable. Update the app to see everything.'**
  String get walletSyncExplainUpToDateLimited;

  /// Sync-status headline for SyncStatus.upToDateDegraded (T0-1b): the wallet scanned to the chain tip, but THIS SERVER refused, withheld or misreported the subtree roots of one of Zcash's shielded pools, so funds received in that pool cannot be spent through it. MUST NOT read as a plain 'Up to date' — the balance shown alongside is a floor for that pool. Distinct from walletSyncUpToDateLimited (that one is about this app VERSION and says update; this one is about the SERVER and says switch).
  ///
  /// In en, this message translates to:
  /// **'Caught up, but this server isn\'t serving every pool'**
  String get walletSyncUpToDateDegraded;

  /// Sync-detail sheet explanation paired with walletSyncUpToDateDegraded. The next step MUST be 'switch servers': never 'check your connection' (the link works — the pass reached the tip) and never 'update the app' (that is the UpToDateLimited pair). Names the money consequence honestly (unspendable funds in that pool, balance is a floor) without claiming the user holds any. WHICH pool, and how, is the detail line under it — walletSyncPoolUnsupported / walletSyncPoolWithheld / walletSyncPoolHeightViolation / walletSyncPoolUnknown, one per affected pool (§4r U-3).
  ///
  /// In en, this message translates to:
  /// **'This server is refusing, withholding or misreporting one of Zcash\'s shielded pools. Funds received in that pool can\'t be spent through it, and the balance shown is a floor. Switch to another server to use them — this isn\'t a connection problem.'**
  String get walletSyncExplainUpToDateDegraded;

  /// Sync-detail line (and badge a11y label) for ONE shielded pool whose service on the last pass was PoolService.unsupported — this server does not know the pool at all (an older lightwalletd), so funds received in it cannot be spent through this server (§4r U-3, closing §4j row 8 by rendering). One line per affected pool, under the explanation of walletSyncUpToDateDegraded, walletSyncEndpointBehind or walletSyncUnverified; NO line for a pool served normally. pool is the pool's name (walletPoolSapling / walletPoolOrchard / walletPoolIronwood). 'Refuses', matching walletSyncExplainUpToDateDegraded's 'refusing'. MUST NOT say the pool is empty and MUST NOT say funds are lost; the next step (switch servers) is in the explanation above it.
  ///
  /// In en, this message translates to:
  /// **'{pool}: this server refuses to serve it'**
  String walletSyncPoolUnsupported(String pool);

  /// Sync-detail line for ONE shielded pool whose service was PoolService.withheld — this server served FEWER completed subtree roots than the wallet can prove the pool already has (from the signed data the app ships with), so funds received in the part it did not serve cannot be spent through this server (§4r U-3). Same placement and rules as walletSyncPoolUnsupported. 'Withholding', matching the explanation's 'withholding'. MUST NOT say the pool is empty.
  ///
  /// In en, this message translates to:
  /// **'{pool}: this server is withholding part of it'**
  String walletSyncPoolWithheld(String pool);

  /// Sync-detail line for ONE shielded pool whose service was PoolService.heightViolation — this server served subtree completion heights that cannot be true and the wallet refused to record them; the server ANSWERED and the answer was wrong (§4r U-3). Same placement and rules as walletSyncPoolUnsupported. 'Misreporting', matching the explanation's 'misreporting'. MUST NOT say 'empty' or 'unknown pool'.
  ///
  /// In en, this message translates to:
  /// **'{pool}: this server is misreporting it'**
  String walletSyncPoolHeightViolation(String pool);

  /// Sync-detail line for ONE shielded pool whose service is PoolService.unknown — the bridge's forward-compatibility arm (a state this version of the UI does not know; only under core/bridge version skew). Rendered as unknown, NEVER as healthy (spec §3.3 unknown handling) — the pool still counts as degraded. Same placement as walletSyncPoolUnsupported.
  ///
  /// In en, this message translates to:
  /// **'{pool}: this server\'s service for it is unknown'**
  String walletSyncPoolUnknown(String pool);

  /// The Sapling shielded pool's name, as the {pool} placeholder of the walletSyncPool* lines. A proper noun: keep it in Latin script unless the locale's Zcash community writes it otherwise.
  ///
  /// In en, this message translates to:
  /// **'Sapling'**
  String get walletPoolSapling;

  /// The Orchard shielded pool's name, as the {pool} placeholder of the walletSyncPool* lines. A proper noun: keep it in Latin script unless the locale's Zcash community writes it otherwise.
  ///
  /// In en, this message translates to:
  /// **'Orchard'**
  String get walletPoolOrchard;

  /// The Ironwood shielded pool's name (NU6.3), as the {pool} placeholder of the walletSyncPool* lines. A proper noun: keep it in Latin script unless the locale's Zcash community writes it otherwise.
  ///
  /// In en, this message translates to:
  /// **'Ironwood'**
  String get walletPoolIronwood;

  /// Sync-status headline for SyncStatus.endpointBehind (T0-1c): the wallet scanned to THIS SERVER's reported tip, but that tip is below a block height the network had already passed before this version of the app was built — the server is behind the chain (a node still syncing, stuck or forked, or a server under-reporting its height). MUST NOT read as a plain 'Up to date': the balance shown alongside is current only as of that older block. Distinct from walletSyncUpToDateLimited (this app VERSION — says update) and walletSyncUpToDateDegraded (this server's POOLS — says switch); this one is about this server's HEIGHT and also says switch.
  ///
  /// In en, this message translates to:
  /// **'Caught up with this server, but it\'s behind the network'**
  String get walletSyncEndpointBehind;

  /// Sync-detail sheet explanation paired with walletSyncEndpointBehind. The next step MUST be 'switch servers': never 'check your connection' (the link works — the pass completed) and never 'update the app' (that is the UpToDateLimited pair). Names both money consequences honestly and without claiming the user holds any: incoming payments after that block are not visible from this server, and a send built against this server's tip carries an expiry the real chain may already be past (it would expire and the funds return, not be lost). MUST NOT say the funds are lost or that anything needs restoring.
  ///
  /// In en, this message translates to:
  /// **'This server\'s copy of the chain stops at a block the network passed before this version of the app was built, so your balance is only current as of that block. New payments to you may not show yet, and a payment sent from here may not go through. Switch to another server to catch up — this isn\'t a connection problem.'**
  String get walletSyncExplainEndpointBehind;

  /// Parked-send row detail when ParkedSend.signingBlock is SigningBlock.networkUpgrade — and ONLY then (GRACE-1 §4p G-6): the drain will not sign this row until the app is updated. MUST NOT say 'will send when ready' (it will not, on this version) and MUST NOT say 'failed' (the money is untouched and the intent is intact). Cancel stays offered; retry does not, since retrying changes nothing until the app is updated. A server that merely stopped reporting its network is walletParkedBlockedByServerSilent, never this.
  ///
  /// In en, this message translates to:
  /// **'Waiting for an app update — your funds are safe and nothing has been sent.'**
  String get walletParkedBlockedByNetworkUpgrade;

  /// Parked-send row detail when ParkedSend.signingBlock is SigningBlock.graceExpired (GRACE-1 §4p): this server will not say which network it is on and the wallet's grace for it has run out (or never began), so the drain will not sign this row until a server that reports its network is used. The next step is SWITCH SERVERS. MUST NOT say 'upgraded' or 'update the app' (nothing was upgraded and an update fixes nothing — that is walletParkedBlockedByNetworkUpgrade), MUST NOT say 'will send when ready' and MUST NOT say 'failed'. Cancel stays offered; retry does not.
  ///
  /// In en, this message translates to:
  /// **'Waiting for a server that reports the network version — switch servers. Your funds are safe and nothing has been sent.'**
  String get walletParkedBlockedByServerSilent;

  /// walletParkedBlockedByServerSilent's variant when the grace ran out on the DEVICE CLOCK (GraceExpiry.clock). The next step is the SAME as the sibling's — a server that reports the network version — and the device clock is a PRECONDITION of it, never an alternative to it (§4p-run fold review row 6, §4r U-5): a corrected clock alone re-permits nothing (the latch holds until a branch-reporting server), so the copy MUST read 'if the date and time are wrong, fix them FIRST — then switch' and MUST NOT read 'switch servers, OR check the date and time'. Same prohibitions as the sibling: never 'upgraded', 'update the app', 'will send when ready' or 'failed'.
  ///
  /// In en, this message translates to:
  /// **'Waiting for a server that reports the network version. If this device\'s date and time are wrong, fix them first — then switch servers. Your funds are safe and nothing has been sent.'**
  String get walletParkedBlockedByServerSilentClock;

  /// Sync-status headline for SyncStatus.upToDateUnverified (GRACE-1 §4p): the wallet scanned to the chain tip, but THIS SERVER will not say which version of the Zcash network it is on, so the app cannot confirm a payment it signs will be accepted; sending works for a short grace and is then refused. MUST NOT read as a plain 'Up to date'. Distinct from walletSyncUpToDateLimited (this app VERSION — says update; MUST NOT be conflated: nothing was upgraded here), walletSyncUpToDateDegraded (this server's POOLS) and walletSyncEndpointBehind (this server's HEIGHT); this one is about the server's NETWORK CLAIM and, like the last two, says switch. MUST NOT contain 'upgraded' or 'update'.
  ///
  /// In en, this message translates to:
  /// **'Caught up, but this server isn\'t reporting the network version'**
  String get walletSyncUnverified;

  /// Detail line under walletSyncUnverified while the grace RUNS and the device clock can be trusted for it: how long sending keeps working. hours is the whole hours left on whichever of the two grace rules (blocks, device clock) runs out FIRST — the SDK already converted the blocks through the network's block spacing, so this is one figure; 0 renders as 'less than an hour'. The next step ('then switch servers') rides in the sentence. MUST NOT say 'upgraded' or 'update'.
  ///
  /// In en, this message translates to:
  /// **'{hours, plural, =0{Sending still works for less than an hour — then switch servers.} =1{Sending still works for about 1 more hour — then switch servers.} other{Sending still works for about {hours} more hours — then switch servers.}}'**
  String walletSyncGraceLeftHours(int hours);

  /// walletSyncGraceLeftHours's variant when the SDK hands NO time — the device clock cannot be trusted for the grace (it reads before the last confirmation), so the block rule alone decides and only the blocks are shown (GRACE-1 §4p G-4; this is the same string set, not a new case). blocks is pre-formatted compactly (e.g. "1.2K"). MUST NOT say 'upgraded' or 'update'.
  ///
  /// In en, this message translates to:
  /// **'Sending still works for about {blocks} more blocks — then switch servers.'**
  String walletSyncGraceLeftBlocks(String blocks);

  /// The grace ENDED by the block rule (GraceExpiry.blocks; GRACE-1 §4p G-6): the chain advanced a day's worth of blocks since the app last confirmed, with a server that reports its network, that it can send — and this server never said. ONE sentence shared by the sync-status detail line, the send-fault body (RW-SYNC-003) and nothing else, so the three never disagree. blocks is the count since that confirmation, pre-formatted compactly. The next step is SWITCH SERVERS. MUST NOT say 'upgraded' or 'update the app' (that is walletSendFaultNetworkUpgrade — a different fault with a different fix) and MUST NOT imply funds are at risk.
  ///
  /// In en, this message translates to:
  /// **'This server hasn\'t reported the network version for {blocks} blocks, so this app can\'t confirm it\'s safe to send. Switch to another server.'**
  String walletSyncGraceEndedBlocks(String blocks);

  /// The grace ENDED by the DEVICE CLOCK rule (GraceExpiry.clock; GRACE-1 §4p G-6): a day passed on this device's clock since the last confirmation, whatever the server's block height did — the axis a server that freezes its height cannot hold still — or the clock was set back after that day was seen (which does not re-open the grace). Shared by the sync detail line and the send-fault body. ONE next step — a server that reports the network version — with the device clock as its PRECONDITION, never an alternative (§4p-run fold review row 6, §4r U-5): a wrong clock is the one benign cause, but a corrected clock alone re-permits nothing (the latch holds until a branch-reporting server), so the copy MUST read 'if the date and time are wrong, fix them FIRST — then switch' and MUST NOT read 'switch, OR check the date and time'. MUST NOT say 'upgraded' or 'update the app'.
  ///
  /// In en, this message translates to:
  /// **'This server hasn\'t reported the network version for a day, so this app can\'t confirm it\'s safe to send. If this device\'s date and time are wrong, fix them first — then switch to a server that reports the network version.'**
  String get walletSyncGraceEndedClock;

  /// The grace never BEGAN (GraceExpiry.neverConfirmed; GRACE-1 §4p): every server this wallet has met withheld the network version, so the app has never confirmed it can send at all. Shared by the sync detail line and the send-fault body; also the forward-compat fallback for a grace shape this UI does not know. NOT the never-synced case (that is walletSendFaultNotSynced — wait for sync). Next step: SWITCH SERVERS. MUST NOT say 'upgraded' or 'update the app'.
  ///
  /// In en, this message translates to:
  /// **'This server has never reported the network version, so this app can\'t confirm it\'s safe to send. Switch to another server.'**
  String get walletSyncGraceNeverConfirmed;

  /// Sync-detail sheet explanation paired with walletSyncUnverified; the grace's own line (walletSyncGraceLeft*/walletSyncGraceEnded*) renders beside it and OWNS the sending claim (running: a countdown; ended: a refusal) — this body makes NO claim about sending (fold of the security and crypto angles: it used to say 'sending keeps working for a short grace period', false once the grace has ended). The next step MUST be 'switch servers': never 'check your connection' (the link works — the pass reached the tip) and never 'update the app' (that is the UpToDateLimited pair; nothing was upgraded here). States honestly that the balance IS current (this server serves blocks; only its network claim is missing) — unlike the Limited and Degraded pairs, this is not a balance-is-a-floor state. Under a reported rewinding streak the sheet shows walletSyncExplainUnverifiedStreak instead. MUST NOT say 'upgraded' or 'update'.
  ///
  /// In en, this message translates to:
  /// **'This server isn\'t saying which version of the Zcash network it\'s on, so this app can\'t confirm that a payment it signs will be accepted. Your balance is current. Switch to another server — this isn\'t a connection problem.'**
  String get walletSyncExplainUnverified;

  /// Sync-detail sheet explanation paired with walletSyncUnverified when the SDK reports streakReported: true on the grace claim (P3-12, maintainer): the loop has judged this server misbehaving (repeated rewinds — the endpointMisbehaving stall the grace outranks, P2-6). MUST NOT say the balance is current; names the rewinds and their consequence for the balance; makes NO claim about sending (the grace line beside it owns that — fold). The next step MUST be 'switch servers': never 'check your connection', never 'update the app'. MUST NOT say 'upgraded' or 'update'.
  ///
  /// In en, this message translates to:
  /// **'This server isn\'t saying which version of the Zcash network it\'s on, so this app can\'t confirm that a payment it signs will be accepted. It has also kept serving blocks this wallet then had to undo, so your balance may not be current. Switch to another server — this isn\'t a connection problem.'**
  String get walletSyncExplainUnverifiedStreak;

  /// Sync-detail line rendered directly under the grace line when the SDK reports streakReported: true on the grace claim (P3-12 fold, security review MEDIUM 1): the badge's screen-reader label is the headline plus the detail lines, so the streak the grace outranks reaches a user who never opens the sheet. Same next step, 'switch servers'; MUST NOT say 'update' or 'upgraded'.
  ///
  /// In en, this message translates to:
  /// **'This server also keeps serving blocks this wallet then has to undo — switch servers.'**
  String get walletSyncUnverifiedStreakDetail;

  /// Form fault detail under the insufficient-funds message while the wallet is still catching up (#381): the 'you have X' figure is the partial repopulating balance, not a final verdict. Hedged ('may') — it must not promise funds exist.
  ///
  /// In en, this message translates to:
  /// **'Your balance is still catching up — more may become available as the wallet syncs.'**
  String get walletSendFaultInsufficientCatchingUp;

  /// Form fault detail: pending-incoming funds shown alongside an insufficient-funds error. 'Once the wallet catches up', NOT 'once it confirms': the amount is most often a note with thousands of confirmations that is held only until more of the chain is scanned (witness unavailable), so 'confirms' was false for the common case (phase-2 P2-4, maintainer decision 3).
  ///
  /// In en, this message translates to:
  /// **'{pending} ZEC is still arriving and will be spendable once the wallet catches up.'**
  String walletSendFaultInsufficientPending(String pending);

  /// Form fault: the amount field is empty.
  ///
  /// In en, this message translates to:
  /// **'Enter an amount to send.'**
  String get walletSendFaultAmountEmpty;

  /// Form fault: the amount isn't a plain decimal number.
  ///
  /// In en, this message translates to:
  /// **'Enter the amount as a number, for example 0.25.'**
  String get walletSendFaultAmountNotANumber;

  /// Form fault: more than 8 fractional digits.
  ///
  /// In en, this message translates to:
  /// **'ZEC has at most 8 decimal places.'**
  String get walletSendFaultAmountDecimals;

  /// Form fault: the amount parses to zero.
  ///
  /// In en, this message translates to:
  /// **'Enter an amount greater than zero.'**
  String get walletSendFaultAmountNotPositive;

  /// Form fault: the amount exceeds max money.
  ///
  /// In en, this message translates to:
  /// **'That amount is larger than the total ZEC supply.'**
  String get walletSendFaultAmountOutOfRange;

  /// Form fault: the amount exceeds the HOST's policy send ceiling (walletSendCeilingZatProvider, e.g. an alpha roll-out cap). Honest app-policy phrasing — the amount itself is valid.
  ///
  /// In en, this message translates to:
  /// **'This app currently limits sends to {limit} ZEC.'**
  String walletSendFaultOverCeiling(String limit);

  /// Form fault: the recipient address failed to parse.
  ///
  /// In en, this message translates to:
  /// **'That doesn\'t look like a valid Zcash address for this network. Check it and try again.'**
  String get walletSendFaultAddressInvalid;

  /// Form fault: a memo was given to a transparent recipient.
  ///
  /// In en, this message translates to:
  /// **'This recipient can\'t receive a memo. Remove the memo, or send to a shielded (private) address.'**
  String get walletSendFaultMemoToTransparent;

  /// Form fault: the memo exceeds its length bound.
  ///
  /// In en, this message translates to:
  /// **'Your memo is too long. Shorten it and try again.'**
  String get walletSendFaultMemoTooLong;

  /// Form fault: a reserved/invalid memo.
  ///
  /// In en, this message translates to:
  /// **'That memo can\'t be sent. Remove it and try again.'**
  String get walletSendFaultMemoNotSendable;

  /// Form fault: the app supplied BOTH a text memo and machine bytes on one payment. A programming error, not a user mistake — the copy must not tell the user to fix or remove their memo, and must say plainly that no money moved.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t send this payment — the app attached two notes to it. Nothing was sent.'**
  String get walletSendFaultMemoConflict;

  /// Form fault: the address belongs to the other Zcash network.
  ///
  /// In en, this message translates to:
  /// **'That address is for a different network.'**
  String get walletSendFaultNetworkMismatch;

  /// Form fault: the composed payment URI was rejected.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t build this payment. Check the address and amount.'**
  String get walletSendFaultUriInvalid;

  /// Form fault: not anchorable yet (proposal stale) — offers the queue path. Shown ONLY where the queue affordance actually renders; otherwise walletSendFaultNotSyncedNoQueue.
  ///
  /// In en, this message translates to:
  /// **'Your wallet isn\'t synced far enough yet. Wait for sync to catch up, or queue this to send later.'**
  String get walletSendFaultNotSynced;

  /// The same not-anchorable fault WITHOUT the queue invitation — for surfaces with no offline-queue affordance (the move-to-transparent sheet always; the send form when the host's custody disables the queue, #327). Must stay walletSendFaultNotSynced with ONLY its trailing ', or queue this to send later' clause removed (grammar-mandated closes allowed, e.g. ja 待つ→待ってください) so the two never drift.
  ///
  /// In en, this message translates to:
  /// **'Your wallet isn\'t synced far enough yet. Wait for sync to catch up.'**
  String get walletSendFaultNotSyncedNoQueue;

  /// The not-anchorable send fault when no sync pass will run (#405 → the SSOT, so a FAILED start no longer falls through to 'wait for sync to catch up'). The queue clause is gated off by the host policy separately. CAUSE-AGNOSTIC and points at the wallet screen's sync status rather than naming one cause's remedy — the send screen has no ambient badge of its own.
  ///
  /// In en, this message translates to:
  /// **'Your wallet isn\'t synced far enough yet, and syncing isn\'t running right now. Check the sync status on the wallet screen.'**
  String get walletSendFaultNotSyncedSyncNotRunning;

  /// Form fault on the SEND path: the reviewed proposal's anchor went stale between confirm and send (the wallet IS synced; the numbers aged out) — re-propose for fresh figures. Distinct from walletSendFaultNotSynced (the propose-path not-anchorable case).
  ///
  /// In en, this message translates to:
  /// **'The amounts expired while you were reviewing. Please review the payment again.'**
  String get walletSendFaultAmountsExpired;

  /// Form fault: the durable offline-send queue is at capacity.
  ///
  /// In en, this message translates to:
  /// **'Too many sends are waiting to go out. Let them send first, then try again.'**
  String get walletSendFaultQueueFull;

  /// Form fault: the wallet is mid-lifecycle (busy/closing).
  ///
  /// In en, this message translates to:
  /// **'The wallet is busy right now. Try again in a moment.'**
  String get walletSendFaultWalletBusy;

  /// Send fault: the device is out of disk space, so persisting the send hit DiskFull (#373). Retrying without freeing space fails again, so the copy asks for space instead of a plain retry. Funds are untouched — nothing was written or broadcast. Sibling of walletRescanNeedsSpaceNotice and walletOnboardingFailedStorageFull.
  ///
  /// In en, this message translates to:
  /// **'There isn\'t enough free space to complete this send. Free up some space and try again.'**
  String get walletSendFaultStorageFull;

  /// Send fault: the one-time (ephemeral) address gap-limit ceiling for a multi-step (TEX) send. DUAL-NATURED (#315): slots held by confirming transfers free up on their own; slots used up by sends that never confirmed do NOT — so the copy must promise neither 'just wait' nor doom. Routed back to the form (orange-transient), never the red dead-end.
  ///
  /// In en, this message translates to:
  /// **'Too many one-time addresses are in use right now. Some may free up as transfers confirm, but this may not clear on its own. Your funds are safe.'**
  String get walletSendFaultOneTimeAddressLimit;

  /// Form fault: a prepare failure with no finer mapping that is DETERMINISTIC on the input (retrying unchanged re-fails), so the honest next step is to check the details. The retryable class has its own key, walletSendFaultCouldNotPrepareTransient — never render this one for it (INC-018 (b)).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t prepare this payment. Check the details and try again.'**
  String get walletSendFaultCouldNotPrepare;

  /// Form fault: a prepare failure the wallet's OWN state will clear without the user changing anything — a note whose witness the scan has not completed, an anchor not yet recorded, an input a concurrent proposal holds (WalletErrorKind.proposeTransient, INC-018 (b), phase-2 P2-2, maintainer decision 4). MUST NOT say 'check the details': on the device proof the details were correct and the identical send prepared fine two minutes later. 'Just now' + 'in a moment' — a short wait, not a sync-length one (that is walletSendFaultNotSynced).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t prepare this payment just now. Try again in a moment.'**
  String get walletSendFaultCouldNotPrepareTransient;

  /// Entry button on the active wallet surface that opens the swap flow (shown only when the host has enabled swap).
  ///
  /// In en, this message translates to:
  /// **'Swap'**
  String get walletSwapButton;

  /// App-bar title of the swap screen.
  ///
  /// In en, this message translates to:
  /// **'Swap ZEC'**
  String get walletSwapTitle;

  /// Honest state when the swap screen has no live wallet session (defensive).
  ///
  /// In en, this message translates to:
  /// **'Your wallet isn\'t ready right now. Go back and try again.'**
  String get walletSwapUnavailableWallet;

  /// Honest state when swap is turned off at this build/instance (§3.5 host kill state).
  ///
  /// In en, this message translates to:
  /// **'Swap isn\'t available right now.'**
  String get walletSwapUnavailableOff;

  /// Swap screen reached (host deep-link) on a watch-only wallet (#397 §3.7 D3): the condition is PERMANENT for this wallet, so no 'right now' transience — view-only framing, no retry invitation.
  ///
  /// In en, this message translates to:
  /// **'This wallet is view-only — it can\'t swap.'**
  String get walletSwapUnavailableWatchOnly;

  /// Action that leaves the swap screen and returns to the wallet.
  ///
  /// In en, this message translates to:
  /// **'Done'**
  String get walletSwapDone;

  /// Primary button on NON-terminal tracking cards (pending/detected/processing/unknown/not-found), the establish-failure card, and the swap-unavailable screen (#364 F12). 'Done' there read as 'the swap is done' — this label states the navigation honestly. Terminal outcome cards (success/refunded/failed) keep 'Done'.
  ///
  /// In en, this message translates to:
  /// **'Back to wallet'**
  String get walletSwapBackToWallet;

  /// Spendable-balance hint above the swap form; amount is integer-formatted.
  ///
  /// In en, this message translates to:
  /// **'Available to swap: {amount} ZEC'**
  String walletSwapAvailable(String amount);

  /// Variant of walletSwapAvailable while the wallet is still catching up (#380): the spendable figure is the partial repopulating balance, so a low/zero figure must not read as final.
  ///
  /// In en, this message translates to:
  /// **'Available to swap: {amount} ZEC — your balance is still catching up'**
  String walletSwapAvailableCatchingUp(String amount);

  /// Label for the destination-asset dropdown (what the user swaps their ZEC into).
  ///
  /// In en, this message translates to:
  /// **'Receive asset'**
  String get walletSwapAssetLabel;

  /// Label for the exact ZEC-in amount field.
  ///
  /// In en, this message translates to:
  /// **'Amount to swap (ZEC)'**
  String get walletSwapAmountLabel;

  /// Placeholder for the swap amount field.
  ///
  /// In en, this message translates to:
  /// **'0.00'**
  String get walletSwapAmountHint;

  /// Label for the foreign receive-address field (where the swapped asset is delivered).
  ///
  /// In en, this message translates to:
  /// **'Destination address'**
  String get walletSwapDestinationLabel;

  /// Placeholder for the destination address field.
  ///
  /// In en, this message translates to:
  /// **'Your receiving address on the destination chain'**
  String get walletSwapDestinationHint;

  /// OutOfZec form: the destination field label once a target asset is picked, naming its chain.
  ///
  /// In en, this message translates to:
  /// **'Your {chain} receiving address'**
  String walletSwapDestinationLabelChain(String chain);

  /// OutOfZec form: chain-aware helper for the destination field once a target asset is picked (cross-chain mistakes lose funds).
  ///
  /// In en, this message translates to:
  /// **'A {chain} address — where your swapped asset is sent. Double-check the chain is right.'**
  String walletSwapDestinationHelperChain(String chain);

  /// OutOfZec form: tooltip on the destination-address QR scan button (mobile only).
  ///
  /// In en, this message translates to:
  /// **'Scan a destination-address QR code'**
  String get walletSwapDestinationScanTooltip;

  /// OutOfZec form: the target-asset picker placeholder before an asset is chosen.
  ///
  /// In en, this message translates to:
  /// **'Select an asset to receive'**
  String get walletSwapTargetAssetHint;

  /// Primary form action: request a bounds-checked quote.
  ///
  /// In en, this message translates to:
  /// **'Get quote'**
  String get walletSwapQuoteButton;

  /// Busy label while requesting a quote.
  ///
  /// In en, this message translates to:
  /// **'Getting a quote…'**
  String get walletSwapQuoting;

  /// Busy label while registering the swap and queuing the deposit.
  ///
  /// In en, this message translates to:
  /// **'Starting your swap…'**
  String get walletSwapExecuting;

  /// Snackbar when the user tries to leave (back gesture/button) while the swap execute is still running — the screen blocks leaving for this bounded step (#367 execute pop-guard). Money is being committed; every outcome screen is leavable.
  ///
  /// In en, this message translates to:
  /// **'Still working — the swap is starting. This can take up to a minute.'**
  String get walletSwapExecuteStillWorking;

  /// Heading on the swap review/confirm screen.
  ///
  /// In en, this message translates to:
  /// **'Confirm swap'**
  String get walletSwapReviewTitle;

  /// Review line: the exact ZEC amount leaving the wallet.
  ///
  /// In en, this message translates to:
  /// **'You send'**
  String get walletSwapYouSendLabel;

  /// Review line: the minimum guaranteed amount of the destination asset.
  ///
  /// In en, this message translates to:
  /// **'You receive at least'**
  String get walletSwapYouReceiveLabel;

  /// Formatted receive figure: the provider's decimal min-out amount and the asset label.
  ///
  /// In en, this message translates to:
  /// **'{amount} {asset}'**
  String walletSwapReceiveValue(String amount, String asset);

  /// Review line label (OutOfZec only, #367 fee disclosure): the Zcash network fee the deposit transaction will pay ON TOP of the 'You send' amount — without this line the review implied the deposit was the whole debit.
  ///
  /// In en, this message translates to:
  /// **'Network fee'**
  String get walletSwapNetworkFeeLabel;

  /// Review line value for the network fee: the exact ZIP-317 fee is computed only when the deposit transaction is signed at execute (there is no swap fee-preview round-trip), so the review honestly discloses the fee's EXISTENCE and timing — never a fabricated number.
  ///
  /// In en, this message translates to:
  /// **'Added when the deposit is sent'**
  String get walletSwapNetworkFeeValue;

  /// Review screen: the live quote countdown while time remains. Since #367 the SDK's expiresAt is the ACTIONABLE deadline (display and the execute gate share one number), so the sentence may promise confirmability up to it; 'about' hedges only device-clock skew. Do NOT use wording that guarantees the quote past the shown time.
  ///
  /// In en, this message translates to:
  /// **'Quote valid for about {time} — confirm before it expires.'**
  String walletSwapQuoteExpiresIn(String time);

  /// Screen-reader label for the review quote countdown once under 60 seconds (review M4): a dedicated sentence — composing 'less than a minute' into the {time} slot of walletSwapQuoteExpiresIn double-hedged ('about less than a minute') in every locale at the most time-critical spoken moment.
  ///
  /// In en, this message translates to:
  /// **'Quote valid for less than a minute — confirm before it expires.'**
  String get walletSwapQuoteExpiresUnderMinute;

  /// Review screen: shown when the quote countdown reaches zero; Start swap is disabled.
  ///
  /// In en, this message translates to:
  /// **'This quote has expired. Go back and get a new one — its rate is no longer guaranteed, and sending now risks a refund.'**
  String get walletSwapQuoteExpired;

  /// Screen-reader-only countdown magnitude used in the {time} slot of the quote/deposit countdown sentences once under 60 seconds (#364 F9): a per-second live-region announcement was a 1 Hz storm, so the accessible label goes coarse while the visual text keeps ticking.
  ///
  /// In en, this message translates to:
  /// **'less than a minute'**
  String get walletCountdownUnderMinute;

  /// Countdown magnitude at minute granularity, composed into the countdown sentences' {time} slot (review F2: unit forms live in the ARB so each locale renders its own — the first cut hardcoded Latin 'min' into all 16 locales' spoken labels). Floored minutes — never overstates a money window.
  ///
  /// In en, this message translates to:
  /// **'{minutes} min'**
  String walletCountdownMinutes(int minutes);

  /// Countdown magnitude under a minute (visual; the a11y label uses walletCountdownUnderMinute / the dedicated quote sentence). Localize the unit per locale (review F2).
  ///
  /// In en, this message translates to:
  /// **'{seconds} s'**
  String walletCountdownSeconds(int seconds);

  /// Countdown magnitude past an hour (e.g. '1h 05m'; {minutes} arrives zero-padded). Localize the unit forms per locale (review F2).
  ///
  /// In en, this message translates to:
  /// **'{hours}h {minutes}m'**
  String walletCountdownHoursMinutes(int hours, String minutes);

  /// §2.6 disclosure heading: swapping out de-shields ZEC and exposes the provider legs.
  ///
  /// In en, this message translates to:
  /// **'This swap is not private'**
  String get walletSwapDeshieldTitle;

  /// §2.6 disclosure body — honest about the de-shield and the public provider legs.
  ///
  /// In en, this message translates to:
  /// **'Swapping out de-shields your ZEC — the deposit is a public transaction, and the provider\'s side is public on its network.'**
  String get walletSwapDeshieldBody;

  /// Heading above the §2.6 provider-disclosure list.
  ///
  /// In en, this message translates to:
  /// **'What the swap provider will see'**
  String get walletSwapDiscloseTitle;

  /// Disclosure item: provider sees both amounts.
  ///
  /// In en, this message translates to:
  /// **'The amounts on both sides'**
  String get walletSwapDiscloseAmounts;

  /// Disclosure item: provider links the two assets to one intent.
  ///
  /// In en, this message translates to:
  /// **'That this ZEC and the asset you receive are one swap'**
  String get walletSwapDiscloseCrossLink;

  /// Disclosure item: provider sees the destination address.
  ///
  /// In en, this message translates to:
  /// **'Your destination address'**
  String get walletSwapDiscloseDestination;

  /// Disclosure item: provider sees the source address.
  ///
  /// In en, this message translates to:
  /// **'Your source address'**
  String get walletSwapDiscloseSource;

  /// Disclosure item: provider sees the caller IP unless on Tor.
  ///
  /// In en, this message translates to:
  /// **'Your IP address (unless you route through Tor)'**
  String get walletSwapDiscloseIp;

  /// Disclosure item: a forward-compat disclosure line this build can't name.
  ///
  /// In en, this message translates to:
  /// **'Other details of this swap'**
  String get walletSwapDiscloseGeneric;

  /// Disclosure item shown when providerLegsTransparent: the provider's chain legs are public (distinct from our de-shield).
  ///
  /// In en, this message translates to:
  /// **'The provider\'s own transactions are public on its network'**
  String get walletSwapDiscloseProviderLegsPublic;

  /// Blocking acknowledgment checkbox label — gates the Start swap action (§2.6 disclosures-as-blocking-UX).
  ///
  /// In en, this message translates to:
  /// **'I understand the provider will see the information above.'**
  String get walletSwapAckLabel;

  /// Review-screen action that executes the swap (registers intent + queues the deposit).
  ///
  /// In en, this message translates to:
  /// **'Start swap'**
  String get walletSwapConfirmButton;

  /// Review-screen action that returns to the editable form.
  ///
  /// In en, this message translates to:
  /// **'Back'**
  String get walletSwapBackButton;

  /// Tracking: provider is waiting for the deposit.
  ///
  /// In en, this message translates to:
  /// **'Swap started'**
  String get walletSwapStatusPendingTitle;

  /// Tracking: the COLD-attach / first-load busy title, shown while the swap's status is being established and no answer has arrived yet — a fresh re-attach after process death (no carried state), or the moment just after execute before the first poll returns. Neutral BY DESIGN: 'Swap started' (walletSwapStatusPendingTitle) over-claims a state we have not confirmed (the swap may already be further along, or not yet started). The '…' is a real U+2026 ellipsis. (#347, cold-attach label)
  ///
  /// In en, this message translates to:
  /// **'Checking swap status…'**
  String get walletSwapStatusCheckingTitle;

  /// Tracking body for the pending-deposit state when the WALLET sends the deposit (OutOfZec). Honest across every state this screen can cover, tightened by #367 (UX MED-4 + reliability MED-2): 'briefly offline' + 'window is short' replace the old unbounded 'once you're back online' promise (false past ~11 min of the 15-min window), and 'stays yours … up to an hour to show as spendable' replaces 'stay in your wallet' (the locked-notes balance dip after a gate-caught miss). Keep both hedges.
  ///
  /// In en, this message translates to:
  /// **'Your wallet is sending the ZEC deposit to the provider. If you\'re briefly offline it\'s sent automatically when you\'re back — but the sending window is short, and if it closes first the swap simply ends and nothing is exchanged. Your ZEC stays yours, and it can take up to an hour to show as spendable again.'**
  String get walletSwapStatusPendingBodyOutOfZec;

  /// Tracking body for the pending-deposit state when the USER sends the deposit externally (IntoZec — a foreign coin from their own wallet, never ZEC from this one). Used ONLY within the issuing app run, where the deposit screen showed the instructions; a re-attached swap uses walletSwapStatusPendingBodyIntoZecReattached.
  ///
  /// In en, this message translates to:
  /// **'Waiting for your deposit to arrive. If you haven\'t sent the funds from your other wallet yet, send them before the quote expires.'**
  String get walletSwapStatusPendingBodyIntoZec;

  /// Tracking body for a RE-ATTACHED IntoZec pending-deposit swap (#367 — opened from the wallet-screen row after a restart or re-entry). The original body's 'send them before the quote expires' is impossible to follow here: the deposit address/memo are deliberately not stored, and they must NOT be re-shown (a memo-less deposit can lose funds on memo chains). Honest about both arms: already-sent (will be detected) and never-sent (let it expire, start fresh).
  ///
  /// In en, this message translates to:
  /// **'This swap is still waiting for its deposit. The deposit instructions aren\'t available on this device anymore — if you already sent the funds, they\'ll be detected; if you haven\'t, let this swap expire and start a new one.'**
  String get walletSwapStatusPendingBodyIntoZecReattached;

  /// Tracking detail under the pending-deposit body (W-swap-5 #366-e): the quote's deposit window, so a pending swap is never open-ended on screen. {time} is a locale-formatted DATE AND TIME (month, day, and clock time — a swap window can cross a day boundary), so keep the sentence grammatical with a full datetime, NOT a bare clock time (e.g. no preposition that only reads for a time-of-day).
  ///
  /// In en, this message translates to:
  /// **'The deposit window ends {time}.'**
  String walletSwapPendingWindowEndsAt(String time);

  /// Tracking detail when the pending-deposit window already lapsed and the WALLET was the deposit sender (OutOfZec) — the honest dead-quote line replacing an eternal 'swap started'. Hedged: a late-sent deposit is refunded provider-side, so the claim is only about the not-sent case.
  ///
  /// In en, this message translates to:
  /// **'The deposit window has passed. If the deposit wasn\'t sent in time, the swap ends and your ZEC stays in your wallet.'**
  String get walletSwapPendingWindowPassedOutOfZec;

  /// Tracking detail when the pending-deposit window already lapsed and the USER was the deposit sender (IntoZec). No refund promise — a deposit that arrived late is the provider's refund flow, handled by other states.
  ///
  /// In en, this message translates to:
  /// **'The deposit window has passed. If you haven\'t sent your deposit, this swap simply ends — get a fresh quote when you\'re ready.'**
  String get walletSwapPendingWindowPassedIntoZec;

  /// Wallet-screen section header for the durable in-flight swap list (W-swap-5 #366).
  ///
  /// In en, this message translates to:
  /// **'{count, plural, =1{Swap in progress} other{Swaps in progress}}'**
  String walletSwapsInFlightTitle(int count);

  /// In-flight swap row line when the WALLET sends the deposit (OutOfZec).
  ///
  /// In en, this message translates to:
  /// **'Your ZEC is on its way to the swap provider.'**
  String get walletSwapInFlightRowOutOfZec;

  /// In-flight swap row line when the USER sends the deposit externally (IntoZec).
  ///
  /// In en, this message translates to:
  /// **'Waiting for your deposit to reach the swap provider.'**
  String get walletSwapInFlightRowIntoZec;

  /// In-flight swap row line for an unrecognized direction (forward-compat) — neutral, never a guess about who sends what.
  ///
  /// In en, this message translates to:
  /// **'A swap is in progress.'**
  String get walletSwapInFlightRowGeneric;

  /// In-flight swap row line once the record's deposit window has lapsed (#367, both directions): the present-tense motion lines ('on its way' / 'waiting for your deposit') would be false for the rest of the record's ~48 h life. Neutral — the swap may have settled, refunded, or expired; View swap shows the live truth.
  ///
  /// In en, this message translates to:
  /// **'The deposit window has passed — check this swap\'s status.'**
  String get walletSwapInFlightRowPastWindow;

  /// In-flight swap row line for an UNRESOLVED record past its settlement window (#382 — such rows now list indefinitely instead of vanishing at 48 h; the wallet keeps watching every sync while unresolved), OUT-OF-ZEC + unknown-direction arm since #385 ('coming back' is refund-shaped, which is exactly the OutOfZec ZEC leg; the IntoZec row has its own delivery-shaped line). MUST stay outcome-neutral ('hasn't reached a confirmed outcome HERE' — review): the swap may in fact have SUCCEEDED unobserved, so 'taking longer than expected' would assert a falsehood over a completed swap. The second sentence is the money promise and covers only ZEC legs — it must not claim anything about a foreign-asset refund, which happens provider-side. ACCEPTED OVERCLAIM (#386, shared with the not-found body): for a pre-#368 upgrade-era record with no recorded watch leg, 'after a sync' is true only of a user-initiated full rescan — upgrade-era-only, shrinking population, documented rather than gated.
  ///
  /// In en, this message translates to:
  /// **'This swap hasn\'t reached a confirmed outcome here yet — open it to check. Any ZEC coming back to this wallet shows up in your balance after a sync.'**
  String get walletSwapInFlightRowOverdue;

  /// The IntoZec arm of the overdue row line (#385 — 'any ZEC coming back' read refund-shaped for a swap whose ZEC leg is the incoming DELIVERY; delivered ZEC never left this wallet's side). Same outcome-neutrality contract as the OutOfZec arm; the money promise covers the ZEC delivery leg only — the foreign deposit's refund, if any, happens provider-side on the source chain. Shares the #386 accepted overclaim documented on the OutOfZec arm (a watchless pre-#368 record's late ZEC is full-rescan-only).
  ///
  /// In en, this message translates to:
  /// **'This swap hasn\'t reached a confirmed outcome here yet — open it to check. Any ZEC it delivers to this wallet shows up in your balance after a sync.'**
  String get walletSwapInFlightRowOverdueIntoZec;

  /// In-flight swap row line once a SUCCESS terminal was observed and pinned (#367) — the row stays until the user removes it or opens tracking and taps Done.
  ///
  /// In en, this message translates to:
  /// **'Swap completed.'**
  String get walletSwapRowOutcomeSuccess;

  /// In-flight swap row line once a REFUNDED terminal was observed and pinned (#367). A named outcome, not an error — details (where the refund went) are on the tracking view.
  ///
  /// In en, this message translates to:
  /// **'Swap refunded.'**
  String get walletSwapRowOutcomeRefunded;

  /// In-flight swap row line once a FAILED terminal was observed and pinned (#367). Soft wording — a deposited amount settles or refunds provider-side; the row must not assert loss.
  ///
  /// In en, this message translates to:
  /// **'Swap didn\'t complete.'**
  String get walletSwapRowOutcomeFailed;

  /// Tooltip/semantics label of the per-row remove affordance on the in-flight swap list (#367).
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get walletSwapRemove;

  /// Confirm-dialog title for removing an in-flight swap row (#367).
  ///
  /// In en, this message translates to:
  /// **'Remove this swap from the list?'**
  String get walletSwapRemoveTitle;

  /// Confirm-dialog body when the row being removed has NO observed terminal yet, OUT-OF-ZEC arm ONLY since #385 (#367 origin, hedge extended by #382): the watched leg IS this wallet's refund address, so 'stop watching for its refund' and the rescan-recovery claim are true. Removing drops the only re-attach handle AND stops the unresolved-swap watch (the per-sync re-arm keys off this record); a later refund is still recoverable by rescan — never silently lost (the refund address stays registered with the wallet's own engine). The IntoZec/unknown rows use their own bodies — every claim here is FALSE for IntoZec (UX HIGH-1).
  ///
  /// In en, this message translates to:
  /// **'This only removes the swap from this list — it doesn\'t cancel the swap, and this wallet will stop watching for its refund. ZEC refunded later still belongs to this wallet; a full rescan can find it.'**
  String get walletSwapRemoveBodyInFlight;

  /// Confirm-dialog body for removing an UNRESOLVED IntoZec row (#385, UX HIGH-1 — the shared body lied to IntoZec users): the watched leg is the DELIVERY destination (this wallet's own engine-minted address), so the watch/rescan claims are about the incoming ZEC delivery; an IntoZec refund is the user's FOREIGN deposit returned on the source chain — this wallet never sees it and rescanning here can never find it, so the copy says where it happens instead.
  ///
  /// In en, this message translates to:
  /// **'This only removes the swap from this list — it doesn\'t cancel the swap, and this wallet will stop watching for its incoming ZEC. ZEC delivered later still belongs to this wallet; a full rescan can find it. If the swap is refunded instead, the refund goes back in the coin you sent, outside this wallet.'**
  String get walletSwapRemoveBodyInFlightIntoZec;

  /// Confirm-dialog body for removing an UNRESOLVED row whose direction this build doesn't recognize (forward-compat, #385): makes only the direction-independent claims — a watched leg is always one of this wallet's own engine-registered addresses (so the rescan claim holds), and no refund-location claim is made (it differs per direction).
  ///
  /// In en, this message translates to:
  /// **'This only removes the swap from this list — it doesn\'t cancel the swap, and this wallet will stop watching for ZEC still arriving from it. ZEC that arrives later still belongs to this wallet; a full rescan can find it.'**
  String get walletSwapRemoveBodyInFlightUnknown;

  /// Confirm-dialog body when the row being removed already shows its pinned terminal outcome (#367) — plain list hygiene, nothing is lost.
  ///
  /// In en, this message translates to:
  /// **'This removes the finished swap from the list.'**
  String get walletSwapRemoveBodyDone;

  /// Confirm-dialog dismiss action for the swap-row remove (#367).
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get walletSwapRemoveCancel;

  /// Confirm-dialog confirming action for the swap-row remove (#367).
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get walletSwapRemoveConfirm;

  /// In-flight swap row time stamp. {time} is a locale-formatted DATE AND TIME (month, day, and clock time — since #382 an UNRESOLVED row is unbounded in age, no longer capped at ~48h, so the date matters), so keep the sentence grammatical with a full datetime, NOT a bare clock time (no preposition that only reads for a time-of-day). Known display bound (#377): the compact format carries no YEAR, so a >1-year-old unresolved row reads year-less — the shared Activity-row idiom.
  ///
  /// In en, this message translates to:
  /// **'Started {time}'**
  String walletSwapInFlightStarted(String time);

  /// Action label that re-opens live tracking for an in-flight swap — on the wallet-screen row and on the 'a swap is already in progress' fault (W-swap-5 #366).
  ///
  /// In en, this message translates to:
  /// **'View swap'**
  String get walletSwapViewSwap;

  /// Honest error line when the durable in-flight swap list can't be read — never a silent hide (this list is the only wallet-side witness of a mid-flight swap).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t load your swaps in progress right now.'**
  String get walletSwapsInFlightError;

  /// Inline retry button under the in-flight-swaps read-error line: re-pulls the list in place (the home has no pull-to-refresh, and this list is a swap's only wallet-side witness). Same 'try again' wording as the other read-error retries (walletReceiveRetry, walletSwapPickerRetry).
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletSwapsInFlightRetry;

  /// The in-flight-swaps read-error retry button's label WHILE the re-pull is in flight (#407 R5) — the twin of walletParkedErrorRetryInProgress, and load-bearing for the same reason: the label change is what re-announces the retry to a screen reader. Keep it SHORT (it replaces 'Try again' inside a button beside a spinner).
  ///
  /// In en, this message translates to:
  /// **'Trying…'**
  String get walletSwapsInFlightRetryInProgress;

  /// Secondary action on a still-tracking swap card (W-swap-5): return to the swap form to begin a new swap. The swap being tracked is NOT cancelled — it stays listed on the wallet screen — so the wording is 'another', not 'cancel' or 'new'.
  ///
  /// In en, this message translates to:
  /// **'Start another swap'**
  String get walletSwapStartAnother;

  /// Tracking: only a partial deposit has been received.
  ///
  /// In en, this message translates to:
  /// **'Waiting for the full deposit'**
  String get walletSwapStatusUnderTitle;

  /// Tracking body for the under-deposited state when the WALLET sent the deposit (OutOfZec) — the user cannot top up a wallet-sent deposit, so the body stays passive (completing or refunding provider-side).
  ///
  /// In en, this message translates to:
  /// **'Part of the deposit has arrived. The rest is completing, or the provider will refund.'**
  String get walletSwapStatusUnderBody;

  /// Tracking body for the under-deposited state when the USER sends the deposit externally (IntoZec, #367): unlike the OutOfZec arm the user CAN act — top up the missing amount — so the body says so, with the honest refund fallback.
  ///
  /// In en, this message translates to:
  /// **'Part of your deposit has arrived. Send the missing amount before the deadline, or the provider refunds what arrived.'**
  String get walletSwapStatusUnderBodyIntoZec;

  /// Tracking detail under the under-deposited body (#367 — these DTO fields existed and were never rendered): the provider's received/missing amounts as DECIMAL STRINGS in the deposit asset's units (render verbatim — never re-computed host-side) and the top-up deadline. {time} is a locale-formatted DATE AND TIME (month, day, and clock time — the window can cross a day boundary), so keep the sentence grammatical with a full datetime, NOT a bare clock time (no preposition that only reads for a time-of-day).
  ///
  /// In en, this message translates to:
  /// **'Received {received}; {missing} still missing. The deposit window ends {time}.'**
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  );

  /// Tracking: the provider detected the deposit.
  ///
  /// In en, this message translates to:
  /// **'Deposit received'**
  String get walletSwapStatusDetectedTitle;

  /// Tracking body for the deposit-detected state.
  ///
  /// In en, this message translates to:
  /// **'The provider received your deposit and will process the swap.'**
  String get walletSwapStatusDetectedBody;

  /// Tracking: the provider is processing the swap.
  ///
  /// In en, this message translates to:
  /// **'Processing your swap'**
  String get walletSwapStatusProcessingTitle;

  /// Tracking body for the processing state.
  ///
  /// In en, this message translates to:
  /// **'The provider is completing your swap.'**
  String get walletSwapStatusProcessingBody;

  /// Tracking: terminal success.
  ///
  /// In en, this message translates to:
  /// **'Swap complete'**
  String get walletSwapStatusSuccessTitle;

  /// Tracking body for the success state.
  ///
  /// In en, this message translates to:
  /// **'Your swap finished successfully.'**
  String get walletSwapStatusSuccessBody;

  /// Tracking: terminal refund (a named outcome, not an error).
  ///
  /// In en, this message translates to:
  /// **'Swap refunded'**
  String get walletSwapStatusRefundedTitle;

  /// Tracking body for the refunded state when the USER sent the deposit (IntoZec, #367): the refund goes to the user's own refund address on the SOURCE chain — this wallet never sees it, so the body says where to look instead of the old placeless 'your funds were refunded'.
  ///
  /// In en, this message translates to:
  /// **'The swap didn\'t complete, so the provider sent the funds back to your refund address.'**
  String get walletSwapStatusRefundedBody;

  /// Tracking body for the refunded state when the WALLET sent the deposit (OutOfZec; #368 replaced the #367 'may not appear yet' interim): the refund address is watched (refund-index registration), viewing this screen re-arms the watch, and since #382 EVERY sync pass re-arms it while the swap is unresolved — so 'shows up in your balance after the wallet next syncs' is mechanical for ANY absence length, not only within 48 h of executing. Keep the timing hedge ('can take a little while') — the provider's refund transaction must mine and a sync pass must run before the balance moves; never promise instant.
  ///
  /// In en, this message translates to:
  /// **'The swap didn\'t complete, so the provider sent your ZEC back to this wallet. It arrives as unshielded funds and shows up in your balance after the wallet next syncs — this can take a little while.'**
  String get walletSwapStatusRefundedBodyOutOfZec;

  /// Tracking: terminal failure.
  ///
  /// In en, this message translates to:
  /// **'Swap failed'**
  String get walletSwapStatusFailedTitle;

  /// Tracking body for the failed state (funds-safety honest).
  ///
  /// In en, this message translates to:
  /// **'The swap couldn\'t be completed. Any deposited funds settle or refund on the provider\'s side.'**
  String get walletSwapStatusFailedBody;

  /// Tracking: the SDK's poll policy concluded the provider no longer recognizes this swap (#367 — several consecutive definitive not-found answers; most likely the order expired and was cleaned up provider-side).
  ///
  /// In en, this message translates to:
  /// **'Swap not found'**
  String get walletSwapStatusNotFoundTitle;

  /// Tracking body for the not-found terminal (#367; last sentence added by #385, hedge sharpened by #386). Must NOT assert loss: a deposit that landed on an expired order is refunded provider-side to the recorded refund address; 'most likely expired' stays hedged (the provider can no longer tell us anything definitive). Since #385 this card's Done does NOT dismiss the still-unresolved record (MED-1ux — not-found is a heuristic, never pinned, and a silent dismiss voided the watch the overdue row had just promised), so the copy says the swap stays listed + watched and points at the list's Remove (which carries the full disclosure dialog). The watching claim is hedged 'in case it still arrives' (M-1, precision-fixed by #386: 'until it has arrived' PRESUPPOSED an arrival, but this card's own headline case — an expired order whose foreign-coin deposit is refunded provider-side on the source chain — never delivers ZEC here at all); a served leg (money arrived, then shielded/moved) stops the per-pass watch. ACCEPTED OVERCLAIM (#386, documented rather than gated): a pre-#368 upgrade-era record with no recorded watch leg (NULL watch columns, its one-shot backfill window spent) is NOT per-pass watched — its late ZEC is engine-registered and surfaces on a user-initiated full rescan only. That population is upgrade-era-only and shrinking; gating this sentence on watch presence would need a new DTO bit for a corner that retires itself. HEDGED (review M5): 'should refund' — this card's own premise is a GC'd order, the exact case that makes an unconditional refund promise false.
  ///
  /// In en, this message translates to:
  /// **'The provider no longer has a record of this swap — it most likely expired. If a deposit was made, the provider should refund it to the refund address. The swap stays in your list, and this wallet keeps watching for its ZEC in case it still arrives — you can remove it from the list anytime.'**
  String get walletSwapStatusNotFoundBody;

  /// Tracking: a forward-compat status this build can't name (neutral, never alarming).
  ///
  /// In en, this message translates to:
  /// **'Status unavailable'**
  String get walletSwapStatusUnknownTitle;

  /// Tracking body for the unknown state.
  ///
  /// In en, this message translates to:
  /// **'We can\'t read this swap\'s status right now.'**
  String get walletSwapStatusUnknownBody;

  /// Tracking: swap was turned off, so live tracking stopped (§3.5 host kill state).
  ///
  /// In en, this message translates to:
  /// **'Tracking unavailable'**
  String get walletSwapTrackingUnavailableTitle;

  /// Tracking body when the host has killed swap (§3.5 — funds-safety honest). SINCE #382 UNREFERENCED by the package (both directions render their own honest kill body — the IntoZec/OutOfZec siblings); retained for the #347 l10n review to prune rather than churn 16 locales mid-GA.
  ///
  /// In en, this message translates to:
  /// **'Swap is turned off, so we can\'t track this here. Any funds settle or refund on the provider\'s side.'**
  String get walletSwapTrackingUnavailableBody;

  /// OutOfZec tracking: the honest killed-swap message (#382 — the OutOfZec mirror of the IntoZec L8 body). Pre-#382 this arm said funds 'settle or refund on the provider's side' — a misdirect once #368 made refunds land at THIS wallet's own address (the provider would truthfully answer 'we already sent it back'). Mechanics: the kill clears the detection watch, but the refund address stays engine-registered and the unresolved-swap re-arm restores the watch on the first sync after swap is re-enabled — so the promise is mechanical, conditioned on swap being turned back on.
  ///
  /// In en, this message translates to:
  /// **'Swap is turned off here, so this swap can\'t be tracked right now. If it was refunded, the ZEC comes back to this wallet — it shows up in your balance after swap is turned back on and the wallet syncs.'**
  String get walletSwapTrackingUnavailableBodyOutOfZec;

  /// Tracking: an establish-time typed failure (the stream can't be opened).
  ///
  /// In en, this message translates to:
  /// **'We couldn\'t track this swap.'**
  String get walletSwapTrackingError;

  /// Body of the tracking ESTABLISH-failure card (review M1): its own body — the failed-status body ('The swap couldn't be completed') contradicted the title on a money claim; an establish failure says nothing about the swap's outcome and must not read as a failure verdict.
  ///
  /// In en, this message translates to:
  /// **'We couldn\'t open tracking for this swap. The swap itself may still be going ahead — any deposited funds settle or refund on the provider\'s side.'**
  String get walletSwapTrackingErrorBody;

  /// Form fault: the destination address was empty (OutOfZec requires it).
  ///
  /// In en, this message translates to:
  /// **'Enter the address where you want to receive the swapped asset.'**
  String get walletSwapFaultDestinationRequired;

  /// Form fault: the destination was rejected by the SDK.
  ///
  /// In en, this message translates to:
  /// **'That destination address isn\'t valid for this asset. Check it and try again.'**
  String get walletSwapFaultDestinationInvalid;

  /// Form fault: the quote deadline lapsed — re-quote.
  ///
  /// In en, this message translates to:
  /// **'This quote expired. Get a fresh quote to continue.'**
  String get walletSwapFaultExpired;

  /// Form fault: the quote fell outside the user-anchored bound (protective).
  ///
  /// In en, this message translates to:
  /// **'The provider\'s price moved outside your limit, so the swap was stopped before anything moved. Try again.'**
  String get walletSwapFaultOutOfBounds;

  /// Form fault: requested slippage above the SDK ceiling (defensive).
  ///
  /// In en, this message translates to:
  /// **'The slippage limit is too high for a safe swap. Try again.'**
  String get walletSwapFaultSlippageTooHigh;

  /// Form fault: provider unreachable/erroring (retryable).
  ///
  /// In en, this message translates to:
  /// **'The swap provider is unavailable right now. Try again in a moment.'**
  String get walletSwapFaultProviderUnavailable;

  /// Form fault: the swap request to the 1Click service timed out / the connection broke (host-side timeout) — the user's connectivity is the likely cause, distinct from the provider itself being down.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t reach the swap service. Please check your internet connection and try again.'**
  String get walletSwapFaultConnection;

  /// Form fault: the provider broke the protocol contract.
  ///
  /// In en, this message translates to:
  /// **'The swap provider returned an unexpected response, so the swap was stopped. Try again.'**
  String get walletSwapFaultProviderMisbehaved;

  /// Form fault: swap disabled at this instance (defensive). Also the classifier verdict for the defensively-unreachable watchOnly kind — if that kind ever becomes reachable at quote/execute, promote it to a dedicated permanent-framing key (walletSwapUnavailableWatchOnly is the model).
  ///
  /// In en, this message translates to:
  /// **'Swap is turned off right now.'**
  String get walletSwapFaultSwapOff;

  /// Form fault: our side couldn't queue the deposit (no ZEC moved; re-quote).
  ///
  /// In en, this message translates to:
  /// **'We couldn\'t send your deposit, so nothing left your wallet. Get a fresh quote to try again.'**
  String get walletSwapFaultDepositFailed;

  /// Form fault: the SDK's one-deposit-in-flight guard refused a second swap while one is still queued/signing/sending/settling. Deliberately does NOT invite a re-quote (re-quoting is the double-deposit door). 'Fully settles … can take a while' is honest about the settlement tail: the guard clears at reorg-final burial (~2 h after the deposit mines) or after quote expiry + tx expiry (review NIT — the earlier copy implied an immediate clear).
  ///
  /// In en, this message translates to:
  /// **'A swap is already in progress. You can start a new one after it fully settles or its quote expires — this can take a while.'**
  String get walletSwapFaultAlreadyInFlight;

  /// Form fault: our side couldn't mint a fresh refund address (#382 rewrite). The DOMINANT real cause since #368 is a pre-first-sync Sell — the refund mints through the engine, which needs the lazily-provisioned account, and the swap surface is activation-gated — so the copy names the wait-for-sync remedy instead of the pre-#382 bare 'try again' (which looped false hope while lightwalletd was down and the swap provider up). 'Usually' keeps the rare structural tail honest.
  ///
  /// In en, this message translates to:
  /// **'This wallet can\'t set up a refund address yet — that usually just means the first sync hasn\'t finished. Wait for the sync to complete, then try again.'**
  String get walletSwapFaultRefundUnavailable;

  /// Form fault: our side couldn't mint a fresh swap receiving address (IntoZec — the #382 mirror of walletSwapFaultRefundUnavailable; pre-#382 this kind fell through to the generic could-not-quote). Same pre-first-sync dominant cause, same wait-for-sync remedy. 'Receiving address' means the wallet-side ZEC delivery address, NOT the user's typed destination.
  ///
  /// In en, this message translates to:
  /// **'This wallet can\'t set up a receiving address for this swap yet — that usually just means the first sync hasn\'t finished. Wait for the sync to complete, then try again.'**
  String get walletSwapFaultDestinationUnavailable;

  /// IntoZec execute overran the host-side timeout (#367, F5): the cause may be transport OR a local stall (a busy store consuming most of the window), so this HEDGES both — unlike walletSwapFaultConnection, it must not firmly blame the user's connection. Money-safe: an IntoZec execute moves no wallet funds; re-quoting is the remedy.
  ///
  /// In en, this message translates to:
  /// **'The swap couldn\'t start in time — the connection may be slow, or the wallet was busy. Get a new quote and try again.'**
  String get walletSwapFaultExecuteTimeout;

  /// Retryable fault (#367): the wallet's own store was momentarily busy and NOTHING was consumed — re-running the same action works. On the review screen it renders inline and Start swap is the retry (the quote is still valid); on the form, tapping Get quote again is the retry. Distinct from walletSwapFaultStateUnavailable (whose remedy is a re-quote).
  ///
  /// In en, this message translates to:
  /// **'The wallet is busy for a moment. Try again.'**
  String get walletSwapFaultStoreBusyRetry;

  /// Stage S8 (R01): the quote handed to execute names a quote the wallet issued but its terms (address, amounts, memo, refund target, binding) differ from the wallet's own durable record — refused BEFORE the quote's single-use claim, so nothing was consumed and nothing left the wallet. Must state that nothing was sent (true by construction) and point at a fresh quote; must not accuse the provider (the DTO was altered on the way back through the host, not by the provider).
  ///
  /// In en, this message translates to:
  /// **'This quote doesn\'t match the one your wallet issued, so nothing was sent. Get a fresh quote and try again.'**
  String get walletSwapFaultTermsDiffer;

  /// Spendable pre-check refusal at quote review (#367): the deposit plus a conservative network-fee allowance exceeds what is spendable. 'about' is load-bearing — the needed figure includes an allowance, not the exact fee. Amounts are locale-formatted ZEC decimals.
  ///
  /// In en, this message translates to:
  /// **'This swap needs about {needed} ZEC including the network fee, but only {spendable} ZEC is spendable right now.'**
  String walletSwapFaultInsufficient(String needed, String spendable);

  /// Swap form: the host's FR-23 alpha ceiling bounds the swap deposit; stated as an app restriction, never as an invalid amount (#364 S6 — its own key: the send form's 'limits sends' copy misread on a swap form).
  ///
  /// In en, this message translates to:
  /// **'This app currently limits swaps to {limit} ZEC.'**
  String walletSwapFaultOverCeiling(String limit);

  /// The catching-up variant of walletSwapFaultInsufficient (#367): the wallet is mid catch-up/rescan, so the spendable figure may be partial — the closing hedge stops the refusal reading as a final verdict over a partial figure. Keep the hedge conditional ('may'), never a promise.
  ///
  /// In en, this message translates to:
  /// **'This swap needs about {needed} ZEC including the network fee, but only {spendable} ZEC is spendable right now. Your balance is still catching up — more may become spendable soon.'**
  String walletSwapFaultInsufficientCatchingUp(String needed, String spendable);

  /// Form fault: our side couldn't persist durable swap state (fail-closed).
  ///
  /// In en, this message translates to:
  /// **'The wallet couldn\'t safely record this swap, so nothing moved. Try again.'**
  String get walletSwapFaultStateUnavailable;

  /// Form fault: not-issued/already-executed/malformed request — re-quote.
  ///
  /// In en, this message translates to:
  /// **'That swap request couldn\'t be processed. Get a fresh quote and try again.'**
  String get walletSwapFaultRequestInvalid;

  /// Form fault: a generic quote/execute failure with no finer mapping.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t get a swap quote. Check the details and try again.'**
  String get walletSwapFaultCouldNotQuote;

  /// Form fault: no live wallet session (defensive).
  ///
  /// In en, this message translates to:
  /// **'Your wallet isn\'t ready right now. Go back and try again.'**
  String get walletSwapFaultWalletUnavailable;

  /// Swap form: the IntoZec direction segment (the default) — buy ZEC with another asset.
  ///
  /// In en, this message translates to:
  /// **'Buy ZEC'**
  String get walletSwapDirectionBuy;

  /// Swap form: the OutOfZec direction segment — sell ZEC for another asset.
  ///
  /// In en, this message translates to:
  /// **'Sell ZEC'**
  String get walletSwapDirectionSell;

  /// IntoZec form: the source-chain refund address field label.
  ///
  /// In en, this message translates to:
  /// **'Your refund address'**
  String get walletSwapRefundLabel;

  /// IntoZec form: the refund address field hint.
  ///
  /// In en, this message translates to:
  /// **'Where your coins return if the swap fails'**
  String get walletSwapRefundHint;

  /// IntoZec form: helper text clarifying the refund address is a foreign-chain address.
  ///
  /// In en, this message translates to:
  /// **'On the chain you\'re sending from — not a Zcash address.'**
  String get walletSwapRefundHelper;

  /// IntoZec form: the refund field label once a source asset is picked, naming its chain.
  ///
  /// In en, this message translates to:
  /// **'Your {chain} refund address'**
  String walletSwapRefundLabelChain(String chain);

  /// IntoZec form: chain-aware helper for the refund field once a source asset is picked.
  ///
  /// In en, this message translates to:
  /// **'A {chain} address — where your coins return if the swap fails. Not a Zcash address.'**
  String walletSwapRefundHelperChain(String chain);

  /// IntoZec form: title of the refund-address explainer dialog.
  ///
  /// In en, this message translates to:
  /// **'About your refund address'**
  String get walletSwapRefundInfoTitle;

  /// IntoZec form: body of the refund-address explainer dialog (§3.3b D6).
  ///
  /// In en, this message translates to:
  /// **'If the swap can\'t complete, the provider sends your coins back to this address on the chain you paid from. Enter an address you control — the wallet can\'t check a foreign address for you, so verify it carefully.'**
  String get walletSwapRefundInfoBody;

  /// IntoZec form: tooltip on the refund-address QR scan button (IZ-4; mobile only).
  ///
  /// In en, this message translates to:
  /// **'Scan a refund-address QR code'**
  String get walletSwapRefundScanTooltip;

  /// Address QR scanner screen: app-bar title (shared by the IntoZec refund + OutOfZec destination scans).
  ///
  /// In en, this message translates to:
  /// **'Scan address'**
  String get walletSwapScanTitle;

  /// Address QR scanner screen: the aiming hint / accessible label (shared brick).
  ///
  /// In en, this message translates to:
  /// **'Point your camera at the address QR code.'**
  String get walletSwapScanInstruction;

  /// Address QR scanner screen: the always-present escape button that returns to the type/paste field (shared brick).
  ///
  /// In en, this message translates to:
  /// **'Enter manually'**
  String get walletSwapScanManualEntry;

  /// Address QR scanner screen: the close-button tooltip (shared brick).
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get walletSwapScanCancel;

  /// Address QR scanner screen: honest message when the camera can't start (permission denied / no camera) (shared brick).
  ///
  /// In en, this message translates to:
  /// **'Camera unavailable. Enter the address manually below.'**
  String get walletSwapScanCameraUnavailable;

  /// IntoZec form: the source-asset picker field label.
  ///
  /// In en, this message translates to:
  /// **'Asset to swap from'**
  String get walletSwapSourceAssetLabel;

  /// IntoZec form: the source-asset picker placeholder before an asset is chosen.
  ///
  /// In en, this message translates to:
  /// **'Select an asset'**
  String get walletSwapSourceAssetHint;

  /// IntoZec form: the foreign amount field label once an asset is picked.
  ///
  /// In en, this message translates to:
  /// **'Amount to send ({symbol})'**
  String walletSwapForeignAmountLabel(String symbol);

  /// IntoZec form: the foreign amount field label before an asset is picked.
  ///
  /// In en, this message translates to:
  /// **'Amount to send'**
  String get walletSwapForeignAmountLabelGeneric;

  /// Review: a foreign amount + asset, e.g. the IntoZec 'you send' line.
  ///
  /// In en, this message translates to:
  /// **'{amount} {asset}'**
  String walletSwapForeignValue(String amount, String asset);

  /// Token picker: a source asset's display label (symbol + chain, both uppercased).
  ///
  /// In en, this message translates to:
  /// **'{symbol} on {chain}'**
  String walletSwapTokenLabel(String symbol, String chain);

  /// Token picker sheet title for the IntoZec (Buy) direction — the SOURCE asset the user swaps FROM.
  ///
  /// In en, this message translates to:
  /// **'Choose an asset to swap from'**
  String get walletSwapPickerTitle;

  /// Token picker sheet title for the OutOfZec (Sell) direction — the TARGET asset the user receives (the shared picker is direction-neutral; the caller supplies the framing).
  ///
  /// In en, this message translates to:
  /// **'Choose an asset to receive'**
  String get walletSwapPickerTitleReceive;

  /// Token picker: the L6 serve-stale banner (the live fetch failed; cached data shown).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t refresh the asset list — showing the last known list.'**
  String get walletSwapPickerStale;

  /// Token picker: the honest empty state (no assets after filtering).
  ///
  /// In en, this message translates to:
  /// **'No assets are available to swap right now. Try again later.'**
  String get walletSwapPickerEmpty;

  /// Token picker: placeholder in the search field.
  ///
  /// In en, this message translates to:
  /// **'Search by name or chain'**
  String get walletSwapPickerSearchHint;

  /// Token picker: shown when the search query matches no asset.
  ///
  /// In en, this message translates to:
  /// **'No assets match \"{query}\".'**
  String walletSwapPickerNoMatch(String query);

  /// Token picker: a first-ever fetch failure with no cache.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t load the asset list. Check your connection and try again.'**
  String get walletSwapPickerError;

  /// Token picker: retry button after a load error.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletSwapPickerRetry;

  /// Swap form: the slippage control label (§3.3b D4).
  ///
  /// In en, this message translates to:
  /// **'Slippage tolerance'**
  String get walletSwapSlippageLabel;

  /// Swap form: a slippage preset/value rendered as a percent.
  ///
  /// In en, this message translates to:
  /// **'{value}%'**
  String walletSwapSlippagePercent(String value);

  /// Swap form: the custom-slippage chip.
  ///
  /// In en, this message translates to:
  /// **'Custom'**
  String get walletSwapSlippageCustom;

  /// Swap form: the custom-slippage percent field label.
  ///
  /// In en, this message translates to:
  /// **'Custom slippage'**
  String get walletSwapSlippageCustomLabel;

  /// Swap form: advisory for a too-tight slippage tolerance.
  ///
  /// In en, this message translates to:
  /// **'Very low — the swap may fail if the price moves.'**
  String get walletSwapSlippageMayFail;

  /// Swap form: advisory for a normal slippage tolerance.
  ///
  /// In en, this message translates to:
  /// **'A safe tolerance.'**
  String get walletSwapSlippageNormal;

  /// Swap form: advisory for a wide slippage tolerance.
  ///
  /// In en, this message translates to:
  /// **'High — you could receive noticeably less than quoted.'**
  String get walletSwapSlippageRisky;

  /// Swap form: advisory for a slippage beyond the SDK hard ceiling.
  ///
  /// In en, this message translates to:
  /// **'Too high — the swap will be rejected. Lower it to 10% or less.'**
  String get walletSwapSlippageTooHigh;

  /// IntoZec review: the honest guaranteed-minimum / max-cost line (§3.3b L8).
  ///
  /// In en, this message translates to:
  /// **'You\'ll receive at least {zec} ZEC — your {slippage}% slippage floor. The final amount won\'t drop below this.'**
  String walletSwapIntoZecFloorNote(String zec, String slippage);

  /// IntoZec review: the positive end-state card title.
  ///
  /// In en, this message translates to:
  /// **'You receive ZEC to your own address'**
  String get walletSwapIntoZecShieldTitle;

  /// IntoZec review: the pinned ends-shielded honesty copy (§3.3b D1).
  ///
  /// In en, this message translates to:
  /// **'Until you shield it — one tap, nudged on arrival — the received amount is briefly public and visible on-chain. A small delivery may stay public until it accumulates.'**
  String get walletSwapIntoZecEndsShielded;

  /// IntoZec review: the refund-address verification step title (§3.3b D6).
  ///
  /// In en, this message translates to:
  /// **'Verify your refund address'**
  String get walletSwapRefundVerifyTitle;

  /// IntoZec review: the refund-address verification instruction.
  ///
  /// In en, this message translates to:
  /// **'Check it character by character — this is where your coins return if the swap fails. The wallet can\'t verify a foreign address for you.'**
  String get walletSwapRefundVerifyBody;

  /// IntoZec review: the distinct refund-verification acknowledgment (separate from the privacy ack).
  ///
  /// In en, this message translates to:
  /// **'I\'ve checked my refund address is correct.'**
  String get walletSwapRefundVerifyAck;

  /// OutOfZec review: the payout-address verification step title — the user's own foreign address where the swapped asset is sent.
  ///
  /// In en, this message translates to:
  /// **'Verify your receiving address'**
  String get walletSwapPayoutVerifyTitle;

  /// OutOfZec review: the payout-address verification instruction. {asset} is the asset label, e.g. "USDC on Ethereum".
  ///
  /// In en, this message translates to:
  /// **'Check it character by character — this is where you\'ll receive {asset}. The wallet can\'t verify a foreign address for you.'**
  String walletSwapPayoutVerifyBody(String asset);

  /// OutOfZec review: the distinct payout-verification acknowledgment (separate from the privacy ack).
  ///
  /// In en, this message translates to:
  /// **'I\'ve checked my receiving address is correct.'**
  String get walletSwapPayoutVerifyAck;

  /// IntoZec tracking: the §3.3b L8 honest killed-swap message (delivery arrives on the next sync).
  ///
  /// In en, this message translates to:
  /// **'Swap is turned off here. Any ZEC already on its way will appear in your wallet after your next sync.'**
  String get walletSwapTrackingUnavailableBodyIntoZec;

  /// IntoZec form fault: the source amount was empty.
  ///
  /// In en, this message translates to:
  /// **'Enter the amount you want to swap.'**
  String get walletSwapFaultForeignAmountRequired;

  /// IntoZec form fault: the refund address was empty.
  ///
  /// In en, this message translates to:
  /// **'Enter your refund address on the source chain.'**
  String get walletSwapFaultRefundAddressRequired;

  /// IntoZec deposit screen: title (§3.3b D7).
  ///
  /// In en, this message translates to:
  /// **'Send your payment'**
  String get walletSwapDepositTitle;

  /// IntoZec deposit screen: the send instruction.
  ///
  /// In en, this message translates to:
  /// **'Send exactly {amount} {asset} on {chain} to the address below.'**
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  );

  /// IntoZec deposit screen: the exact-amount honesty note (§4.4).
  ///
  /// In en, this message translates to:
  /// **'Send the exact amount. Sending less, or sending after the window closes, means the provider refunds you to your refund address.'**
  String get walletSwapDepositExactNote;

  /// IntoZec deposit screen: the live deadline countdown.
  ///
  /// In en, this message translates to:
  /// **'Deposit window: {time} left'**
  String walletSwapDepositExpiresIn(String time);

  /// IntoZec deposit screen: the expired-window message. HEDGED (review MED): 'should refund', never 'will' — a below-minimum/dust late deposit or a GC'd order can make an unconditional promise false on a money screen.
  ///
  /// In en, this message translates to:
  /// **'This deposit window has closed. Don\'t send funds now — start a new swap. If you already sent, the provider should refund to your refund address.'**
  String get walletSwapDepositExpired;

  /// IntoZec deposit screen: accessibility label for the deposit-address QR.
  ///
  /// In en, this message translates to:
  /// **'QR code of the deposit address'**
  String get walletSwapDepositQrLabel;

  /// IntoZec deposit screen: the deposit-address field label.
  ///
  /// In en, this message translates to:
  /// **'Deposit address'**
  String get walletSwapDepositAddressLabel;

  /// IntoZec deposit screen: copy-to-clipboard button.
  ///
  /// In en, this message translates to:
  /// **'Copy deposit address'**
  String get walletSwapDepositCopy;

  /// IntoZec deposit screen: snackbar after copying the deposit address.
  ///
  /// In en, this message translates to:
  /// **'Deposit address copied'**
  String get walletSwapDepositCopied;

  /// IntoZec deposit screen: heading of the required-memo section (some source chains, e.g. XRP/Cosmos, require a destination tag or memo on the deposit).
  ///
  /// In en, this message translates to:
  /// **'This deposit needs a memo / tag'**
  String get walletSwapDepositMemoRequired;

  /// IntoZec deposit screen: the funds-loss warning above the required deposit memo.
  ///
  /// In en, this message translates to:
  /// **'You MUST include this exact memo with your deposit. Sending without it — or with the wrong memo — can permanently lose your funds.'**
  String get walletSwapDepositMemoWarning;

  /// IntoZec deposit screen: label for the required deposit memo value.
  ///
  /// In en, this message translates to:
  /// **'Deposit memo / tag'**
  String get walletSwapDepositMemoLabel;

  /// IntoZec deposit screen: copy-the-memo-to-clipboard button.
  ///
  /// In en, this message translates to:
  /// **'Copy memo'**
  String get walletSwapDepositMemoCopy;

  /// IntoZec deposit screen: snackbar after copying the deposit memo.
  ///
  /// In en, this message translates to:
  /// **'Memo copied'**
  String get walletSwapDepositMemoCopied;

  /// IntoZec deposit screen: advance-to-tracking affordance.
  ///
  /// In en, this message translates to:
  /// **'I\'ve sent the funds'**
  String get walletSwapDepositSent;

  /// IntoZec deposit screen: back-press guard dialog title.
  ///
  /// In en, this message translates to:
  /// **'Leave this screen?'**
  String get walletSwapDepositBackTitle;

  /// IntoZec deposit screen: back-press guard dialog body (the in-flight reassurance).
  ///
  /// In en, this message translates to:
  /// **'This won\'t cancel your swap — it continues in the background. But you\'ll need the deposit address to pay, so copy it first if you haven\'t.'**
  String get walletSwapDepositBackBody;

  /// Leave-screen dialog body ONCE THE DEPOSIT WINDOW HAS EXPIRED (#364 F11): the live-window body invites copying the address 'to pay', which post-expiry is exactly what the user must not do — this variant warns off sending instead. HEDGED (review MED): 'should refund', never 'will' — same rule as walletSwapDepositExpired.
  ///
  /// In en, this message translates to:
  /// **'This won\'t cancel your swap — it continues in the background. The deposit window has closed, so don\'t send funds to the deposit address now. If you already sent, the provider should refund to your refund address.'**
  String get walletSwapDepositBackBodyExpired;

  /// IntoZec deposit screen: back-press guard — stay on the screen.
  ///
  /// In en, this message translates to:
  /// **'Stay'**
  String get walletSwapDepositBackStay;

  /// IntoZec deposit screen: back-press guard — confirm leaving.
  ///
  /// In en, this message translates to:
  /// **'Leave'**
  String get walletSwapDepositBackLeave;

  /// Wallet-surface button + receive screen title.
  ///
  /// In en, this message translates to:
  /// **'Receive'**
  String get walletReceive;

  /// Receive screen: explains the address is public and shareable.
  ///
  /// In en, this message translates to:
  /// **'Share this address to receive ZEC. It\'s safe to share publicly.'**
  String get walletReceiveSubtitle;

  /// Receive screen: copy-to-clipboard button label.
  ///
  /// In en, this message translates to:
  /// **'Copy address'**
  String get walletReceiveCopy;

  /// Receive screen: snackbar confirmation after copying the address.
  ///
  /// In en, this message translates to:
  /// **'Address copied'**
  String get walletReceiveCopied;

  /// Receive screen: shown when there is no live wallet session.
  ///
  /// In en, this message translates to:
  /// **'Your wallet isn\'t ready yet.'**
  String get walletReceiveUnavailable;

  /// Receive screen: address lookup failed; paired with a Try again button.
  ///
  /// In en, this message translates to:
  /// **'We couldn\'t load your address. Please try again.'**
  String get walletReceiveError;

  /// Receive screen: honest loading headline while the address derivation is in flight (replaces a bare spinner).
  ///
  /// In en, this message translates to:
  /// **'Preparing your address…'**
  String get walletReceivePreparing;

  /// Receive screen: reassuring sub-line under the loading headline. CAUSE-HONEST since #385 (E2E-1): the derivation is LOCAL — the old 'catches up with the network' blamed the network on a fully-synced wallet; the honest cause is the derive queueing behind other wallet work (a heavy sync being one example, not the only one). Avoids 'first sync' since a restore also hits this on a fresh device.
  ///
  /// In en, this message translates to:
  /// **'Your wallet prepares this address on your device — it can take a moment if the wallet is busy with other work.'**
  String get walletReceivePreparingHint;

  /// Receive screen: button that re-attempts the address load after a failure.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletReceiveRetry;

  /// Receive screen: accessibility label for the address QR image.
  ///
  /// In en, this message translates to:
  /// **'QR code of your receive address'**
  String get walletReceiveQrLabel;

  /// Receive screen: the address-type toggle segment for the private shielded address (the default).
  ///
  /// In en, this message translates to:
  /// **'Shielded'**
  String get walletReceiveTypeShielded;

  /// Receive screen: the address-type toggle segment for the public transparent address.
  ///
  /// In en, this message translates to:
  /// **'Public'**
  String get walletReceiveTypeTransparent;

  /// Receive screen: subtitle shown when the transparent address is selected.
  ///
  /// In en, this message translates to:
  /// **'Share this public address to receive ZEC from a sender that can\'t pay a shielded address.'**
  String get walletReceiveSubtitleTransparent;

  /// Receive screen: the honest public-address warning shown above the transparent receive address.
  ///
  /// In en, this message translates to:
  /// **'This is a public address: it\'s visible on-chain and links your payments if reused. Prefer your shielded address; shield these funds after receiving.'**
  String get walletReceiveTransparentWarning;

  /// Receive screen: accessibility label for the transparent address QR image.
  ///
  /// In en, this message translates to:
  /// **'QR code of your public receive address'**
  String get walletReceiveQrLabelTransparent;

  /// Receive screen (shielded tab): button that mints a fresh diversified address — a new unlinkable address for a contact or invoice that still pays into this wallet.
  ///
  /// In en, this message translates to:
  /// **'Use a fresh address'**
  String get walletReceiveFreshAddress;

  /// Receive screen: note shown with a freshly minted diversified address. Must convey all four facts: unlinkable; funds arrive in this wallet; earlier addresses stay valid; the display is one-time (copy before leaving).
  ///
  /// In en, this message translates to:
  /// **'Fresh address — can\'t be linked to your other addresses. Payments to it arrive in this wallet, and your earlier addresses keep working. It won\'t be shown here again — copy it now.'**
  String get walletReceiveFreshCaption;

  /// Receive screen: snackbar shown when minting a fresh diversified address fails; the previous address stays on screen.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t create a fresh address. Try again.'**
  String get walletReceiveFreshError;

  /// Receive screen: snackbar when the fresh-address mint timed out because the wallet is briefly busy (e.g. signing a payment); a retry normally succeeds.
  ///
  /// In en, this message translates to:
  /// **'The wallet is busy right now. Try the fresh address again in a moment.'**
  String get walletReceiveFreshBusy;

  /// Receive screen: button that opens the host's share sheet with the address, or the payment request when an amount is set (S13, maintainer). Shown only when the host supplies a share hook.
  ///
  /// In en, this message translates to:
  /// **'Share'**
  String get walletReceiveShare;

  /// Receive screen: button that opens the optional amount field; with an amount, the QR, Copy and Share carry a payment request for it (S13, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Request amount'**
  String get walletReceiveRequestAmount;

  /// Receive screen: label of the requested-amount field, in ZEC (S13, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Amount (optional)'**
  String get walletReceiveRequestAmountLabel;

  /// Receive screen: the fresh-address note's on-screen state, the last sentence of walletReceiveFreshCaption verbatim; the whole caption moved behind the note's (i) (S13 §1.7: the 'copy it now' state stays on screen).
  ///
  /// In en, this message translates to:
  /// **'It won\'t be shown here again — copy it now.'**
  String get walletReceiveFreshCopyNow;

  /// Wallet overflow menu: opens the Security screen (key custody + delete wallet).
  ///
  /// In en, this message translates to:
  /// **'Security…'**
  String get walletSecurityMenuItem;

  /// Security screen: app bar title.
  ///
  /// In en, this message translates to:
  /// **'Security'**
  String get securityTitle;

  /// Security screen: honest body when the host app owns wallet custody (no package provisioner is wired), so the package's custody probe and delete-wallet actions are not available here.
  ///
  /// In en, this message translates to:
  /// **'Wallet security settings are managed by this app, not by the wallet itself.'**
  String get securityUnavailableBody;

  /// Security screen: section header for where the wallet keys are protected.
  ///
  /// In en, this message translates to:
  /// **'Key custody'**
  String get securityCustodySectionTitle;

  /// Security screen: custody tier name for Apple Secure Enclave.
  ///
  /// In en, this message translates to:
  /// **'Secure Enclave (hardware)'**
  String get securityCustodyTierSecureEnclave;

  /// Security screen: custody tier name for Android StrongBox.
  ///
  /// In en, this message translates to:
  /// **'StrongBox (hardware)'**
  String get securityCustodyTierStrongBox;

  /// Security screen: custody tier name for an Android TEE-backed keystore.
  ///
  /// In en, this message translates to:
  /// **'Hardware keystore (TEE)'**
  String get securityCustodyTierTee;

  /// Security screen: custody tier name for a software-rooted keystore.
  ///
  /// In en, this message translates to:
  /// **'Software keystore'**
  String get securityCustodyTierSoftware;

  /// Security screen: custody tier name for the raw Apple keychain fallback.
  ///
  /// In en, this message translates to:
  /// **'Keychain (software-encrypted)'**
  String get securityCustodyTierKeychain;

  /// Security screen: custody tier name when the platform has no key vault (desktop).
  ///
  /// In en, this message translates to:
  /// **'No hardware keystore'**
  String get securityCustodyTierNone;

  /// Security screen: custody tier name for an unrecognised (newer-core) tier.
  ///
  /// In en, this message translates to:
  /// **'Unknown'**
  String get securityCustodyTierUnknown;

  /// Security screen: the honest erase line for a hardware-held key tier (no permanence claim, ADR-0571).
  ///
  /// In en, this message translates to:
  /// **'The key that locks this wallet is held in this device\'s secure hardware and is deleted with the wallet.'**
  String get securityCustodyHardwareKey;

  /// Security screen: the honest erase line for a tier whose key is not held by secure hardware (ADR-0571).
  ///
  /// In en, this message translates to:
  /// **'Deleting removes your keys best-effort; a brief forensic recovery window can remain until the device reclaims the storage. For full assurance, also use your device\'s Erase-All-Content.'**
  String get securityCustodyBestEffort;

  /// Security screen: shown when the custody-tier probe fails (e.g. a locked device).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t read the custody status. Pull back and try again.'**
  String get securityCustodyProbeError;

  /// Security screen: the destructive button that opens the delete confirmation.
  ///
  /// In en, this message translates to:
  /// **'Delete wallet'**
  String get securityDeleteWalletButton;

  /// Security screen: subtitle under the delete-wallet button.
  ///
  /// In en, this message translates to:
  /// **'Delete this wallet and its key from this device. Your funds remain on-chain and are restorable from your recovery phrase.'**
  String get securityDeleteWalletSubtitle;

  /// Security screen delete subtitle for a WATCH-ONLY wallet (#397 §3.7 D5): it has no recovery phrase, so the copy must not claim 'restorable from your recovery phrase' — it re-imports from its viewing key instead.
  ///
  /// In en, this message translates to:
  /// **'Delete this wallet and its key from this device. It holds no spending keys, so there is nothing to back up — re-add it anytime with its viewing key.'**
  String get securityDeleteWalletSubtitleWatchOnly;

  /// Delete-wallet confirmation dialog: title.
  ///
  /// In en, this message translates to:
  /// **'Delete this wallet?'**
  String get securityDeleteDialogTitle;

  /// Delete-wallet confirmation dialog: body warning.
  ///
  /// In en, this message translates to:
  /// **'This removes the wallet and its key from this device. Make sure you\'ve backed up your recovery phrase — it is the ONLY way to restore your funds.'**
  String get securityDeleteDialogBody;

  /// Delete-wallet confirmation dialog body for a WATCH-ONLY wallet (#397 §3.7 D5): no recovery phrase, so the copy must not warn about backing one up — it re-imports from its viewing key.
  ///
  /// In en, this message translates to:
  /// **'This removes the wallet and its key from this device. It holds no spending keys, so nothing needs backing up — you can re-add it later with its viewing key.'**
  String get securityDeleteDialogBodyWatchOnly;

  /// Delete-wallet confirmation dialog: the destructive confirm action.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get securityDeleteDialogConfirm;

  /// Delete-wallet confirmation dialog: the cancel action.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get securityDeleteDialogCancel;

  /// Security screen: snackbar shown when a delete fault recovered the wallet (no data lost).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t delete the wallet — your wallet is unchanged. Try again.'**
  String get securityDeleteFailedSnack;

  /// Security screen: snackbar shown when a delete is REFUSED because a sync-server switch is still in flight (S2 M01 — a delete never races a switch). Names the wait: {seconds} is the switch timeout, after which a delete is accepted again.
  ///
  /// In en, this message translates to:
  /// **'Finish the server switch first — it completes or stops within {seconds} seconds. Then try deleting the wallet again.'**
  String securityDeleteRefusedBusySnack(int seconds);

  /// Heading above the parked-sends section: EVERY queued send that has no on-chain transaction yet and so doesn't appear in the activity list — one-time-address (TEX) sends awaiting their window AND plainly-queued offline sends (#331). The copy is deliberately shape-agnostic; keep it honest for both.
  ///
  /// In en, this message translates to:
  /// **'Saved & pending'**
  String get walletParkedTitle;

  /// Sub-heading for the parked-sends section. Deliberately NEUTRAL about whether a row will send on its own (#315): a healthy row sends when ready, a PAUSED row never sends until the user retries — the per-row copy carries that split, so this shared line must be true for both. The amounts are an earmark over the balance (still spendable), never added on top nor deducted. Never invite a re-send.
  ///
  /// In en, this message translates to:
  /// **'These payments haven\'t been sent yet. Their amounts are still part of your balance.'**
  String get walletParkedSubtitle;

  /// The parked-sends sub-heading when AT LEAST ONE row is mid-signature (#401 R2a). The plain sibling asserts the earmark unconditionally — 'their amounts are still part of your balance' — and that is FALSE for a claimed row: past the engine's create the notes are already locally spent, so the amount has left the spendable set. THE OPENER MUST STAY NEUTRAL (#407 R6 reverted #401 R8a's change to it): this heading renders over the WHOLE section whenever ANY row is sending, and that list may also hold PAUSED rows that never send on their own — 'haven't finished sending' told those users progress was underway and invited them to WAIT instead of tapping Send now, the abandoned-funds harm the paused copy exists to prevent. The progress claim belongs in the except-clause, scoped to the rows it is true of.
  ///
  /// In en, this message translates to:
  /// **'These payments haven\'t been sent yet. Their amounts are still part of your balance — except any your wallet is currently sending, which may already be set aside.'**
  String get walletParkedSubtitlePreparing;

  /// Per-row button to cancel (discard) a parked send. The safe counter-affordance — never a re-send.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get walletParkedCancel;

  /// Hint line under a PAUSED parked-send row (#315): the wallet stopped auto-retrying (each retry of a one-time-address send permanently uses up one of a small number of address slots). Must state (a) it will NOT send by itself, (b) funds are safe, (c) the two actions THAT ARE ON SCREEN. #400 R3: the old text said 'Retry it or cancel it' and rendered directly above a button labelled 'Send now' — since FR-23-b that one button both re-arms the row AND signs it, and the separate Retry button no longer exists. Name the visible buttons verbatim (walletParkedSendNow, walletParkedCancel). Never 'will send when ready'.
  ///
  /// In en, this message translates to:
  /// **'Paused — this payment won\'t send on its own. Your funds are safe. Send it now, or cancel it.'**
  String get walletParkedPausedHint;

  /// Snackbar when retry returned false (the row is gone / began sending / changed). Mirrors the cancel-false contract: never invite a re-send; point at the surfaces.
  ///
  /// In en, this message translates to:
  /// **'This payment isn\'t waiting anymore. Check your pending payments and activity.'**
  String get walletParkedRetryStale;

  /// The notFound outcome of the FR-23-b authorize verb (#401 R2b), split off from walletParkedRetryStale. `notFound` means the row is no longer QUEUED — which is EITHER gone (cancelled, completed) OR claimed by the background drain that won the race. In the claimed case the previous copy ('isn't waiting anymore') contradicted the screen itself: the same payment re-renders as PREPARING two lines above, so the user was told a visible row does not exist. The expired outcome keeps walletParkedRetryStale, where 'isn't waiting anymore' is exactly true (the row was deleted). Hedged with 'may' because the surface cannot tell the two apart; points at the two surfaces that can. NEVER invites a re-send — the funds are committed either way and a second send would pay twice.
  ///
  /// In en, this message translates to:
  /// **'This payment is no longer waiting — your wallet may already be sending it. Check Saved & pending and your activity.'**
  String get walletParkedAlreadyInProgress;

  /// #315 slice 2: the line above the 'Reopen sending' button, shown under the parked list when a send is paused. Explains that reopening moves a small amount (your own, returned) — never a fee-only framing that hides the round-trip.
  ///
  /// In en, this message translates to:
  /// **'One-time-address sends are stuck. You can reopen them — it moves a small amount between your own addresses and returns it.'**
  String get walletReclaimExplainer;

  /// The account-level button that runs the #315 reclaim (reopen a bricked one-time-address send window). Shown only when a send is paused.
  ///
  /// In en, this message translates to:
  /// **'Reopen sending'**
  String get walletReclaimButton;

  /// The reclaim button's label while the reclaim is in flight (button disabled + spinner).
  ///
  /// In en, this message translates to:
  /// **'Reopening…'**
  String get walletReclaimInProgress;

  /// Title of the honest-cost disclosure dialog shown before the #315 reclaim runs.
  ///
  /// In en, this message translates to:
  /// **'Reopen one-time-address sending?'**
  String get walletReclaimConfirmTitle;

  /// Body of the reclaim confirm dialog — the honest-cost disclosure. Must state: the amount is your own and returns; the real cost is a couple of network fees; the moved amount is recovered via the existing action once it confirms. Never imply the funds are lost, and never promise the window reopens instantly. #400 R3: the action is named 'Recover now' (walletRecoverNow) — the old 'Recover funds' was never a button label anywhere in the app.
  ///
  /// In en, this message translates to:
  /// **'This moves a small amount between your own addresses to free up one-time-address sending, then returns it. It costs a couple of network fees. Once it confirms, recover the moved amount with Recover now.'**
  String get walletReclaimConfirmBody;

  /// Dismiss the reclaim confirm dialog without acting.
  ///
  /// In en, this message translates to:
  /// **'Not now'**
  String get walletReclaimConfirmCancel;

  /// Confirm the reclaim (proceed to authorize + run it).
  ///
  /// In en, this message translates to:
  /// **'Reopen'**
  String get walletReclaimConfirmAction;

  /// Snackbar after a successful reclaim mint (ReclaimOutcome.Minted). It is INITIATED, not done: the window reopens once the mint confirms (a few minutes) — never claim it is already working. Names the TWO manual follow-ups the user must still do so the flow is not a dead-end. #400 R3: BOTH names were wrong — 'retry the paused send' pointed at a Retry button that FR-23-b replaced with walletParkedSendNow, and 'Recover funds' was never the button's label (it is walletRecoverNow, 'Recover now' — 15 locales had already translated the correct label; EN was the outlier). Every affordance named here must match the visible button label verbatim.
  ///
  /// In en, this message translates to:
  /// **'Reopening started. Once it confirms, send the paused payment, then use Recover now to get the moved amount back.'**
  String get walletReclaimStarted;

  /// Snackbar when the reclaim found no abandoned reservation to reclaim (ReclaimOutcome.NothingToReclaim) — the window is transient / already clear. No money moved.
  ///
  /// In en, this message translates to:
  /// **'Nothing to reopen right now.'**
  String get walletReclaimNothing;

  /// Snackbar when the reclaim mint's broadcast was not acknowledged (ReclaimOutcome.NotBroadcast). Money-safe, but do NOT claim 'nothing was moved': a lost acknowledgement can mean the mint actually landed. Honest about that ambiguity and paces the retry so a rapid re-tap can't double-mint (an extra fee). The principal always returns via the sweep.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t confirm it reached the network. It may still go through — wait a moment before trying again.'**
  String get walletReclaimNotBroadcast;

  /// Snackbar when the reclaim failed because the shielded balance can't fund the small self-mint (InsufficientFunds).
  ///
  /// In en, this message translates to:
  /// **'You need some shielded ZEC to reopen sending.'**
  String get walletReclaimNeedsFunds;

  /// Snackbar when the reclaim threw a typed error (e.g. seed required / busy / closed handle). The funds are untouched and it is retryable.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t reopen sending right now. Your funds are unchanged. Try again.'**
  String get walletReclaimFailed;

  /// Neutral snackbar for a forward-compat reclaim outcome the app does not recognise (ReclaimOutcome.Unknown, only under core/bridge version skew). Never claim success or failure: state it finished and point to the sends + the recover action. Hedge with 'any' since whether an amount moved is unknown. #400 R3: the action is 'Recover now' (walletRecoverNow), matching the visible button.
  ///
  /// In en, this message translates to:
  /// **'Reopen finished. Check your one-time-address sends, and use Recover now to get any moved amount back.'**
  String get walletReclaimUnknown;

  /// Honest error line when the parked-sends read fails. The surface is shown (not silently hidden) because a parked send is money the user is waiting on.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t load your pending payments right now.'**
  String get walletParkedError;

  /// Inline retry button under the parked-sends read-error line: re-pulls the pending-payments list in place (the home has no pull-to-refresh). SECTION-level, not row-level: the per-row affordance is walletParkedSendNow, which signs one saved payment (#401 R7c — this line named walletParkedRetry, retired by #400). Same 'try again' wording as the other read-error retries (walletReceiveRetry, walletShieldRetry).
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get walletParkedErrorRetry;

  /// The parked-sends read-error retry button's label WHILE the re-pull is in flight (#407 R5). Load-bearing for a11y, not decoration: the surrounding Semantics(liveRegion:) is flagged on THIS label, so the label CHANGING is what re-fires the announcement when the identical error re-lands. The pre-#407 shape flagged an outer container whose SemanticsData was byte-identical across the flip — measured, it never re-announced, while the code comment claimed it did. Same shape as walletParkedSendNowInProgress. Keep it SHORT — it replaces 'Try again' inside a button beside a spinner.
  ///
  /// In en, this message translates to:
  /// **'Trying…'**
  String get walletParkedErrorRetryInProgress;

  /// Title of the confirm dialog before discarding a parked send.
  ///
  /// In en, this message translates to:
  /// **'Cancel this pending payment?'**
  String get walletParkedCancelConfirmTitle;

  /// Body of the cancel-parked confirm dialog. Reassures that a queued (not-yet-sent) send moves no funds, while flagging the action is irreversible.
  ///
  /// In en, this message translates to:
  /// **'This discards the saved payment. It hasn\'t been sent, so nothing leaves your wallet — but this can\'t be undone.'**
  String get walletParkedCancelConfirmBody;

  /// Cancel-parked confirm dialog: keep the pending payment (dismiss the dialog).
  ///
  /// In en, this message translates to:
  /// **'Keep it'**
  String get walletParkedCancelConfirmKeep;

  /// Cancel-parked confirm dialog: the irreversible confirm that discards the queued send.
  ///
  /// In en, this message translates to:
  /// **'Discard payment'**
  String get walletParkedCancelConfirmDiscard;

  /// Snackbar after a parked send was successfully cancelled.
  ///
  /// In en, this message translates to:
  /// **'Pending payment cancelled.'**
  String get walletParkedCancelDone;

  /// Snackbar when cancel returned false. The SDK can't yet distinguish 'already gone' from 'began sending', so the copy is hedged ('may') — honest in BOTH cases. NEVER present as cancelled; never invite a re-send — direct the user to the activity list (double-pay safety).
  ///
  /// In en, this message translates to:
  /// **'This payment may already be on its way — check your activity.'**
  String get walletParkedCancelAlreadySending;

  /// Snackbar when the cancel call threw (e.g. wallet busy / closed). The queued send is untouched.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t cancel right now. Your payment is unchanged. Try again.'**
  String get walletParkedCancelFailed;

  /// Button to recover funds sitting on one-time (ephemeral) transparent addresses into the shielded balance. Shown only when a successful read reports recoverable funds.
  ///
  /// In en, this message translates to:
  /// **'Recover now'**
  String get walletRecoverNow;

  /// Title of the confirm dialog before running the one-time-address recovery (sweep).
  ///
  /// In en, this message translates to:
  /// **'Recover to your shielded balance?'**
  String get walletRecoverConfirmTitle;

  /// Body of the recover confirm dialog. The action is privacy-positive (into shielded) and idempotent (re-runnable).
  ///
  /// In en, this message translates to:
  /// **'This checks your one-time addresses and moves anything found into your private shielded balance. It\'s safe to run again any time.'**
  String get walletRecoverConfirmBody;

  /// Recover confirm dialog: dismiss without recovering.
  ///
  /// In en, this message translates to:
  /// **'Not now'**
  String get walletRecoverConfirmCancel;

  /// Recover confirm dialog: the confirm action that runs the recovery.
  ///
  /// In en, this message translates to:
  /// **'Recover'**
  String get walletRecoverConfirmAction;

  /// The recover button's DISABLED in-progress label while a sweep signs + broadcasts per address (it can take a moment on a slow link). Doubles as the single-flight in-progress cue, so a re-tap can't launch a second concurrent sweep.
  ///
  /// In en, this message translates to:
  /// **'Recovering…'**
  String get walletRecoverInProgress;

  /// Snackbar after recovery accepted ALL funds cleanly. The amount is provisional (accepted, not yet confirmed) so the copy says 'recovering'. amount is pre-formatted.
  ///
  /// In en, this message translates to:
  /// **'Recovering {amount} to your shielded balance.'**
  String walletRecoverDone(String amount);

  /// Snackbar after recovery accepted SOME funds but had per-address faults or hit the per-run cap (swept > 0 AND (failed > 0 OR truncated > 0)). Reports the provisional recovered amount AND honestly flags that work remains — never hides the remainder. amount is pre-formatted.
  ///
  /// In en, this message translates to:
  /// **'Recovering {amount} — some funds still need another try.'**
  String walletRecoverDonePartial(String amount);

  /// Snackbar when recovery had per-address faults or hit the per-run cap. The funds stay on-chain and re-runnable — never a loss.
  ///
  /// In en, this message translates to:
  /// **'Some funds need another try — run recovery again.'**
  String get walletRecoverRetry;

  /// Sweep outcome when the per-invocation cap left addresses UNCHECKED and nothing was swept or failed — honest 'incomplete check', never a claim that funds exist (distinct from walletRecoverRetry, which is for per-address faults).
  ///
  /// In en, this message translates to:
  /// **'Not every one-time address was checked yet — run it again to check the rest.'**
  String get walletRecoverTruncated;

  /// Snackbar when recovery found nothing sweepable (e.g. already recovered on a prior run).
  ///
  /// In en, this message translates to:
  /// **'Nothing to recover right now.'**
  String get walletRecoverNothing;

  /// Snackbar when the recovery call threw (e.g. seed required / closed handle). The funds are untouched and re-runnable.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t recover right now. Your funds are unchanged. Try again.'**
  String get walletRecoverFailed;

  /// One saved & pending (parked) send row: the committed amount plus the locale-formatted save time (the discriminator between two same-amount rows).
  ///
  /// In en, this message translates to:
  /// **'{amount} saved & pending · {time}'**
  String walletParkedRowTimed(String amount, String time);

  /// Screen-reader label for a parked row’s Cancel button — binds the action to its amount AND save time so same-amount rows never read identically.
  ///
  /// In en, this message translates to:
  /// **'Cancel the {amount} payment saved {time}'**
  String walletParkedCancelSemanticTimed(String amount, String time);

  /// One PAUSED parked-send row (#315): the committed amount plus the locale-formatted save time. The 'paused' word is the row-level honesty split from the healthy 'saved & pending' — a paused payment never sends on its own (walletParkedPausedHint carries the explanation).
  ///
  /// In en, this message translates to:
  /// **'{amount} paused · {time}'**
  String walletParkedRowPausedTimed(String amount, String time);

  /// One MID-SIGNATURE parked-send row (#400 R2, ParkedSend.sending): the wallet claimed this payment and is building the transaction — or was, until the app was killed during that step (an OOM while proving is the common mobile case). Before this row existed such a payment was on NO surface at all, which is the double-pay shape the whole section exists to prevent. Say PREPARING: nothing failed (never 'failed') and nothing was broadcast (never 'sent'/'on its way'). walletParkedPreparingHint carries the rest.
  ///
  /// In en, this message translates to:
  /// **'{amount} preparing to send · {time}'**
  String walletParkedRowPreparingTimed(String amount, String time);

  /// Hint line under a MID-SIGNATURE parked row (#400 R2) when a sync pass WILL run (`walletSyncPassesRunProvider` — #407 R10d corrected this line, which still said 'the host's sync policy is ON' after #401 R5 / #403 R4 re-keyed the site onto the drive-aware SSOT; a FAILED sync start reads policy-ON and runs no passes). Must state (a) work is in progress, (b) the amount may ALREADY be out of the spendable balance — unlike every other row in this section, whose amount is an earmark over the balance, this row can be past the point where the wallet committed the transaction locally and marked its notes spent, so the section's 'their amounts are still part of your balance' is not true of it — (c) funds are safe, (d) it self-recovers: the wallet re-queues an unfinished claim on its next completed sync pass, so the user has nothing to do and must NOT re-enter the payment. Deliberately unspecific about WHEN. Normally NO action is offered on this row (every verb that could act on it requires a still-queued row), so the copy must not name one — with ONE exception (#403 R9e, correcting a claim this file stated absolutely): while the user's OWN authorization bracket is what claimed the row, the section keeps rendering its spinnered Send now, because deleting the cue mid-proof is the dead-app shape the cue exists to prevent. When no pass will run — host policy off OR a failed start — use walletParkedPreparingHintSyncPaused.
  ///
  /// In en, this message translates to:
  /// **'Your wallet is getting this payment ready — its amount may already be set aside. Your funds are safe. If it doesn\'t finish, it returns to the list on its own.'**
  String get walletParkedPreparingHint;

  /// The MID-SIGNATURE row's hint when NO background sync pass will run (#400 R2, re-keyed by #401 R5). Its sibling promises the row 'returns to the list on its own' — true only where sync passes happen, because that self-recovery IS a sync pass. Without passes the row sits there indefinitely and a user told to wait for a self-heal that cannot come re-enters the payment: the double pay this whole section exists to prevent. Renamed from ...SyncOff because it is keyed on the sync DRIVE, not the host policy: a FAILED sync start has the same consequence and 'turn syncing back on' would be the wrong instruction there — so the promise is replaced by a cause-agnostic condition and the screen's own notice carries the remedy. Keep the same first two clauses as the sibling (work in progress; the amount may already be set aside; funds safe). Never phrase it as a failure — nothing failed.
  ///
  /// In en, this message translates to:
  /// **'Your wallet is getting this payment ready — its amount may already be set aside. Your funds are safe, but it can only finish once your wallet is syncing again.'**
  String get walletParkedPreparingHintSyncPaused;

  /// Per-row button on a parked send (FR-23-b, #361): sign and send the already-saved payment now, instead of waiting for the wallet's next background pass. A host that authorizes each spend individually prompts here. ONE LABEL FOR BOTH ROW STATES — healthy AND paused (#361 M4): on a paused row it re-arms the retry budget and signs, so it does strictly MORE than on a healthy row and 'Retry' understated it. (The description previously said 'HEALTHY (not paused)' and 'a paused row shows walletParkedRetry instead'; that Retry key is retired — #400 R9 corrects the drift.) Not shown on a row that is already mid-signature (ParkedSend.sending).
  ///
  /// In en, this message translates to:
  /// **'Send now'**
  String get walletParkedSendNow;

  /// Screen-reader label for the Send now button WHILE its spend bracket is open (#401 R3c). The visible label shortens to 'Sending…', and the in-flight semantic label used to fall back to that bare string — which re-introduces exactly the collision the timed labels exist to prevent: two parked rows with the same amount become two identical 'Sending…' nodes, and the one the user actually authorized is no longer identifiable. Keep the amount + save-time binding through the in-flight state. Present tense, because the signature is happening now.
  ///
  /// In en, this message translates to:
  /// **'Sending the {amount} payment saved {time}'**
  String walletParkedSendNowInProgressSemanticTimed(String amount, String time);

  /// Screen-reader label for a row’s Send now button — binds the action to its amount AND save time so same-amount rows never read identically (mirrors the Cancel semantic label). Used on healthy AND paused rows alike (see walletParkedSendNow).
  ///
  /// In en, this message translates to:
  /// **'Send the {amount} payment saved {time} now'**
  String walletParkedSendNowSemanticTimed(String amount, String time);

  /// The Send now button's label while the authorization is in flight (#400 R4): the button is disabled and shows a spinner. Load-bearing, not decoration — at held custody there is no host prompt to look at, and the call runs an unbounded proving step (tens of seconds on a fragmented wallet), so without a cue the user sees a dead button and taps elsewhere or re-enters the payment. Mirrors walletReclaimInProgress.
  ///
  /// In en, this message translates to:
  /// **'Sending…'**
  String get walletParkedSendNowInProgress;

  /// Snackbar after a parked send was signed in the authorization bracket (FR-23-b): it is now a real transaction on its way. It leaves the pending-payments list and appears in activity. Present progressive on purpose — reaching the network is still in progress. Used when a sync pass WILL run (`walletSyncPassesRunProvider`); otherwise — host policy off OR a failed sync start — use walletParkedAuthorizeSentSyncPaused (#407 R10d: this line said 'the host's sync policy is ON', which stopped being the gate at #401 R5).
  ///
  /// In en, this message translates to:
  /// **'Sending your payment now.'**
  String get walletParkedAuthorizeSent;

  /// The signed-outcome snackbar when NO background sync pass will run (#400 R1, the reliability HIGH; re-keyed by #401 R5). The transaction is signed and the wallet is broadcasting it — but if the broadcast never reaches the network, the wallet's durable re-send only runs on a completed sync pass: without passes the payment would sit signed and unsent with no correction. So the plain 'Sending your payment now.' is not the whole truth here. Renamed from ...SyncOff and made cause-agnostic because it is keyed on the sync DRIVE: a FAILED sync start strands the residual exactly as the host's sync-off policy does, and 'turn syncing back on' is the wrong instruction for it — the screen's own sync notice carries the remedy either way. Never phrase it as a failure (nothing failed) and never invite re-entering the payment — the funds are already committed and a second send would pay twice.
  ///
  /// In en, this message translates to:
  /// **'Sending your payment now. If it doesn\'t go through, your wallet can only finish the payment once it\'s syncing again.'**
  String get walletParkedAuthorizeSentSyncPaused;

  /// Snackbar when the authorization signed NOTHING and the payment stays saved (not enough spendable balance at this moment, a full one-time-address window, or the wallet's own background pass claimed the row first). MONEY-CRITICAL COPY RULE: this is NOT a failure — never translate it as one. A user who believes the payment failed re-enters it and BOTH send: a double pay. Deliberately promises no retry timing (background retries ride sync passes, which a host may have turned off). Only for a row that was NOT paused — see walletParkedAuthorizeRearmed.
  ///
  /// In en, this message translates to:
  /// **'Not ready to send yet. Your payment is saved and unchanged.'**
  String get walletParkedAuthorizeStillWaiting;

  /// The same not-ready outcome as walletParkedAuthorizeStillWaiting, but for a row that was PAUSED when the user tapped (#400 R5). The authorization re-arms a paused row's retry budget before signing, so even when nothing is signed the row DID change: it is no longer paused, its icon and label change, the paused hint disappears and the reopen-sending prompt may vanish with it. Telling that user their payment is 'unchanged' while the row visibly re-shapes reads as a bug and is simply untrue — so state the re-arm, which is what actually happened. THE SIXTH QUEUE-DRAIN STRING (#403 R3): the previous ending, 'it will be tried again', is the same automatic-drain promise #401 R1(a) swept from five siblings and missed here. A re-armed row is a QUEUED intent, so a background retry needs a sync pass AND a signing credential, and at host custody the unattended pass holds neither. It is the ONLY member of that family a user reaches BY TAPPING THE MONEY BUTTON, and it lands on top of a section whose own note may say these do not send on their own — a promise and its negation four lines apart about one payment. Same rule as walletSendQueuedBody / walletSendQueueHint: promise NO schedule, name the two real actions. Both are on screen by construction here (the re-arm leaves the row Queued, so it renders Send now and Cancel). 'Send now' must match walletParkedSendNow verbatim.
  ///
  /// In en, this message translates to:
  /// **'Not ready to send yet. Your payment is saved and no longer paused — try Send now again later, or cancel it.'**
  String get walletParkedAuthorizeRearmed;

  /// Snackbar when the authorize call itself threw (busy / closed wallet / the host's signing credential was unavailable). The saved payment is untouched — same reassurance shape as the other 'unchanged, try again' faults.
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t send it right now. The payment is unchanged. Try again.'**
  String get walletParkedAuthorizeFailed;

  /// Overflow-menu entry opening the transparent-funds policy sheet (expert gate + auto-shield).
  ///
  /// In en, this message translates to:
  /// **'Public funds…'**
  String get walletTransparentFundsMenuItem;

  /// Title of the transparent-funds policy sheet.
  ///
  /// In en, this message translates to:
  /// **'Public funds'**
  String get walletTransparentFundsTitle;

  /// Plain-factual intro of the transparent-funds sheet: the visibility facts ONLY. The auto-shield claim is the separate CONDITIONAL sentence (walletTransparentFundsAutoOn/Off) so the sheet never states automation that is switched off.
  ///
  /// In en, this message translates to:
  /// **'Public funds are publicly visible on the blockchain — the amount, the addresses, and the history of the coins.'**
  String get walletTransparentFundsIntro;

  /// The expert-gate switch label (default OFF). Turning it on reveals the auto-shield switch and the move-to-transparent entry in the sheet.
  ///
  /// In en, this message translates to:
  /// **'Advanced: public funds'**
  String get walletExpertToggleLabel;

  /// Subtitle under the expert-gate switch. Plain-factual.
  ///
  /// In en, this message translates to:
  /// **'Show expert controls for holding public funds and turning automatic shielding off.'**
  String get walletExpertToggleDescription;

  /// The expert-gate description when the HOST declared auto-shield unsupported (#383 R2): walletExpertToggleDescription with ONLY the 'turning automatic shielding off' clause removed — that switch never renders there, so advertising it would promise a control that doesn't appear. Keep the shared clause identical to the sibling key so the two never drift.
  ///
  /// In en, this message translates to:
  /// **'Show expert controls for holding public funds.'**
  String get walletExpertToggleDescriptionNoAutoShield;

  /// The auto-shield switch label (default ON; only changeable behind the expert gate).
  ///
  /// In en, this message translates to:
  /// **'Shield automatically'**
  String get walletAutoShieldToggleLabel;

  /// Subtitle under the auto-shield switch. {minZec} is the already-formatted minimum amount (e.g. "0.001").
  ///
  /// In en, this message translates to:
  /// **'When your public balance reaches {minZec} ZEC, it is moved into your shielded balance automatically. With this off, public funds stay publicly visible until you shield them yourself.'**
  String walletAutoShieldToggleDescription(String minZec);

  /// Snackbar shown when persisting a settings toggle failed; the switch stays at its saved value.
  ///
  /// In en, this message translates to:
  /// **'Couldn’t save the setting. Try again.'**
  String get walletSettingsSaveFailed;

  /// Balance-card cue under the transparent line when the auto-shield loop failed or was denied; the manual Shield button sits right below it.
  ///
  /// In en, this message translates to:
  /// **'Automatic shielding didn’t complete — these funds are still publicly visible. You can shield them now.'**
  String get walletAutoShieldIncomplete;

  /// Send-review visibility statement for a fully shielded payment.
  ///
  /// In en, this message translates to:
  /// **'Shielded payment — the amount and recipient stay private on-chain.'**
  String get walletSendPrivacyShielded;

  /// Send-review visibility statement when any output is transparent; compact restatement of the de-shield warning.
  ///
  /// In en, this message translates to:
  /// **'Public payment — the amount and addresses are visible on the blockchain.'**
  String get walletSendPrivacyTransparent;

  /// Screen-reader words for the activity row’s transparency badge (the visual is a small globe icon), and the value of the tx-detail Visibility row. Direction-neutral: covers sent AND received transparent legs.
  ///
  /// In en, this message translates to:
  /// **'Publicly visible on-chain'**
  String get walletActivityPublicBadge;

  /// Body of the shield sheet’s failure terminal when the wallet session ended mid-sheet; mirrors walletMoveWalletEnded.
  ///
  /// In en, this message translates to:
  /// **'The wallet session ended. Close and reopen to try again.'**
  String get walletShieldWalletEnded;

  /// The sheet’s conditional automation sentence while auto-shield is ON. {minZec} is the already-formatted minimum (e.g. "0.001").
  ///
  /// In en, this message translates to:
  /// **'New public funds are shielded into your private balance automatically once they reach {minZec} ZEC.'**
  String walletTransparentFundsAutoOn(String minZec);

  /// The sheet’s conditional automation sentence while auto-shield is OFF (shown even when the expert gate hides the switch, so a persisted OFF is never invisible).
  ///
  /// In en, this message translates to:
  /// **'Automatic shielding is off — public funds stay publicly visible until you shield them.'**
  String get walletTransparentFundsAutoOff;

  /// Orange note on the move-to-transparent confirm when auto-shield is ON — without it the loop silently reverts the deliberate unshield and burns a second fee.
  ///
  /// In en, this message translates to:
  /// **'Automatic shielding is on: after these funds arrive, they will be shielded back automatically (for another fee). To keep them public, first turn off automatic shielding under Public funds.'**
  String get walletMoveAutoShieldNote;

  /// Orange note on the move-to-transparent review when the move leaves the PUBLIC balance (what is already public at the shield source plus this move) under the core's shield floor (stage S14, maintainer copy): no Shield, manual or automatic, can take it until more arrives. Replaces the auto-shield note in that case.
  ///
  /// In en, this message translates to:
  /// **'After this move your public balance will be {amount} ZEC — under the {floor} ZEC needed to shield it back. It stays public until more arrives.'**
  String walletMoveBelowFloorNote(String amount, String floor);

  /// Variant of walletMoveOwnAddressNote shown when the move leaves the public balance under the shield floor (stage S14): it drops 'You can shield these funds again later', which would be false there.
  ///
  /// In en, this message translates to:
  /// **'You\'re moving to your own public address. This move stays on the public record permanently.'**
  String get walletMoveOwnAddressNoteStaysPublic;

  /// Label of the tx-detail row stating the transparency fact (value: walletActivityPublicBadge). Rendered only for transactions with a publicly visible output.
  ///
  /// In en, this message translates to:
  /// **'Visibility'**
  String get walletTxDetailVisibility;

  /// The sheet’s automation sentence while the host authorizer has declined automatic shielding for this session (the switch is ON but the loop is paused). Plain-factual; the manual Shield button on the balance card remains the recovery.
  ///
  /// In en, this message translates to:
  /// **'Automatic shielding is paused for this session — it wasn’t approved. You can still shield manually.'**
  String get walletTransparentFundsAutoDenied;

  /// #390 overflow-menu entry (adjacent to Rescan) opening the deep-scan sheet. CHECK verb, never ‘recover’ — the scan can’t know funds exist. ‘addresses’ not ‘refunds’ — it recovers swap deliveries too.
  ///
  /// In en, this message translates to:
  /// **'Check older swap addresses…'**
  String get walletDeepScanMenuItem;

  /// Sub-label under the disabled Rescan / Check-older-swap-addresses menu entries: no sync pass will run, so neither op can ever finish (#405 — widened from the host policy alone to the SSOT walletSyncPassesRunProvider, which also covers a FAILED sync start). CAUSE-AGNOSTIC ON PURPOSE: 'turn syncing on in settings' is the right instruction for a host-off policy and the WRONG one for a failed start, so this states the CONDITION only; the sync badge and the start-failed notice on the same screen own the cause and its remedy. Needed because the explaining badge is occluded behind the open menu and a screen reader would otherwise hear only 'dimmed'.
  ///
  /// In en, this message translates to:
  /// **'Syncing isn\'t running right now.'**
  String get walletMenuSyncNotRunningHint;

  /// #390 deep-scan sheet title.
  ///
  /// In en, this message translates to:
  /// **'Check older swap addresses'**
  String get walletDeepScanTitle;

  /// #390 deep-scan sheet explanation. Honest: it CHECKS (does not itself find); results appear via the normal balance as sync proceeds.
  ///
  /// In en, this message translates to:
  /// **'If you restored this wallet and it once used swaps a lot, money from its oldest swaps can take an extra step to find. This checks for it — anything found appears in your balance as your wallet syncs.'**
  String get walletDeepScanBody;

  /// #390 sheet coverage line when a range is fully checked (covered > 0, pending == 0). NO count — the address index is not a swap tally, so any number would be false (B2). Offers going deeper.
  ///
  /// In en, this message translates to:
  /// **'Your older swap addresses are checked up to here. If money from an old swap is still missing, check even deeper.'**
  String get walletDeepScanCoverage;

  /// #390 sheet coverage line while the current range is still registering + polling (pending > 0). Sync-NEUTRAL — no ‘while your wallet syncs’ claim, which is false when offline/stalled (B3).
  ///
  /// In en, this message translates to:
  /// **'Still checking the current range — anything found appears in your balance. This can take a little while.'**
  String get walletDeepScanCoveragePending;

  /// #390 sheet coverage line fallback while the read is loading or unavailable (never a scary error over a recovery sheet).
  ///
  /// In en, this message translates to:
  /// **'Checks for money from your wallet’s oldest swaps.'**
  String get walletDeepScanCoverageUnknown;

  /// #390 sheet primary action — start (or continue) the deep scan.
  ///
  /// In en, this message translates to:
  /// **'Check older addresses'**
  String get walletDeepScanCheckButton;

  /// #390 sheet primary action once a range is already fully checked (covered > 0 and nothing pending) — each run goes deeper.
  ///
  /// In en, this message translates to:
  /// **'Check even older addresses'**
  String get walletDeepScanCheckDeeperButton;

  /// #390 in-flight label — on the sheet button while a scan runs, and on the disabled overflow-menu entry (says why it is disabled).
  ///
  /// In en, this message translates to:
  /// **'Checking…'**
  String get walletDeepScanChecking;

  /// #390 sheet dismiss button (the scan continues in the background once started).
  ///
  /// In en, this message translates to:
  /// **'Close'**
  String get walletDeepScanClose;

  /// #390 sheet privacy hint shown when Tor was requested but fell back to clearnet (or is unavailable) — the scan is elective, so recommend deferral. Never blocks the action.
  ///
  /// In en, this message translates to:
  /// **'You’re not connected over Tor right now. For more privacy, consider waiting until Tor is active before checking.'**
  String get walletDeepScanTorHint;

  /// #390 sheet note while a rescan is running/rebuilding — the deep scan and a rescan both re-poll the transparent set, so they are mutually exclusive.
  ///
  /// In en, this message translates to:
  /// **'You can check older swap addresses once the rescan finishes.'**
  String get walletDeepScanRescanBusy;

  /// #390 inline confirmation after an accepted scan. Honest: it does NOT claim ‘found X’ — money surfaces via the normal balance. Sync-NEUTRAL (no ‘as your wallet syncs’, false when offline; B3).
  ///
  /// In en, this message translates to:
  /// **'Checking older swap addresses — anything found will appear in your balance.'**
  String get walletDeepScanRan;

  /// #390 snackbar when the scan could not start (a transient fault). Reassures nothing changed.
  ///
  /// In en, this message translates to:
  /// **'Couldn’t start the check. Nothing changed — try again.'**
  String get walletDeepScanFailed;

  /// #390 rel-H1 inline message when the check exceeds the FFI wedge timeout. The widen MAY have committed (a durable local write), so this must NOT claim 'nothing changed' — it's neutral and says money appears in the balance if it did.
  ///
  /// In en, this message translates to:
  /// **'This is taking longer than usual. If your older swap addresses were checked, anything found appears in your balance — check back shortly.'**
  String get walletDeepScanSlow;

  /// #390 snackbar for the swapDisabled refusal (a host kill switch stopped the polling the check depends on).
  ///
  /// In en, this message translates to:
  /// **'Swap is turned off right now, so this can’t run. Try again when swap is available.'**
  String get walletDeepScanRefusedDisabled;

  /// #390 inline message for the checkOutstanding refusal (a prior check is still registering/polling — at most one per ~48h settlement window). B4: names the real horizon (‘up to a couple of days’) instead of ‘a little while’.
  ///
  /// In en, this message translates to:
  /// **'Still checking the last range — this can take up to a couple of days, but usually much less. It finishes on its own; check again later.'**
  String get walletDeepScanRefusedOutstanding;

  /// #390 B5 — softer privacy nudge shown when the connection/transport state hasn’t loaded yet (Tor status unknown), so the privacy hint is never silently dropped. Distinct from walletDeepScanTorHint (a CONFIRMED Tor fallback).
  ///
  /// In en, this message translates to:
  /// **'We can’t confirm your connection privacy yet. For more privacy, check once Tor is active.'**
  String get walletDeepScanTorUnknownHint;

  /// #390 C1 — a dismissible banner on the wallet home while a deep scan the user ran is still surfacing money (pending > 0). Sync-NEUTRAL wording (‘as it’s found’, not ‘as your wallet syncs’) so it stays honest while offline; survives closing the sheet.
  ///
  /// In en, this message translates to:
  /// **'Still checking your older swap addresses — anything found appears in your balance.'**
  String get walletDeepScanBannerChecking;

  /// #390 cross-pointer on the Rescan sheet: a rescan re-scans compact blocks (no transparent outputs) and keeps the restore ceiling, so it cannot surface old-swap funds; point the affected user at the deep scan.
  ///
  /// In en, this message translates to:
  /// **'Looking for money from an old swap? A rescan won’t find that — use “Check older swap addresses” instead.'**
  String get walletRescanSwapPointer;

  /// #390 one-time post-restore note title, shown once after the first restore’s catch-up finishes.
  ///
  /// In en, this message translates to:
  /// **'Restored a wallet that used swaps?'**
  String get walletDeepScanRestoreNoteTitle;

  /// #390 one-time post-restore note body — educates that the situation can happen and how to resolve it, without alarming (‘most wallets need nothing’).
  ///
  /// In en, this message translates to:
  /// **'If this wallet had a very long swap history, money from its oldest swaps can take an extra step to find. Most wallets need nothing.'**
  String get walletDeepScanRestoreNoteBody;

  /// #390 post-restore note action — open the deep-scan sheet.
  ///
  /// In en, this message translates to:
  /// **'Check now'**
  String get walletDeepScanRestoreNoteCheck;

  /// #390 post-restore note dismiss action (the note never shows again once displayed).
  ///
  /// In en, this message translates to:
  /// **'Dismiss'**
  String get walletDeepScanRestoreNoteDismiss;

  /// Transport chip: wallet traffic rides the dialer the host app registered (FR-29), whose path hides the device's address and honours per-purpose isolation. transport = the HOST'S OWN name for its transport, verbatim (ADR-0547: the wallet has no list of transport kinds), or walletTorHostOtherTransport when the SDK has no name to show. Protected tone.
  ///
  /// In en, this message translates to:
  /// **'Via your app\'s private path ({transport})'**
  String walletTorHostPath(String transport);

  /// Transport chip: as walletTorHostPath, but the host declared isolation unsupported (or did not declare it) — the wallet's connections can be linked to each other at the proxy — or the host did not declare whether the path hides the device's address (exposure unknown). Caution tone; the state never promises what the host did not declare (ADR-0545, ADR-0547). transport = the host's own name, verbatim.
  ///
  /// In en, this message translates to:
  /// **'Via your app\'s private path ({transport}); connections can be linked by the proxy'**
  String walletTorHostPathLinkable(String transport);

  /// The transport name substituted into walletTorHostPath / walletTorHostPathLinkable when the SDK has NO host name to show (the SDK's own unattributed rendering — an empty name never comes from a host, the crossing refuses it): the wallet never names a transport the host did not name (ADR-0547). Lower-case fragment that reads inside the parentheses.
  ///
  /// In en, this message translates to:
  /// **'a private path'**
  String get walletTorHostOtherTransport;

  /// Transport chip: the host app's registered dialer declared its path EXPOSED (ADR-0547 exposure = exposed: a plain direct connection, or a forward proxy that passes the client address) — not private, the server sees the IP, whatever the host named it and whatever the isolation says. Neutral tone, paired with walletTransportExplainDirect.
  ///
  /// In en, this message translates to:
  /// **'Not private (your app\'s direct connection)'**
  String get walletTorHostDirect;

  /// Banner for SyncServerFallback.choiceRefusedByTransport (FR-29 E12): the remembered CUSTOM server is an unencrypted http:// address, which only the SDK's own direct connection may carry — under the app's private path the default is in use, the choice is kept. host = the server now in use.
  ///
  /// In en, this message translates to:
  /// **'Your saved server uses an unencrypted address, which your app\'s private path cannot carry — using {host}.'**
  String walletSyncServerFallbackRefusedByTransport(String host);

  /// Screen-reader name of the (i) button after a label; opens the explanation that used to sit on screen (S13). label = the label the button follows, verbatim.
  ///
  /// In en, this message translates to:
  /// **'More about {label}'**
  String walletInfoButtonLabel(String label);

  /// Send form: fills the recipient from the clipboard (S13, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Paste'**
  String get walletSendPaste;

  /// Send form: opens the camera to scan an address or a zcash: payment request (S13, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Scan QR code'**
  String get walletSendScanQr;

  /// Send review: the amount the recipient receives, fee and change excluded (S13, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Recipient gets'**
  String get walletSendRecipientGetsLabel;

  /// Swap deposit screen: copies the exact amount to send (S13, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Copy amount'**
  String get walletSwapDepositCopyAmount;

  /// Snackbar after Copy amount (S13, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Amount copied'**
  String get walletSwapDepositAmountCopied;

  /// Scanner, camera refused: opens the app's system settings through the host (S13, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Open settings'**
  String get walletScanOpenSettings;

  /// Scanner, camera refused: the host's settings opener failed (maintainer).
  ///
  /// In en, this message translates to:
  /// **'Couldn\'t open settings.'**
  String get walletScanOpenSettingsFailed;

  /// Dialog title when the user presses Back while a payment is being sent (S13 H2, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Still sending'**
  String get walletSendLeaveTitle;

  /// Dialog body for walletSendLeaveTitle (S13 H2, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Your payment keeps going if you leave. You\'ll see how it ended in your activity.'**
  String get walletSendLeaveBody;

  /// Dialog action: stay on the send screen (S13 H2, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Stay'**
  String get walletSendLeaveStay;

  /// Dialog action: leave the send screen; sending continues (S13 H2, maintainer).
  ///
  /// In en, this message translates to:
  /// **'Leave'**
  String get walletSendLeaveConfirm;

  /// Shield / Move sheet: dialog body when Back is pressed while it submits; title walletSendLeaveTitle, actions walletSendLeaveStay / walletSendLeaveConfirm (maintainer).
  ///
  /// In en, this message translates to:
  /// **'This keeps going if you leave. You\'ll see how it ended in your activity.'**
  String get walletSheetLeaveBody;

  /// Screen-reader name of a spinner that stands alone, with no label beside it (the wallet's first load, the activity list's first load, the swap token list). S13 §2, maintainer.
  ///
  /// In en, this message translates to:
  /// **'Loading'**
  String get walletLoadingLabel;

  /// Title of the send screen's terminal when the payment ran but its answer was lost (S7 U1, SendOutcomeUnknown): the host's own authorization code threw or declined after the spend ran. Neither 'sent' nor 'nothing was sent' is true. Bodies: walletSendUnknownBody / walletSendUnknownQueuedBody. No retry is offered.
  ///
  /// In en, this message translates to:
  /// **'Check before sending again'**
  String get walletSendUnknownTitle;

  /// Body for walletSendUnknownTitle on the SEND path (S7 U1). Must never say whether money moved: the transaction may have been signed and broadcast. Points at Activity, where the payment's real state is shown.
  ///
  /// In en, this message translates to:
  /// **'We couldn\'t confirm this payment. Check Activity before sending it again.'**
  String get walletSendUnknownBody;

  /// Body for walletSendUnknownTitle on the offline QUEUE path (S7 U1): the payment may have been durably queued to send later. Points at the wallet's pending (parked) payments list.
  ///
  /// In en, this message translates to:
  /// **'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.'**
  String get walletSendUnknownQueuedBody;

  /// Title of the shield sheet's terminal when the shield ran but its answer was lost (ShieldOutcomeUnknown): the transaction may already be saved. Neither 'shielded' nor 'nothing happened' is true. Body: walletShieldUnknownBody. Only Close is offered, never a retry.
  ///
  /// In en, this message translates to:
  /// **'Check before shielding again'**
  String get walletShieldUnknownTitle;

  /// Body for walletShieldUnknownTitle. Must never say whether funds moved. Points at Activity, where the shield's real state is shown.
  ///
  /// In en, this message translates to:
  /// **'We couldn\'t confirm this shield. Check Activity before trying again.'**
  String get walletShieldUnknownBody;

  /// Title of the move-to-transparent sheet's terminal when the move ran but its answer was lost (MoveOutcomeUnknown): the transaction may already be saved. Neither 'moved' nor 'nothing happened' is true. Body: walletMoveUnknownBody. Only Close is offered, never a retry.
  ///
  /// In en, this message translates to:
  /// **'Check before moving again'**
  String get walletMoveUnknownTitle;

  /// Body for walletMoveUnknownTitle. Must never say whether funds moved. Points at Activity, where the move's real state is shown.
  ///
  /// In en, this message translates to:
  /// **'We couldn\'t confirm this move. Check Activity before trying again.'**
  String get walletMoveUnknownBody;

  /// Detail-sheet explanation for a wallet-created row that reads Expired while the wallet still owes the payment (TxSummary.delivery == retryPending, S7 C1): the old transaction expired unmined and the wallet will re-send it once the expiry is safely buried. The row label stays 'Retrying'. The 'don't send it again yourself' clause is the point: a manual re-send pays twice.
  ///
  /// In en, this message translates to:
  /// **'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.'**
  String get walletTxExplainRetryingExpired;
}

class _WalletLocalizationsDelegate
    extends LocalizationsDelegate<WalletLocalizations> {
  const _WalletLocalizationsDelegate();

  @override
  Future<WalletLocalizations> load(Locale locale) {
    return SynchronousFuture<WalletLocalizations>(
      lookupWalletLocalizations(locale),
    );
  }

  @override
  bool isSupported(Locale locale) => <String>[
    'ar',
    'de',
    'en',
    'es',
    'fi',
    'fr',
    'he',
    'it',
    'ja',
    'nb',
    'nl',
    'pl',
    'pt',
    'ru',
    'uk',
    'zh',
  ].contains(locale.languageCode);

  @override
  bool shouldReload(_WalletLocalizationsDelegate old) => false;
}

WalletLocalizations lookupWalletLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'ar':
      return WalletLocalizationsAr();
    case 'de':
      return WalletLocalizationsDe();
    case 'en':
      return WalletLocalizationsEn();
    case 'es':
      return WalletLocalizationsEs();
    case 'fi':
      return WalletLocalizationsFi();
    case 'fr':
      return WalletLocalizationsFr();
    case 'he':
      return WalletLocalizationsHe();
    case 'it':
      return WalletLocalizationsIt();
    case 'ja':
      return WalletLocalizationsJa();
    case 'nb':
      return WalletLocalizationsNb();
    case 'nl':
      return WalletLocalizationsNl();
    case 'pl':
      return WalletLocalizationsPl();
    case 'pt':
      return WalletLocalizationsPt();
    case 'ru':
      return WalletLocalizationsRu();
    case 'uk':
      return WalletLocalizationsUk();
    case 'zh':
      return WalletLocalizationsZh();
  }

  throw FlutterError(
    'WalletLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
