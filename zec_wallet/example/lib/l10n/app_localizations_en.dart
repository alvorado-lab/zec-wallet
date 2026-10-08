// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'ZEC Wallet';

  @override
  String get appearanceTitle => 'Settings';

  @override
  String get themeSectionTitle => 'Theme';

  @override
  String get themeModeSystemTitle => 'System';

  @override
  String get themeModeSystemSubtitle => 'Follow the device setting';

  @override
  String get themeModeLightTitle => 'Light';

  @override
  String get themeModeDarkTitle => 'Dark';

  @override
  String get amoledTitle => 'True-black dark';

  @override
  String get amoledSubtitle =>
      'Pure black surfaces on OLED screens. Affects the dark theme only.';

  @override
  String get textSizeSectionTitle => 'Text size';

  @override
  String textSizeSemanticValue(int percent) {
    return 'Text size $percent percent';
  }

  @override
  String get textSizePreview => 'The quick brown fox jumps over the lazy dog.';

  @override
  String get routeErrorTitle => 'Page not found';

  @override
  String get routeErrorBody =>
      'That screen doesn\'t exist in this build. Head back home and continue from there.';

  @override
  String get routeErrorGoHome => 'Go to wallet';

  @override
  String get diagIncomingTitle => 'Incoming events';

  @override
  String get diagIncomingEntryTitle => 'Incoming events';

  @override
  String get diagIncomingEntrySubtitle =>
      'Developer: watch the live arrival stream';

  @override
  String get diagIncomingEmpty =>
      'No events yet. The stream\'s first event is the catch-up replay; new arrivals appear as the wallet syncs.';

  @override
  String get diagIncomingDeveloperSection => 'Developer';

  @override
  String get prefillDemoEntryTitle => 'Prefilled send (ZIP-321)';

  @override
  String get prefillDemoEntrySubtitle =>
      'Developer: open the send flow from a payment URI or a typed request';

  @override
  String get prefillDemoTitle => 'Prefilled send';

  @override
  String get prefillDemoIntro =>
      'Paste a ZIP-321 payment URI (as a scanned QR or deep link would supply) and open the audited send flow prefilled. A malformed or wrong-network URI is rejected here, never a half-filled form.';

  @override
  String get prefillDemoUriLabel => 'ZIP-321 payment URI';

  @override
  String get prefillDemoUriHint => 'zcash:u1…?amount=…';

  @override
  String get prefillDemoLock => 'Lock the recipient (open it read-only)';

  @override
  String get prefillDemoOpenFromUri => 'Open send from URI';

  @override
  String get prefillDemoOpenTyped => 'Open send from a typed request';

  @override
  String prefillDemoRejected(String fault) {
    return 'Rejected at the seam: $fault';
  }

  @override
  String get prefillDemoNoWallet =>
      'No active wallet — create or unlock one first.';

  @override
  String get torPluginSectionTitle => 'Network';

  @override
  String get torPluginToggleTitle => 'Use the built-in Tor plugin';

  @override
  String get torPluginToggleSubtitle =>
      'Takes effect the next time the app starts.';

  @override
  String get deviceLogSectionTitle => 'Device log';

  @override
  String get deviceLogOffTitle => 'Off';

  @override
  String get deviceLogErrorsTitle => 'Errors only';

  @override
  String get deviceLogDetailedTitle => 'Detailed';

  @override
  String get deviceLogUnavailable => 'This device can\'t keep a log';

  @override
  String get deviceLogShareTitle => 'Share device log';

  @override
  String get deviceLogShareFailed => 'Sharing isn\'t available on this device.';

  @override
  String get torIdentityResetTitle => 'Reset the Tor identity too?';

  @override
  String get torIdentityResetBody =>
      'Tor remembers which relays this device uses. Deleting the wallet keeps that record.';

  @override
  String get torIdentityResetConfirm => 'Reset';

  @override
  String get torIdentityResetCancel => 'Keep';

  @override
  String get torIdentityResetFailedTitle => 'Tor identity not reset';

  @override
  String get torIdentityResetFailedBody =>
      'This device\'s Tor relay record is still stored.';

  @override
  String get torIdentityResetFailedRetry => 'Try again';

  @override
  String get torIdentityResetFailedDismiss => 'OK';
}
