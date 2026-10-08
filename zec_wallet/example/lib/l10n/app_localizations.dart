import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
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
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

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
  static const List<Locale> supportedLocales = <Locale>[Locale('en')];

  /// Application title (task switcher / window title).
  ///
  /// In en, this message translates to:
  /// **'ZEC Wallet'**
  String get appTitle;

  /// The example's settings screen title (network, theme, text size, device log, diagnostics).
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get appearanceTitle;

  /// Section header above the theme-mode options.
  ///
  /// In en, this message translates to:
  /// **'Theme'**
  String get themeSectionTitle;

  /// Theme mode option: follow the OS.
  ///
  /// In en, this message translates to:
  /// **'System'**
  String get themeModeSystemTitle;

  /// Subtitle for the System theme option.
  ///
  /// In en, this message translates to:
  /// **'Follow the device setting'**
  String get themeModeSystemSubtitle;

  /// Theme mode option: light.
  ///
  /// In en, this message translates to:
  /// **'Light'**
  String get themeModeLightTitle;

  /// Theme mode option: dark.
  ///
  /// In en, this message translates to:
  /// **'Dark'**
  String get themeModeDarkTitle;

  /// AMOLED variant toggle title.
  ///
  /// In en, this message translates to:
  /// **'True-black dark'**
  String get amoledTitle;

  /// AMOLED toggle subtitle; also explains why the toggle is disabled while the light theme is active.
  ///
  /// In en, this message translates to:
  /// **'Pure black surfaces on OLED screens. Affects the dark theme only.'**
  String get amoledSubtitle;

  /// Section header above the text-size slider.
  ///
  /// In en, this message translates to:
  /// **'Text size'**
  String get textSizeSectionTitle;

  /// Screen-reader value announced by the text-size slider (control type + current value in one node).
  ///
  /// In en, this message translates to:
  /// **'Text size {percent} percent'**
  String textSizeSemanticValue(int percent);

  /// Live preview line that scales with the slider.
  ///
  /// In en, this message translates to:
  /// **'The quick brown fox jumps over the lazy dog.'**
  String get textSizePreview;

  /// Unknown-route error screen title.
  ///
  /// In en, this message translates to:
  /// **'Page not found'**
  String get routeErrorTitle;

  /// Unknown-route error body — plain language with a next step, never an error code.
  ///
  /// In en, this message translates to:
  /// **'That screen doesn\'t exist in this build. Head back home and continue from there.'**
  String get routeErrorBody;

  /// Unknown-route error action button.
  ///
  /// In en, this message translates to:
  /// **'Go to wallet'**
  String get routeErrorGoHome;

  /// Diagnostics screen title — the live incoming-funds event stream (ADR-0536).
  ///
  /// In en, this message translates to:
  /// **'Incoming events'**
  String get diagIncomingTitle;

  /// Developer-section entry row on the Appearance screen.
  ///
  /// In en, this message translates to:
  /// **'Incoming events'**
  String get diagIncomingEntryTitle;

  /// Subtitle for the diagnostics entry row.
  ///
  /// In en, this message translates to:
  /// **'Developer: watch the live arrival stream'**
  String get diagIncomingEntrySubtitle;

  /// Empty state for the incoming-events diagnostics list.
  ///
  /// In en, this message translates to:
  /// **'No events yet. The stream\'s first event is the catch-up replay; new arrivals appear as the wallet syncs.'**
  String get diagIncomingEmpty;

  /// Appearance-screen section header for developer/diagnostics entries.
  ///
  /// In en, this message translates to:
  /// **'Developer'**
  String get diagIncomingDeveloperSection;

  /// Developer-section entry title for the FR-25 prefilled-send demo.
  ///
  /// In en, this message translates to:
  /// **'Prefilled send (ZIP-321)'**
  String get prefillDemoEntryTitle;

  /// Developer-section entry subtitle for the prefilled-send demo.
  ///
  /// In en, this message translates to:
  /// **'Developer: open the send flow from a payment URI or a typed request'**
  String get prefillDemoEntrySubtitle;

  /// App-bar title of the prefilled-send demo screen.
  ///
  /// In en, this message translates to:
  /// **'Prefilled send'**
  String get prefillDemoTitle;

  /// Explanatory intro on the prefilled-send demo screen.
  ///
  /// In en, this message translates to:
  /// **'Paste a ZIP-321 payment URI (as a scanned QR or deep link would supply) and open the audited send flow prefilled. A malformed or wrong-network URI is rejected here, never a half-filled form.'**
  String get prefillDemoIntro;

  /// Label for the payment-URI text field.
  ///
  /// In en, this message translates to:
  /// **'ZIP-321 payment URI'**
  String get prefillDemoUriLabel;

  /// Placeholder for the payment-URI text field.
  ///
  /// In en, this message translates to:
  /// **'zcash:u1…?amount=…'**
  String get prefillDemoUriHint;

  /// Toggle: pass lockRecipient:true so the recipient opens read-only.
  ///
  /// In en, this message translates to:
  /// **'Lock the recipient (open it read-only)'**
  String get prefillDemoLock;

  /// Button that parses the URI and opens the prefilled send flow.
  ///
  /// In en, this message translates to:
  /// **'Open send from URI'**
  String get prefillDemoOpenFromUri;

  /// Button that opens the send flow from a hardcoded typed WalletSendRequest (no URI).
  ///
  /// In en, this message translates to:
  /// **'Open send from a typed request'**
  String get prefillDemoOpenTyped;

  /// Snackbar shown when fromUri rejects the URI; {fault} is the typed fault name.
  ///
  /// In en, this message translates to:
  /// **'Rejected at the seam: {fault}'**
  String prefillDemoRejected(String fault);

  /// Shown when the demo is opened with no live wallet session.
  ///
  /// In en, this message translates to:
  /// **'No active wallet — create or unlock one first.'**
  String get prefillDemoNoWallet;

  /// Appearance-screen section header above the Tor plugin toggle.
  ///
  /// In en, this message translates to:
  /// **'Network'**
  String get torPluginSectionTitle;

  /// Toggle: route the wallet through the optional zec_wallet_tor plugin. Off by default.
  ///
  /// In en, this message translates to:
  /// **'Use the built-in Tor plugin'**
  String get torPluginToggleTitle;

  /// Subtitle under the Tor plugin toggle: the transport is chosen before the wallet opens.
  ///
  /// In en, this message translates to:
  /// **'Takes effect the next time the app starts.'**
  String get torPluginToggleSubtitle;

  /// Appearance-screen section header above the device-log level and its Share action.
  ///
  /// In en, this message translates to:
  /// **'Device log'**
  String get deviceLogSectionTitle;

  /// Device-log level: nothing is written.
  ///
  /// In en, this message translates to:
  /// **'Off'**
  String get deviceLogOffTitle;

  /// Device-log level: only what went wrong.
  ///
  /// In en, this message translates to:
  /// **'Errors only'**
  String get deviceLogErrorsTitle;

  /// Device-log level: errors plus the connection narrative.
  ///
  /// In en, this message translates to:
  /// **'Detailed'**
  String get deviceLogDetailedTitle;

  /// One line shown when the level the wallet answers differs from the one picked: nothing on this device can write the log.
  ///
  /// In en, this message translates to:
  /// **'This device can\'t keep a log'**
  String get deviceLogUnavailable;

  /// Action: open the system share sheet with the device log as text. Disabled while the log is empty.
  ///
  /// In en, this message translates to:
  /// **'Share device log'**
  String get deviceLogShareTitle;

  /// Snackbar when the system share sheet could not be opened.
  ///
  /// In en, this message translates to:
  /// **'Sharing isn\'t available on this device.'**
  String get deviceLogShareFailed;

  /// Dialog title offered after a wallet delete while the Tor plugin is on.
  ///
  /// In en, this message translates to:
  /// **'Reset the Tor identity too?'**
  String get torIdentityResetTitle;

  /// Dialog body: the Tor guard state is a per-device record a wallet delete leaves behind.
  ///
  /// In en, this message translates to:
  /// **'Tor remembers which relays this device uses. Deleting the wallet keeps that record.'**
  String get torIdentityResetBody;

  /// Dialog action: delete the Tor plugin's state.
  ///
  /// In en, this message translates to:
  /// **'Reset'**
  String get torIdentityResetConfirm;

  /// Dialog action: keep the Tor plugin's state.
  ///
  /// In en, this message translates to:
  /// **'Keep'**
  String get torIdentityResetCancel;

  /// Dialog title when the Tor identity reset the user asked for failed.
  ///
  /// In en, this message translates to:
  /// **'Tor identity not reset'**
  String get torIdentityResetFailedTitle;

  /// Dialog body: the reset did not happen; the guard state is still on disk.
  ///
  /// In en, this message translates to:
  /// **'This device\'s Tor relay record is still stored.'**
  String get torIdentityResetFailedBody;

  /// Dialog action: retry the Tor identity reset.
  ///
  /// In en, this message translates to:
  /// **'Try again'**
  String get torIdentityResetFailedRetry;

  /// Dialog action: dismiss without retrying.
  ///
  /// In en, this message translates to:
  /// **'OK'**
  String get torIdentityResetFailedDismiss;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
