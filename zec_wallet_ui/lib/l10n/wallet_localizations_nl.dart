// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Dutch Flemish (`nl`).
class WalletLocalizationsNl extends WalletLocalizations {
  WalletLocalizationsNl([String locale = 'nl']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Instellingen';

  @override
  String get walletTitle => 'Wallet';

  @override
  String get walletNotSetUpTitle => 'Wallet nog niet ingesteld';

  @override
  String get walletNotSetUpBody =>
      'Het instellen van de wallet volgt in een latere versie. Daarbij wordt u stap voor stap geholpen om uw herstelzin op te schrijven voordat er tegoeden kunnen worden ontvangen — zo loopt u nooit risico zonder back-up.';

  @override
  String get walletStartupFailedTitle => 'De wallet kon niet worden gestart';

  @override
  String get walletStartupFailedBody =>
      'Er is iets misgegaan waardoor de wallet niet kon laden op dit apparaat. Als u al een wallet heeft, lopen de tegoeden ervan geen risico — ze bevinden zich op het Zcash-netwerk en kunnen worden hersteld met uw herstelzin. Probeer het opnieuw; als dit blijft gebeuren, sluit de app en open deze opnieuw.';

  @override
  String get walletBalanceLabel => 'Saldo';

  @override
  String get walletHideBalance => 'Saldo verbergen';

  @override
  String get walletShowBalance => 'Saldo tonen';

  @override
  String get walletBalanceHiddenAmount => 'Saldo verborgen';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Nu besteedbaar';

  @override
  String get walletArrivingLabel => 'Onderweg';

  @override
  String get walletNotSpendableYetLabel => 'Nog niet besteedbaar';

  @override
  String get walletActivityTitle => 'Activiteit';

  @override
  String get walletActivityEmpty => 'Nog geen activiteit';

  @override
  String get walletActivityError => 'Activiteit kon niet worden geladen';

  @override
  String get walletActivityReceived => 'Ontvangen';

  @override
  String get walletActivitySent => 'Verzonden';

  @override
  String get walletActivityPending => 'In behandeling';

  @override
  String get walletActivityQueued => 'In wachtrij';

  @override
  String get walletActivityRetrying => 'Opnieuw proberen';

  @override
  String get walletActivitySaved => 'Opgeslagen';

  @override
  String get walletActivityExpired => 'Verlopen';

  @override
  String get walletActivityFailed => 'Mislukt';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count bevestigingen',
      one: '1 bevestiging',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count betalingen ontvangen',
      one: 'Betaling ontvangen',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Transactiedetails weergeven';

  @override
  String get walletTxDetailStatus => 'Status';

  @override
  String get walletTxDetailFee => 'Netwerkkosten';

  @override
  String get walletTxDetailDate => 'Datum';

  @override
  String get walletTxDetailHeight => 'Blokhoogte';

  @override
  String get walletTxDetailMemo => 'Memo';

  @override
  String get walletTxDetailMemoAttached => 'Aanwezig';

  @override
  String get walletTxDetailTxid => 'Transactie-ID';

  @override
  String get walletTxDetailCopyTxid => 'Transactie-ID kopiëren';

  @override
  String get walletTxDetailCopied => 'Transactie-ID gekopieerd';

  @override
  String get walletTxDetailClose => 'Sluiten';

  @override
  String get walletTxFundsKept => 'Er heeft geen tegoed uw wallet verlaten';

  @override
  String get walletTxExplainQueued =>
      'Opgeslagen op dit apparaat, onder Opgeslagen & in behandeling — u kunt deze daar verzenden of annuleren.';

  @override
  String get walletTxExplainPending =>
      'Verzonden naar het Zcash-netwerk — wacht op bevestiging in een blok.';

  @override
  String get walletTxExplainRetrying =>
      'Uw wallet kon dit nog niet naar het Zcash-netwerk verzenden. De ondertekende transactie blijft bewaard en wordt bij elke synchronisatie opnieuw geprobeerd totdat deze doorkomt of verloopt.';

  @override
  String get walletTxExplainSaved =>
      'Uw wallet heeft deze ondertekende transactie bewaard, maar verzendt deze op dit moment niet uit zichzelf.';

  @override
  String get walletTxExplainConfirmed => 'Bevestigd op het Zcash-netwerk.';

  @override
  String get walletTxExplainExpired =>
      'Deze transactie is verlopen voordat het netwerk haar bevestigde en is daarom geannuleerd. Het bedrag is nog steeds van u om te besteden.';

  @override
  String get walletTxExplainFailed =>
      'Het netwerk heeft deze transactie geweigerd, waardoor ze niet is doorgegaan. Het bedrag is nog steeds van u om te besteden.';

  @override
  String get walletTxExplainUnknown =>
      'De huidige status van deze transactie kan niet worden vastgesteld. Dit wordt bijgewerkt na de volgende synchronisatie.';

  @override
  String get walletMenuTooltip => 'Meer opties';

  @override
  String get walletRescanMenuItem => 'Geschiedenis opnieuw scannen…';

  @override
  String get walletCheckOneTimeMenuItem => 'Eenmalige adressen controleren…';

  @override
  String get walletRescanTitle => 'Uw geschiedenis opnieuw scannen';

  @override
  String get walletRescanBody =>
      'Mist u oudere tegoeden? Scan de blockchain verder terug om stortingen te herstellen die door een latere startdatum zijn overgeslagen. Uw tegoeden en herstelzin lopen nooit risico.';

  @override
  String get walletRescanRangeTitle => 'Hoe ver terug te scannen';

  @override
  String get walletRescanRangeAll =>
      'Scan uw hele geschiedenis — traagst, maar herstelt alles.';

  @override
  String get walletRescanRangeDefault =>
      'Scant vanaf het begin van uw wallet. Mist u nog steeds oudere tegoeden? Kies een eerdere datum, of Scan alle geschiedenis.';

  @override
  String get walletRescanRangeResolving =>
      'Aanbevolen periode wordt voorbereid…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Ongeveer $blocks blokken om te scannen.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Scant vanaf $date. Mist u nog steeds oudere tegoeden? Kies een eerdere datum, of Scan alle geschiedenis.';
  }

  @override
  String get walletRescanPick => 'Kies een datum';

  @override
  String get walletRescanChange => 'Datum wijzigen';

  @override
  String get walletRescanScanAll => 'Scan alle geschiedenis';

  @override
  String get walletRescanDatePick => 'Vroegste datum om te scannen';

  @override
  String get walletRescanWarning =>
      'Dit scant de blockchain opnieuw. Recente datums duren enkele minuten; ver terugscannen kan uren duren. Synchronisatie loopt op de achtergrond — u kunt uw wallet blijven gebruiken.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Een betaling vanuit deze wallet wordt nog bevestigd. De wallet weigert meestal opnieuw te scannen totdat deze is voltooid — u kunt het proberen, maar houd er rekening mee dat dit wordt geweigerd.';

  @override
  String get walletRescanConfirm => 'Herscan starten';

  @override
  String get walletRescanCancel => 'Annuleren';

  @override
  String get walletRescanRunning => 'Wordt herbouwd…';

  @override
  String get walletRescanRebuildingAll =>
      'Uw geschiedenis wordt herbouwd — de hele keten wordt gescand. Uw saldo en activiteit vullen zich aan naarmate dit vordert.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Uw geschiedenis wordt herbouwd vanaf $date — uw saldo en activiteit vullen zich aan naarmate dit vordert.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Uw geschiedenis wordt herbouwd vanaf het begin van uw wallet — uw saldo en activiteit vullen zich aan naarmate dit vordert.';

  @override
  String get walletCatchUpBanner =>
      'Bezig met inhalen — uw saldo en activiteit vullen zich aan terwijl de wallet synchroniseert. Alles wat u heeft ontvangen is veilig.';

  @override
  String get walletCatchUpRescanBanner =>
      'Uw geschiedenis wordt herbouwd na een herscan — uw saldo en activiteit vullen zich aan naarmate dit vordert. Alles wat u heeft ontvangen is veilig.';

  @override
  String get walletRescanFailedNotice =>
      'Kon nu niet opnieuw scannen — uw tegoeden zijn veilig, al kunnen uw saldo en geschiedenis wat tijd nodig hebben om het weer bij te benen. Probeer het straks opnieuw.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Een betaling wordt nog bevestigd, daarom is opnieuw scannen gepauzeerd om uw tegoeden te beschermen. Uw wallet is ongewijzigd — probeer het over een paar uur opnieuw en houd de app ondertussen open en online.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'Opnieuw scannen bouwt uw geschiedenis opnieuw op terwijl uw wallet synchroniseert, en synchronisatie loopt nu niet. Uw wallet is ongewijzigd — probeer het opnieuw zodra synchronisatie loopt.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Er is niet genoeg vrije opslagruimte om de geschiedenis van uw wallet opnieuw op te bouwen — uw tegoeden zijn veilig, al kunnen uw saldo en geschiedenis wat tijd nodig hebben om het weer bij te benen. Maak ruimte vrij en probeer het opnieuw.';

  @override
  String get walletRescanFailedDismiss => 'Sluiten';

  @override
  String get walletActivityRebuilding => 'Uw geschiedenis wordt herbouwd…';

  @override
  String get walletActivityCatchingUp =>
      'Nog steeds bezig met inhalen — alles wat u heeft ontvangen, verschijnt hier.';

  @override
  String get walletActivitySyncNotRunning =>
      'Het laden van uw saldo en geschiedenis wordt voltooid zodra synchronisatie loopt.';

  @override
  String get walletActivityLoadMore => 'Meer laden';

  @override
  String get walletPendingChangeLabel => 'Wisselgeld in behandeling';

  @override
  String get walletTransparentLabel => 'Niet-afgeschermd (openbaar)';

  @override
  String get walletTransparentNote =>
      'Niet inbegrepen in \"Nu besteedbaar\" — scherm deze tegoeden af om ze te kunnen besteden. Tot die tijd blijven ze openbaar zichtbaar op de blockchain.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Deze tegoeden zijn openbaar zichtbaar op de blockchain.';

  @override
  String walletPoolShielded(String amount) {
    return 'Afgeschermd $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Openbaar $amount';
  }

  @override
  String get walletPoolAllShielded => 'Alles afgeschermd · privé';

  @override
  String get walletPoolTapHint => 'Openbare tegoeden weergeven';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount van uw saldo staat op een eenmalig adres (herstelbaar).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount van uw saldo staat op een eenmalig adres.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Betalingen met een totaal van $amount zijn toegewezen en worden nog voltooid via eenmalige adressen die uw wallet beheert. Verstuur ze niet opnieuw.',
      one:
          '$amount is toegewezen aan een betaling die uw wallet nog voltooit via een eenmalig adres dat uw wallet beheert. Verstuur het niet opnieuw.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Betalingen met een totaal van $amount zijn toegewezen en zijn halverwege via eenmalige adressen die uw wallet beheert. Ze zijn gepauzeerd totdat uw wallet weer synchroniseert. Verstuur ze niet opnieuw.',
      one:
          '$amount is toegewezen aan een betaling die halverwege is via een eenmalig adres dat uw wallet beheert. Het is gepauzeerd totdat uw wallet weer synchroniseert. Verstuur het niet opnieuw.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Kon niet controleren of een betaling nog wordt voltooid. Opnieuw proberen — kijk tot die tijd in uw activiteit of er een betaling in behandeling is voordat u opnieuw verzendt.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount van uw saldo staat op een eenmalig adres (wordt nog bevestigd).';
  }

  @override
  String get walletShieldButton => 'Afschermen';

  @override
  String get walletShieldSheetTitle => 'Openbare tegoeden afschermen';

  @override
  String get walletShieldNote =>
      'Dit verplaatst tegoeden van uw openbare, op de blockchain zichtbare saldo naar uw privé afgeschermde saldo.';

  @override
  String get walletShieldPreparing => 'Wordt voorbereid…';

  @override
  String get walletShieldAmountLabel => 'Af te schermen';

  @override
  String get walletShieldFeeLabel => 'Netwerkkosten';

  @override
  String get walletShieldNetLabel => 'Netto afgeschermd';

  @override
  String get walletShieldConfirmButton => 'Nu afschermen';

  @override
  String get walletShieldSubmitting => 'Wordt afgeschermd…';

  @override
  String get walletShieldNothingTitle => 'Nog niets om af te schermen';

  @override
  String get walletShieldNothingBody =>
      'Dit bedrag ligt nu onder de drempel waarbij afschermen de moeite waard is — de netwerkkosten zouden hoger zijn dan het voordeel. Zodra er iets meer binnenkomt, kan het worden afgeschermd.';

  @override
  String get walletShieldDoneTitle => 'Afschermen ingediend';

  @override
  String get walletShieldDoneBody =>
      'Uw tegoeden worden verplaatst naar uw afgeschermde saldo. Dit wordt binnenkort bevestigd op de blockchain.';

  @override
  String get walletShieldSavedTitle =>
      'Opgeslagen — we ronden het afschermen af';

  @override
  String get walletShieldSavedBody =>
      'We konden het netwerk nu niet bereiken. Uw afscherming is opgeslagen en uw wallet rondt deze af bij een latere synchronisatie. Er is niets verloren gegaan.';

  @override
  String get walletShieldAlreadyTitle => 'Al ingediend';

  @override
  String get walletShieldFailedTitle => 'Kon nu niet afschermen';

  @override
  String get walletShieldStaleBody =>
      'De wallet synchroniseert nog. Probeer straks opnieuw af te schermen.';

  @override
  String get walletShieldTransientBody =>
      'De afscherming kon zojuist niet worden voorbereid. Probeer het over een ogenblik opnieuw.';

  @override
  String get walletShieldStorageFullBody =>
      'Er is niet genoeg vrije opslagruimte om nu af te schermen. Maak ruimte vrij en probeer het opnieuw. Uw tegoeden zijn veilig.';

  @override
  String get walletShieldClose => 'Sluiten';

  @override
  String get walletShieldRetry => 'Opnieuw proberen';

  @override
  String get walletMoveMenuItem => 'Verplaatsen naar openbaar…';

  @override
  String get walletMoveSheetTitle => 'Verplaatsen naar openbaar';

  @override
  String get walletMoveSheetSubtitle =>
      'Verstuur afgeschermde ZEC naar uw eigen openbare adres — handig voor een beurs die geen afgeschermde storting accepteert.';

  @override
  String get walletMoveDestinationLabel => 'Uw openbare adres';

  @override
  String walletMoveAvailable(String amount) {
    return 'Beschikbaar om te verplaatsen: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Beschikbaar om te verplaatsen: $amount ZEC — uw saldo is nog aan het inhalen';
  }

  @override
  String get walletMoveDeshieldTitle =>
      'Deze verplaatsing maakt uw tegoeden openbaar';

  @override
  String get walletMoveDeshieldBody =>
      'Verplaatsen naar een openbaar adres haalt deze tegoeden uit uw afgeschermde saldo — het bedrag en uw openbare adres worden openbaar zichtbaar op de Zcash-blockchain.';

  @override
  String get walletMoveWalletEnded =>
      'De walletsessie is beëindigd. Sluit de wallet en open deze opnieuw om het nogmaals te proberen.';

  @override
  String get walletMoveLoading => 'Wordt voorbereid…';

  @override
  String get walletMovePreparing => 'Bedrag wordt gecontroleerd…';

  @override
  String get walletMoveSubmitting => 'Wordt verplaatst…';

  @override
  String get walletMoveReviewButton => 'Controleren';

  @override
  String get walletMoveCancel => 'Annuleren';

  @override
  String get walletMoveReviewTitle => 'Verplaatsing controleren';

  @override
  String get walletMoveOwnAddressNote =>
      'U verplaatst naar uw eigen openbare adres. U kunt deze tegoeden later weer afschermen, maar deze verplaatsing blijft permanent in het openbare register staan.';

  @override
  String get walletMoveConfirmButton => 'Verplaatsen naar openbaar';

  @override
  String get walletMoveBackButton => 'Terug';

  @override
  String get walletMoveDoneTitle => 'Verplaatst naar openbaar';

  @override
  String get walletMoveDoneBody =>
      'Uw tegoeden worden verplaatst naar uw openbare adres. Dit wordt binnenkort bevestigd op de blockchain.';

  @override
  String get walletMoveSavedTitle =>
      'Opgeslagen — we ronden de verplaatsing af';

  @override
  String get walletMoveSavedBody =>
      'Deze verplaatsing is opgeslagen en uw wallet verzendt deze bij een latere synchronisatie. Er is niets verloren gegaan.';

  @override
  String get walletMoveAlreadyTitle => 'Al ingediend';

  @override
  String get walletMoveAlreadyBody =>
      'Deze tegoeden zijn al ingediend en onderweg naar uw openbare adres.';

  @override
  String get walletMoveFailedTitle => 'Kon deze verplaatsing niet voltooien';

  @override
  String get walletMoveNothingTitle => 'Nog niets om te verplaatsen';

  @override
  String get walletMoveNothingBody =>
      'U heeft nu geen afgeschermd saldo beschikbaar om te verplaatsen. Zodra tegoeden bevestigd zijn, kunt u ze verplaatsen naar uw openbare adres.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Uw wallet is nog aan het inhalen — alles wat u heeft ontvangen wordt beschikbaar om te verplaatsen zodra het synchroniseren is voltooid.';

  @override
  String get walletMoveCouldNotLoad =>
      'Uw openbare adres kon niet worden geladen. Probeer het opnieuw.';

  @override
  String get walletMoveRetry => 'Opnieuw proberen';

  @override
  String get walletMoveClose => 'Sluiten';

  @override
  String get walletSnapshotUnavailable =>
      'De wallet kon nu niet worden gelezen. Dit wordt automatisch ververst.';

  @override
  String get walletBalanceStale =>
      'Verversen mislukt — laatst bekende saldo wordt getoond.';

  @override
  String get walletSyncStartFailed =>
      'Synchroniseren kon niet worden gestart. We blijven het proberen.';

  @override
  String get walletSyncRetry => 'Opnieuw proberen';

  @override
  String get walletSyncTryNow => 'Nu proberen';

  @override
  String get walletSyncIdle => 'Nog niet aan het synchroniseren';

  @override
  String get walletSyncIdleDetail => 'Synchronisatie start automatisch.';

  @override
  String get walletSyncDisabled => 'Synchronisatie uit';

  @override
  String get walletSyncDisabledDetail =>
      'Zet synchronisatie aan in de instellingen van deze app om uw saldo bij te werken.';

  @override
  String get walletSyncExplainDisabled =>
      'Synchronisatie staat uit in de instellingen van deze app. Uw tegoeden zijn veilig. Uw saldo en activiteit tonen de laatst gesynchroniseerde stand en worden niet bijgewerkt totdat synchronisatie weer wordt ingeschakeld.';

  @override
  String get walletParkedSyncPausedNote =>
      'Uw wallet synchroniseert niet, dus deze worden niet vanzelf verzonden. Gebruik Nu verzenden om er zelf een te verzenden.';

  @override
  String get walletSyncPausedMoneyNote =>
      'Gepauzeerd totdat uw wallet weer synchroniseert.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Verbinden…';

  @override
  String get walletSyncStartingDetail =>
      'Verbinding wordt gemaakt met het Zcash-netwerk en scannen wordt voorbereid.';

  @override
  String get walletSyncConnecting => 'Verbinden…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Verbinden… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Scannen $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'Scannen…';

  @override
  String get walletSyncSpendableReady => 'Tegoeden zijn klaar om te besteden.';

  @override
  String get walletSyncCatchingUp =>
      'Bezig met inhalen op het netwerk — een uitgebreide eerste synchronisatie kan even duren. U kunt de app blijven gebruiken terwijl dit wordt afgerond';

  @override
  String walletSyncScanRemaining(String count) {
    return 'Nog $count blokken';
  }

  @override
  String get walletSyncUpToDate => 'Actueel';

  @override
  String get walletSyncOffline => 'Offline';

  @override
  String get walletSyncOfflineDetail =>
      'Verzendingen in de wachtrij blijven opgeslagen onder Opgeslagen & in behandeling.';

  @override
  String get walletSyncUnknown => 'Synchroniseren…';

  @override
  String get walletSyncStalled => 'Synchronisatie gepauzeerd';

  @override
  String get walletStallEndpoint =>
      'Het Zcash-netwerk is nu niet bereikbaar. We blijven het automatisch proberen — controleer uw internetverbinding, of de server is mogelijk tijdelijk niet beschikbaar.';

  @override
  String get walletStallTor =>
      'Het privépad van uw app is niet beschikbaar, dus de wallet maakt geen verbinding. Controleer de netwerkinstellingen van uw app of schakel het privépad uit. Synchronisatie wordt hervat zodra het pad er weer is.';

  @override
  String get walletStallStorage =>
      'De opslag van uw apparaat is vol. Maak ruimte vrij, dan wordt synchronisatie hervat.';

  @override
  String get walletStallReorg =>
      'Er heeft zich een chain-reorganisatie voorgedaan; recente blokken worden opnieuw gecontroleerd.';

  @override
  String get walletStallInternal =>
      'Een lokaal probleem heeft synchronisatie gestopt. Blijft dit gebeuren, herstel dan vanaf uw herstelzin.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Deze server heeft gegevens gestuurd die niet kunnen kloppen, dus de synchronisatie is gestopt. Dit is geen verbindingsprobleem — schakel over naar een andere server. Wordt elke server geweigerd, scan dan de geschiedenis opnieuw: de wallet bewaart mogelijk een foutief record van een eerdere server.';

  @override
  String get walletStallBirthdayInFuture =>
      'Deze wallet is ingesteld om te starten vanaf een blok dat deze server nog niet heeft bereikt. Controleer het startblok waarop deze wallet is ingesteld, of probeer een andere server.';

  @override
  String get walletStallStorageUnavailable =>
      'Synchronisatie gepauzeerd op dit apparaat. Wordt opnieuw geprobeerd.';

  @override
  String get walletStallUnknown =>
      'Synchronisatie is gestopt om een onbekende reden.';

  @override
  String get walletSyncBadgeHint => 'Synchronisatiedetails weergeven';

  @override
  String get walletSyncSheetClose => 'Sluiten';

  @override
  String get walletSyncSheetProgress => 'Voortgang';

  @override
  String get walletSyncSheetBlocksLeft => 'Resterende blokken';

  @override
  String get walletSyncSheetSyncedTo => 'Gesynchroniseerd tot blok';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Minstens $blocks blokken achter',
      one: 'Minstens 1 blok achter',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'Synchronisatie is nog niet gestart — dit gebeurt automatisch. Geen actie nodig.';

  @override
  String get walletSyncExplainStartFailed =>
      'Synchroniseren kon niet starten. Uw tegoeden zijn veilig — de wallet controleert momenteel gewoon niet op nieuwe activiteit. Probeer het hieronder opnieuw, of open de app opnieuw.';

  @override
  String get walletSyncExplainStarting =>
      'De wallet legt contact met het Zcash-netwerk en bereidt het scannen voor. Dit duurt meestal enkele seconden.';

  @override
  String get walletSyncExplainConnecting =>
      'Er wordt verbinding gemaakt met het Zcash-netwerk.';

  @override
  String get walletSyncExplainScanning =>
      'De wallet controleert blockchainblokken op uw tegoeden. Uw saldo en activiteit worden bijgewerkt zodra nieuwe transacties worden gevonden — u kunt de app blijven gebruiken terwijl dit wordt afgerond.';

  @override
  String get walletSyncExplainUpToDate =>
      'Volledig gesynchroniseerd met het Zcash-netwerk. Uw saldo en activiteit zijn actueel.';

  @override
  String get walletSyncExplainStalled =>
      'Synchronisatie is op een probleem gestuit en gepauzeerd. Dit wordt automatisch opnieuw geprobeerd.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Het Zcash-netwerk is niet bereikbaar — dat is normaal als u offline bent, of de server is mogelijk tijdelijk niet beschikbaar. Uw tegoeden zijn veilig: het saldo toont de laatst gesynchroniseerde stand, en verzendingen in de wachtrij blijven opgeslagen onder Opgeslagen & in behandeling. De verbinding wordt automatisch opnieuw geprobeerd.';

  @override
  String get walletSyncExplainOffline =>
      'Geen netwerkverbinding. Uw tegoeden zijn veilig — het saldo toont de laatst gesynchroniseerde stand, en verzendingen in de wachtrij blijven opgeslagen onder Opgeslagen & in behandeling.';

  @override
  String get walletSyncExplainUnknown =>
      'De wallet synchroniseert. Uw saldo en activiteit worden bijgewerkt naarmate dit vordert.';

  @override
  String get walletTorOff => 'Tor uit';

  @override
  String get walletTorBootstrapping => 'Privépad wordt gestart…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport wordt gestart…';
  }

  @override
  String get walletTorActive => 'Tor actief';

  @override
  String get walletTorActiveUnverified =>
      'Tor actief (niet-geverifieerde runtime)';

  @override
  String get walletTorActiveUnattested =>
      'Privépad in gebruik (privacy niet geverifieerd)';

  @override
  String get walletTorFellBack =>
      'Tor niet beschikbaar — directe verbinding wordt gebruikt';

  @override
  String get walletTorUnavailable =>
      'Privépad niet beschikbaar — geen verbinding';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport niet beschikbaar — geen verbinding';
  }

  @override
  String get walletTorUnanswered => 'Privépad verbonden — er komt niets terug';

  @override
  String get walletTorUnansweredUnattested =>
      'Privépad verbonden — er komt niets terug (privacy niet geverifieerd)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport verbonden — er komt niets terug';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Niet privé (directe verbinding van uw app) — er komt niets terug';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Verbonden via $transport — er komt niets terug; verbindingen kunnen door de proxy aan elkaar worden gekoppeld';
  }

  @override
  String get walletTorUnknown =>
      'Tor-status onbekend — beschouw als niet beschermd';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Saldo (per blok $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Saldo · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Saldo (per blok $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Verbinding';

  @override
  String get walletSyncSheetServer => 'Server';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Server, $host, opent de serverkeuze';
  }

  @override
  String get walletSyncServerSheetTitle => 'Synchronisatieserver';

  @override
  String get walletSyncServerInUse => 'In gebruik';

  @override
  String get walletSyncServerAppDefault => 'App-standaard';

  @override
  String get walletSyncServerCustom => 'Eigen server…';

  @override
  String get walletSyncServerCustomHint => 'https://host:poort';

  @override
  String get walletSyncServerCheck => 'Server controleren';

  @override
  String get walletSyncServerChecking => 'Controleren…';

  @override
  String get walletSyncServerUse => 'Deze server gebruiken';

  @override
  String get walletSyncServerSwitching => 'Wisselen…';

  @override
  String get walletSyncServerContinue => 'Doorgaan';

  @override
  String get walletSyncServerCancel => 'Annuleren';

  @override
  String get walletSyncServerTrustTitle => 'Deze server vertrouwen?';

  @override
  String get walletSyncServerTrustNotice =>
      'U vertrouwt deze server om uw saldo en geschiedenis te melden en uw betalingen door te sturen. Hij ziet uw IP-adres tenzij Tor aanstaat, ongeveer wanneer uw wallet is aangemaakt, de openbare adressen die uw wallet controleert, de transacties die hij opzoekt en de transacties die u verstuurt.';

  @override
  String get walletSyncServerKeyLabel => 'Toegangssleutel (optioneel)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Header van de sleutel';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Voer de header in die uw server verwacht';

  @override
  String get walletSyncServerKeyInvalid =>
      'Deze sleutel of header kan niet worden gebruikt';

  @override
  String get walletSyncServerKeySaved => 'Sleutel opgeslagen';

  @override
  String get walletSyncServerKeyShow => 'Tonen';

  @override
  String get walletSyncServerKeyHide => 'Verbergen';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Uw sleutel identificeert u bij deze server. Hij kan uw betalingen aan uw wallet koppelen, ook via Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Wisselen herstart de lopende synchronisatie. Uw saldo en geschiedenis blijven behouden. Tegoed kan als onderweg worden getoond totdat de scan van de nieuwe server is bijgewerkt.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Wisselen maakt opnieuw verbinding met de nieuwe server. Uw saldo en geschiedenis blijven behouden.';

  @override
  String get walletSyncServerUnreachable =>
      'Deze server is niet bereikbaar. Controleer het adres — en als dat klopt, antwoordt deze server niet of kan uw app hem nu niet bereiken. Probeer het opnieuw of kies een andere server.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Deze server is niet bereikbaar. De wallet kan niet onderscheiden of deze server niet antwoordt of dat uw app hem nu niet kan bereiken. Kies een andere server of probeer het later opnieuw.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Deze server zit op een ander Zcash-netwerk.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Dat ziet er niet uit als een serveradres. Gebruik https://host:poort.';

  @override
  String get walletSyncServerNotOffered =>
      'Deze server wordt niet aangeboden door deze app.';

  @override
  String get walletSyncServerBusy =>
      'De wallet is momenteel bezig. Probeer het zo opnieuw.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'De gekozen server wordt niet meer aangeboden door deze app. $host wordt gebruikt.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'De opgeslagen serverkeuze kon niet worden gelezen. $host wordt gebruikt.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Wisselen is mislukt — $host wordt nog gebruikt.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Walletverkeer maakt rechtstreeks verbinding met de server. De server kan uw IP-adres zien.';

  @override
  String get walletTransportExplainTor =>
      'Walletverkeer wordt via het Tor-netwerk geleid, waardoor uw IP-adres voor de server verborgen blijft.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Het privépad van uw app wordt opgestart. Walletverkeer wacht hierop voordat er verbinding wordt gemaakt.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport wordt opgestart. Walletverkeer wacht hierop voordat er verbinding wordt gemaakt.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Tor kon niet worden bereikt, waardoor het verkeer is teruggevallen op een directe verbinding. De server kan uw IP-adres zien.';

  @override
  String get walletTransportExplainUnavailable =>
      'Het privépad van uw app is niet beschikbaar, dus de wallet maakt geen verbinding. Schakel het privépad uit of controleer de netwerkinstellingen van uw app.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport is niet beschikbaar, dus de wallet maakt geen verbinding. Schakel het uit of controleer de netwerkinstellingen van uw app.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'Het privépad heeft de verbinding geaccepteerd, maar er komt al een minuut niets terug. Het kan het pad zijn of de wallet-server — de wallet kan dat niet onderscheiden. Hij blijft het proberen; als het niet overgaat, probeer een andere server of controleer de netwerkinstellingen van uw app.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport heeft de verbinding geaccepteerd, maar er komt al een minuut niets terug. Het kan het pad zijn of de wallet-server — de wallet kan dat niet onderscheiden. Hij blijft het proberen; als het niet overgaat, probeer een andere server of controleer de netwerkinstellingen van uw app.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Walletverkeer maakt rechtstreeks verbinding met de server. De server kan uw IP-adres zien. De verbinding is geaccepteerd, maar er komt al een minuut niets terug. Het kan het pad zijn of de wallet-server — de wallet kan dat niet onderscheiden. Hij blijft het proberen; als het niet overgaat, probeer een andere server of controleer de netwerkinstellingen van uw app.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'De privacy van deze verbinding kan niet worden geverifieerd — beschouw deze als niet privé. De verbinding is geaccepteerd, maar er komt al een minuut niets terug. Het kan het pad zijn of de wallet-server — de wallet kan dat niet onderscheiden. Hij blijft het proberen; als het niet overgaat, probeer een andere server of controleer de netwerkinstellingen van uw app.';

  @override
  String get walletTransportExplainUnverified =>
      'De privacy van deze verbinding kan niet worden geverifieerd — beschouw deze als niet privé.';

  @override
  String get walletTransportExplainHostProxy =>
      'Walletverkeer wordt via de privacyverbinding van deze app geleid, waardoor uw IP-adres voor de server verborgen blijft.';

  @override
  String get walletOnboardingWelcomeTitle => 'Uw wallet instellen';

  @override
  String get walletOnboardingWelcomeBody =>
      'Maak een nieuwe wallet aan om ZEC te ontvangen en te bewaren. We genereren een herstelzin en helpen u deze stap voor stap veilig te stellen voordat er tegoeden kunnen binnenkomen — zo loopt u nooit risico zonder back-up.';

  @override
  String get walletCreateButton => 'Nieuwe wallet aanmaken';

  @override
  String get walletRestoreButton => 'Herstellen vanaf een herstelzin';

  @override
  String get walletWatchOnlyButton => 'Een wallet bekijken (alleen-inzage)';

  @override
  String get walletWatchOnlyTitle => 'Een wallet bekijken';

  @override
  String get walletWatchOnlyBody =>
      'Plak een inzagesleutel om een wallet te bekijken zonder de bijbehorende bestedingssleutels. U ziet het saldo en de geschiedenis, maar u kunt geen tegoeden verzenden. Kies de geschatte startdatum van de wallet, zodat we weten hoe ver terug we moeten kijken.';

  @override
  String get walletWatchOnlyKeyLabel => 'Inzagesleutel';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip =>
      'Scan een QR-code van de inzagesleutel';

  @override
  String get walletWatchOnlyScanTitle => 'Inzagesleutel scannen';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Richt uw camera op de QR-code van de inzagesleutel.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Camera niet beschikbaar. Plak de sleutel handmatig in plaats daarvan.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Plak in plaats daarvan';

  @override
  String get walletWatchOnlyScanHint =>
      'Of tik op de scanknop om een QR-code van de inzagesleutel te lezen.';

  @override
  String get walletWatchOnlyScanFilled => 'Inzagesleutel gescand.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Startdatum van de wallet';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Scant vanaf $date — tegoeden die daarvoor zijn ontvangen, verschijnen niet. Oudere wallet? Kies een eerdere datum.';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'Kies de startdatum van de wallet';

  @override
  String get walletWatchOnlyBirthdayChange => 'Datum wijzigen';

  @override
  String get walletWatchOnlySubmit => 'Deze wallet bekijken';

  @override
  String get walletWatchOnlyBack => 'Terug';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Dit lijkt geen geldige inzagesleutel te zijn. Controleer deze en probeer het opnieuw.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Deze inzagesleutel is voor een ander netwerk. Deze kan hier niet worden gebruikt.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'Er bestaat al een wallet op dit apparaat. Ga terug en open deze in plaats daarvan.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Deze startdatum is te recent. Kies een eerdere datum.';

  @override
  String get walletRestoreTitle => 'Uw wallet herstellen';

  @override
  String get walletRestoreBody =>
      'Voer uw herstelzin in om uw wallet te herstellen — typ of plak de woorden in de juiste volgorde, gescheiden door spaties. Alleen standaardzinnen: als uw wallet een extra wachtwoordzin gebruikte (een \"25e woord\"), kan deze app die nog niet herstellen — u zou een lege wallet zien, geen foutmelding.';

  @override
  String get walletRestorePhraseHint => 'woord een  woord twee  woord drie  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count woorden',
      one: '1 woord',
      zero: 'Nog geen woorden',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'herstelzinnen bestaan uit 12, 15, 18, 21 of 24 woorden';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count woorden zijn geen herstelwoorden — corrigeer de gemarkeerde woorden',
      one: '1 woord is geen herstelwoord — corrigeer het gemarkeerde woord',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'woord $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'woord $index: geen herstelwoord';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Woord $index verwijderen';
  }

  @override
  String get walletRestoreSubmit => 'Wallet herstellen';

  @override
  String get walletRestoreBack => 'Terug';

  @override
  String get walletRestoreBirthdayTitle => 'Hoe ver terug te scannen';

  @override
  String get walletRestoreBirthdayNone =>
      'We scannen uw hele geschiedenis — trager, maar niets wordt gemist.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Scant vanaf $date — tegoeden die daarvoor zijn ontvangen, verschijnen niet. Oudere wallet? Kies een eerdere datum, of Scan alle geschiedenis.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Kies een datum';

  @override
  String get walletRestoreBirthdayChange => 'Datum wijzigen';

  @override
  String get walletRestoreBirthdayClear => 'Scan alle geschiedenis';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'Woord $index is geen herstelwoord. Controleer uw zin op typefouten en probeer het opnieuw.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Deze herstelzin is niet geldig. Controleer de woorden en hun volgorde en probeer het opnieuw.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Deze zin komt niet overeen met de wallet op dit apparaat. Controleer de zin goed en probeer het opnieuw.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'Er bestaat al een wallet op dit apparaat. Ga terug om deze te openen.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Deze datum is te recent. Kies een eerdere datum, of scan alles.';

  @override
  String get walletGeneratingLabel => 'Uw wallet wordt aangemaakt…';

  @override
  String get walletOpeningLabel => 'Uw wallet wordt geopend…';

  @override
  String get walletBackupTitle => 'Maak een back-up van uw herstelzin';

  @override
  String get walletBackupBody =>
      'Deze woorden zijn de ENIGE manier om uw wallet en tegoeden te herstellen. Schrijf ze in de juiste volgorde op en bewaar ze op een veilige, privé plek. Deel ze nooit en bewaar ze nooit online — iedereen die deze woorden kent, kan uw tegoeden overnemen.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Schermafbeeldingen zijn uitgeschakeld op dit scherm.';

  @override
  String get walletBackupSecureNoteOther =>
      'Zorg dat niemand op uw scherm kan meekijken.';

  @override
  String get walletBackupReveal => 'Herstelzin tonen';

  @override
  String get walletBackupRevealing => 'Uw herstelzin wordt voorbereid…';

  @override
  String get walletBackupRevealFailed =>
      'Uw herstelzin kon nu niet worden getoond. Zorg dat uw apparaat ontgrendeld is en probeer het opnieuw.';

  @override
  String get walletBackupRetryReveal => 'Opnieuw proberen';

  @override
  String get walletBackupReauthFailed =>
      'Uw identiteit kon niet worden geverifieerd. Probeer het opnieuw.';

  @override
  String get walletBackupConfirmCheckbox =>
      'Ik heb mijn herstelzin opgeschreven en veilig bewaard.';

  @override
  String get walletBackupContinue => 'Doorgaan';

  @override
  String get walletBackupSaveFailed =>
      'Uw bevestiging kon niet worden opgeslagen. Probeer het opnieuw.';

  @override
  String get walletBackupStartOver => 'Opnieuw beginnen';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Zonder deze wallet opnieuw beginnen?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Dit verwijdert deze wallet van het apparaat en brengt u terug naar het begin. Er kan niets via deze app worden gestort voordat het instellen is voltooid.\n\nAls deze wallet ooit tegoeden heeft bevat — of is hersteld vanaf een herstelzin — kan alleen die zin hem terughalen.';

  @override
  String get walletBackupStartOverConfirm => 'Verwijderen en opnieuw beginnen';

  @override
  String get walletBackupStartOverKeep => 'Deze wallet behouden';

  @override
  String get walletBackupSectionTitle => 'Herstelzin';

  @override
  String get walletBackupTileTitle => 'Maak een back-up van uw herstelzin';

  @override
  String get walletBackupTileSubtitle =>
      'Toon de woorden waarmee u uw wallet en tegoeden kunt herstellen.';

  @override
  String get walletBackupScreenTitle => 'Herstelzin';

  @override
  String get walletBackupDone => 'Gereed';

  @override
  String get walletBackupManagedTitle => 'Geen aparte herstelzin';

  @override
  String get walletBackupManagedBody =>
      'Deze wallet is ingesteld met uw account uit de app die de wallet heeft geïnstalleerd en heeft daarom geen eigen herstelzin. Uw tegoeden worden samen met dat account hersteld — gebruik de back-up daarvan om ze veilig te houden.';

  @override
  String get walletExportViewingKeyTitle => 'Exporteer uw inzagesleutel';

  @override
  String get walletExportViewingKeyTileTitle => 'Exporteer uw inzagesleutel';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Deel een alleen-inzagekopie van uw wallet — deze kan uw geschiedenis zien, maar niet besteden.';

  @override
  String get walletExportViewingKeyWarning =>
      'Met deze sleutel kan iedereen die hem in bezit heeft alles zien wat deze wallet ooit heeft ontvangen en verzonden — en alles wat hij in de toekomst nog zal ontvangen en verzenden. Hij kan uw tegoeden niet besteden en kan uw wallet niet herstellen. Deel hem alleen met iemand die u vertrouwt om uw volledige geschiedenis te zien, zoals een boekhouder of uw eigen tweede apparaat. De enige manier om dit later weer ongedaan te maken, is door uw tegoeden naar een nieuwe wallet te verplaatsen.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Met deze sleutel kan iedereen die hem in bezit heeft alles zien wat deze wallet ooit heeft ontvangen en verzonden — en alles wat hij in de toekomst nog zal ontvangen en verzenden. Hij kan uw tegoeden niet besteden en kan uw wallet niet herstellen. Deel hem alleen met iemand die u vertrouwt om uw volledige geschiedenis te zien, zoals een boekhouder of uw eigen tweede apparaat. Eenmaal gedeeld, kan dit niet meer ongedaan worden gemaakt.';

  @override
  String get walletExportViewingKeyReveal => 'Inzagesleutel tonen';

  @override
  String get walletExportViewingKeyRetry => 'Opnieuw proberen';

  @override
  String get walletExportViewingKeyRevealing =>
      'Uw inzagesleutel wordt voorbereid…';

  @override
  String get walletExportViewingKeyFailed =>
      'Uw inzagesleutel kon nu niet worden getoond. Probeer het straks opnieuw.';

  @override
  String get walletExportViewingKeyQrLabel => 'QR-code van de inzagesleutel';

  @override
  String get walletExportViewingKeyCopy => 'Inzagesleutel kopiëren';

  @override
  String get walletExportViewingKeyCopied => 'Inzagesleutel gekopieerd';

  @override
  String get walletExportViewingKeyDone => 'Gereed';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Schermafbeeldingen zijn uitgeschakeld op dit scherm.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Zorg dat niemand op uw scherm kan meekijken.';

  @override
  String get walletWatchOnlySectionTitle => 'Over deze alleen-inzagewallet';

  @override
  String get walletWatchOnlyAboutBody =>
      'Dit is een alleen-inzagewallet. Deze is ingesteld vanuit een inzagesleutel, waardoor hij uw saldo en geschiedenis kan zien maar geen bestedingssleutels bezit — er is hier niets om een back-up van te maken, en hij kan geen tegoeden verzenden.';

  @override
  String get walletWatchOnlyBadge => 'Alleen-inzage';

  @override
  String get walletOnboardingFailedTitle =>
      'Instellen van wallet kon niet worden voltooid';

  @override
  String get walletOnboardingRetry => 'Opnieuw proberen';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'De beveiligde opslag van uw telefoon reageert niet. Ontgrendel uw apparaat en probeer het opnieuw. Blijft dit gebeuren, start uw telefoon dan opnieuw op.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Deze wallet is geopend in een ander venster of een andere app, of rondt nog een vorige bewerking af. Sluit eventuele andere vensters die de wallet gebruiken — of wacht even — en probeer het daarna opnieuw.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'De beveiligde sleutel van deze wallet is niet meer beschikbaar, waardoor de wallet niet op dit apparaat kan worden geopend. Uw tegoeden zijn veilig — herstel vanaf uw herstelzin om ze terug te krijgen.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Herstellen vanaf herstelzin';

  @override
  String get walletOnboardingRecoverConfirmTitle => 'Deze wallet herstellen?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Zorg dat u uw herstelzin bij de hand heeft voordat u doorgaat — u heeft deze nodig op het volgende scherm om uw tegoeden terug te krijgen. Uw tegoeden zijn veilig op de blockchain en worden beheerd door die herstelzin. Dit verwijdert de onleesbare walletgegevens van dit apparaat, zodat deze opnieuw kan worden opgebouwd.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Annuleren';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Er is niet genoeg vrije opslagruimte om uw wallet in te stellen. Maak ruimte vrij en probeer het opnieuw.';

  @override
  String get walletOnboardingFailedNoVault =>
      'Dit apparaat heeft geen beveiligde sleutelopslag, waardoor de wallet uw herstelzin hier niet kan beschermen.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Het netwerk kon tijdens het instellen niet worden bereikt. Controleer uw verbinding en probeer het opnieuw.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'Het instellen van de wallet is niet voltooid. Probeer opnieuw om dit af te ronden — er is niets verloren gegaan.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Er is iets misgegaan bij het instellen van uw wallet. Probeer het opnieuw.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'De walletinstellingen van deze app zijn onjuist, waardoor de wallet niet kan starten. Opnieuw proberen helpt niet — meld dit bij de ontwikkelaar van de app. Uw tegoeden lopen geen risico.';

  @override
  String get walletSendButton => 'Verzenden';

  @override
  String get walletSendSyncNotRunning =>
      'Synchronisatie loopt niet — uw besteedbare saldo kan niet worden bijgewerkt';

  @override
  String get walletSendWaitingForFunds =>
      'Nog aan het synchroniseren — u kunt verzenden zodra u een besteedbaar saldo heeft';

  @override
  String get walletSendNoSpendableYet => 'Nog geen besteedbaar saldo';

  @override
  String get walletSendSyncUnavailable =>
      'U kunt verzenden zodra synchronisatie wordt hervat';

  @override
  String get walletSendTitle => 'Verzenden';

  @override
  String get walletSendUnavailable =>
      'Uw wallet is nu niet gereed. Ga terug en probeer het opnieuw.';

  @override
  String get walletSendWatchOnly =>
      'Dit is een alleen-inzage wallet. Deze kan saldo\'s tonen en betalingen ontvangen, maar heeft geen bestedingssleutels — dus kan deze niet verzenden.';

  @override
  String get walletSendExpiredTitle => 'Dit betaalverzoek is verlopen';

  @override
  String get walletSendExpiredBody =>
      'Het verzendscherm deed er meer dan vijf seconden over om te openen, dus is aan de app gemeld dat er niets is verzonden. Dat antwoord is definitief: dit verzoek kan hier niet worden betaald. Begin opnieuw vanuit de app om te betalen.';

  @override
  String get walletSendFaultWatchOnly =>
      'Dit is een alleen-inzage wallet — deze heeft geen bestedingssleutels, dus kan deze niet verzenden.';

  @override
  String walletSendAvailable(String amount) {
    return 'Beschikbaar om te verzenden: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Beschikbaar om te verzenden: $amount ZEC — uw saldo is nog aan het inhalen';
  }

  @override
  String get walletSendRecipientLabel => 'Adres van ontvanger';

  @override
  String get walletSendRecipientHint => 'Zcash-adres (begint met u, z of t)';

  @override
  String get walletSendRecipientLocked =>
      'De ontvanger kan hier niet worden gewijzigd';

  @override
  String get walletSendAmountLabel => 'Bedrag (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Memo (optioneel)';

  @override
  String get walletSendMemoHint =>
      'Wordt alleen afgeleverd bij afgeschermde (privé) ontvangers';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Memo\'s vereisen een afgeschermde ontvanger. Dit openbare adres kan er geen ontvangen.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Deze betaling draagt al een referentie van de app en kan daarom geen geschreven memo bevatten.';

  @override
  String get walletSendMachineMemoTitle => 'De app voegt een referentie toe';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Volgens de app is dit voor: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Die blijft bij de transactie en kan later niet worden verwijderd. De wallet kan niet controleren wat erin staat.';

  @override
  String get walletSendRecipientShielded => 'Afgeschermd · privé';

  @override
  String get walletSendRecipientTransparent => 'Openbaar';

  @override
  String get walletSendRecipientInvalid => 'Dit lijkt geen geldig Zcash-adres.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Dit adres is voor een ander Zcash-netwerk.';

  @override
  String get walletSendReviewButton => 'Betaling controleren';

  @override
  String get walletSendQueueButton =>
      'In wachtrij zetten om later te verzenden';

  @override
  String get walletSendQueueHint =>
      'Een betaling in de wachtrij wacht onder Opgeslagen & in behandeling, waar u deze kunt verzenden of annuleren. De netwerkkosten worden berekend op het moment van verzenden.';

  @override
  String get walletSendPreparing => 'Uw betaling wordt voorbereid…';

  @override
  String get walletSendSubmitting => 'Wordt verzonden…';

  @override
  String get walletSendQueuing => 'Wordt in wachtrij gezet…';

  @override
  String get walletSendReviewTitle => 'Betaling bevestigen';

  @override
  String get walletSendTotalLabel => 'Totaal';

  @override
  String get walletSendFeeLabel => 'Netwerkkosten';

  @override
  String get walletSendChangeLabel => 'Wisselgeld geretourneerd';

  @override
  String get walletSendDeshieldTitle => 'Deze betaling is niet privé';

  @override
  String get walletSendDeshieldBody =>
      'Deze wordt verzonden naar een openbaar adres, waardoor het bedrag en de ontvanger openbaar zichtbaar zijn op de Zcash-blockchain.';

  @override
  String get walletSendPublicAckLabel =>
      'Ik begrijp dat deze betaling openbaar wordt.';

  @override
  String get walletSendConfirmButton => 'Nu verzenden';

  @override
  String get walletSendBackButton => 'Terug';

  @override
  String get walletSendSelfSendNote =>
      'U verzendt naar uw eigen wallet. De netwerkkosten zijn nog steeds van toepassing.';

  @override
  String get walletSendLargeConfirmTitle => 'Groot bedrag verzenden?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Dit is bijna uw volledige saldo. Een verzonden betaling kan niet worden teruggedraaid.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Dit is een groot bedrag. Een verzonden betaling kan niet worden teruggedraaid.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Dit is een groot bedrag — bijna uw volledige saldo. Een verzonden betaling kan niet worden teruggedraaid.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return '$amount verzenden';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Terug';

  @override
  String get walletSendSentTitle => 'Betaling verzonden';

  @override
  String get walletSendSentBody => 'Uw betaling is verzonden naar het netwerk.';

  @override
  String get walletSendSavedTitle => 'Opgeslagen — we ronden het verzenden af';

  @override
  String get walletSendSavedBody =>
      'Uw betaling kon nu niet worden verzonden, dus is deze opgeslagen en uw wallet verzendt deze bij een latere synchronisatie. Er is niets verloren gegaan.';

  @override
  String get walletSendKeptTitle => 'Opgeslagen';

  @override
  String get walletSendKeptBody =>
      'Uw wallet heeft deze transactie bewaard, maar heeft niet beloofd deze uit zichzelf te verzenden. Kijk bij Activiteit om te zien hoe het ervoor staat.';

  @override
  String get walletSendPartialBody =>
      'Een deel van uw betaling is verzonden; uw wallet rondt de rest af bij een latere synchronisatie. Er is niets verloren gegaan.';

  @override
  String get walletSendInMotionTitle => 'Betaling in uitvoering';

  @override
  String get walletSendInMotionBody =>
      'Uw betaling is gestart en gaat via een eenmalig adres dat uw wallet beheert. Verstuur deze niet opnieuw. Als deze niet wordt voltooid, kunt u de tegoeden herstellen via uw walletscherm.';

  @override
  String get walletSendAlreadyTitle => 'Al ingediend';

  @override
  String get walletSendAlreadyBody =>
      'Deze betaling is al ingediend — deze wordt niet twee keer verzonden.';

  @override
  String get walletSendFailedTitle => 'Betaling kon niet worden voltooid';

  @override
  String get walletSendFailedBody =>
      'Er is iets misgegaan bij het voltooien van deze betaling en er is niets verzonden. U kunt het opnieuw proberen.';

  @override
  String get walletSendTryAgain => 'Opnieuw proberen';

  @override
  String get walletSendDone => 'Gereed';

  @override
  String get walletSendAnother => 'Nog een verzenden';

  @override
  String get walletSendQueuedTitle => 'In wachtrij voor verzending';

  @override
  String get walletSendQueuedBody =>
      'Deze betaling is opgeslagen. U vindt deze onder Opgeslagen & in behandeling, waar u deze nu kunt verzenden of annuleren.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Onvoldoende besteedbaar saldo — u heeft $available ZEC en dit vereist $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Het Zcash-netwerk is bijgewerkt en deze app heeft een update nodig voordat er verzonden kan worden. Je tegoed is veilig.';

  @override
  String get walletSyncUpToDateLimited =>
      'Bijgewerkt tot zover deze versie kan lezen';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Het Zcash-netwerk is bijgewerkt. Deze versie heeft alles gescand wat ze kan lezen, maar nieuwere blokken kunnen tegoed bevatten dat ze nog niet kan tonen, en memo\'s bij recente betalingen zijn niet beschikbaar. Werk de app bij om alles te zien.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Bijgewerkt, maar deze server bedient niet elke pool';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Deze server weigert, houdt achter of rapporteert een van de afgeschermde Zcash-pools verkeerd. Tegoed dat in die pool is ontvangen kan niet via deze server worden uitgegeven, en het getoonde saldo is een ondergrens. Schakel over naar een andere server om het te gebruiken — dit is geen verbindingsprobleem.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: deze server weigert deze pool te leveren';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: deze server houdt een deel ervan achter';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: deze server rapporteert deze pool onjuist';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: het is onbekend of deze server deze pool levert';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Bijgewerkt met deze server, maar de server loopt achter op het netwerk';

  @override
  String get walletSyncExplainEndpointBehind =>
      'De keten van deze server stopt bij een blok dat het netwerk al was gepasseerd voordat deze versie van de app werd gebouwd, dus uw saldo is alleen actueel tot dat blok. Nieuwe betalingen aan u worden mogelijk nog niet getoond, en een betaling die vanaf hier wordt verzonden komt mogelijk niet aan. Schakel over naar een andere server om bij te werken — dit is geen verbindingsprobleem.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Wachten op een app-update — je tegoed is veilig en er is niets verzonden.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Wachten op een server die de netwerkversie meldt — wissel van server. Je tegoed is veilig en er is niets verzonden.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Wachten op een server die de netwerkversie meldt. Als de datum en tijd van dit apparaat niet kloppen, corrigeer die dan eerst — en wissel daarna van server. Je tegoed is veilig en er is niets verzonden.';

  @override
  String get walletSyncUnverified =>
      'Bijgewerkt, maar deze server meldt de netwerkversie niet';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'Verzenden werkt nog ongeveer $hours uur — wissel daarna van server.',
      one: 'Verzenden werkt nog ongeveer 1 uur — wissel daarna van server.',
      zero:
          'Verzenden werkt nog minder dan een uur — wissel daarna van server.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'Verzenden werkt nog ongeveer $blocks blokken — wissel daarna van server.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Deze server heeft de netwerkversie al $blocks blokken niet gemeld, dus deze app kan niet bevestigen dat verzenden veilig is. Wissel naar een andere server.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Deze server heeft de netwerkversie al een dag niet gemeld, dus deze app kan niet bevestigen dat verzenden veilig is. Als de datum en tijd van dit apparaat niet kloppen, corrigeer die dan eerst — en wissel daarna naar een server die de netwerkversie meldt.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Deze server heeft de netwerkversie nog nooit gemeld, dus deze app kan niet bevestigen dat verzenden veilig is. Wissel naar een andere server.';

  @override
  String get walletSyncExplainUnverified =>
      'Deze server zegt niet op welke versie van het Zcash-netwerk hij zit, dus deze app kan niet bevestigen dat een betaling die hij ondertekent wordt geaccepteerd. Je saldo is actueel. Wissel naar een andere server — dit is geen verbindingsprobleem.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Deze server zegt niet op welke versie van het Zcash-netwerk hij zit, dus deze app kan niet bevestigen dat een betaling die hij ondertekent wordt geaccepteerd. Hij is ook blokken blijven leveren die deze wallet daarna moest terugdraaien, dus je saldo is mogelijk niet actueel. Wissel naar een andere server — dit is geen verbindingsprobleem.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Deze server blijft ook blokken leveren die deze wallet daarna moet terugdraaien — wissel van server.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Uw saldo is nog aan het inhalen — er kan meer beschikbaar komen terwijl de wallet synchroniseert.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC komt nog binnen en wordt besteedbaar zodra de wallet weer bij is.';
  }

  @override
  String get walletSendFaultAmountEmpty =>
      'Voer een bedrag in om te verzenden.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Voer het bedrag in als getal, bijvoorbeeld 0.25.';

  @override
  String get walletSendFaultAmountDecimals => 'ZEC heeft maximaal 8 decimalen.';

  @override
  String get walletSendFaultAmountNotPositive =>
      'Voer een bedrag groter dan nul in.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Dat bedrag is groter dan de totale ZEC-voorraad.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Deze app beperkt verzendingen momenteel tot $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Dit lijkt geen geldig Zcash-adres voor dit netwerk. Controleer het en probeer het opnieuw.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Deze ontvanger kan geen memo ontvangen. Verwijder de memo, of verzend naar een afgeschermd (privé) adres.';

  @override
  String get walletSendFaultMemoTooLong =>
      'Uw memo is te lang. Maak deze korter en probeer het opnieuw.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Deze memo kan niet worden verzonden. Verwijder deze en probeer het opnieuw.';

  @override
  String get walletSendFaultMemoConflict =>
      'Deze betaling kon niet worden verzonden – de app heeft er twee notities aan gekoppeld. Er is niets verzonden.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Dit adres is voor een ander netwerk.';

  @override
  String get walletSendFaultUriInvalid =>
      'Deze betaling kon niet worden opgebouwd. Controleer het adres en het bedrag.';

  @override
  String get walletSendFaultNotSynced =>
      'Uw wallet is nog niet ver genoeg gesynchroniseerd. Wacht tot de synchronisatie is bijgewerkt, of zet dit in de wachtrij om later te verzenden.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Uw wallet is nog niet ver genoeg gesynchroniseerd. Wacht tot de synchronisatie is bijgewerkt.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Uw wallet is nog niet ver genoeg gesynchroniseerd, en synchronisatie loopt nu niet. Controleer de synchronisatiestatus op het walletscherm.';

  @override
  String get walletSendFaultAmountsExpired =>
      'De bedragen zijn verlopen terwijl u aan het controleren was. Controleer de betaling opnieuw.';

  @override
  String get walletSendFaultQueueFull =>
      'Er wachten te veel verzendingen om uit te gaan. Laat deze eerst verzenden en probeer het daarna opnieuw.';

  @override
  String get walletSendFaultWalletBusy =>
      'De wallet is momenteel bezet. Probeer het straks opnieuw.';

  @override
  String get walletSendFaultStorageFull =>
      'Er is niet genoeg vrije opslagruimte om deze verzending te voltooien. Maak ruimte vrij en probeer het opnieuw.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Er zijn nu te veel eenmalige adressen in gebruik. Sommige kunnen vrijkomen zodra transacties worden bevestigd, maar dit lost mogelijk niet vanzelf op. Uw tegoeden zijn veilig.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Deze betaling kon niet worden voorbereid. Controleer de gegevens en probeer het opnieuw.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Deze betaling kon zojuist niet worden voorbereid. Probeer het over een ogenblik opnieuw.';

  @override
  String get walletSwapButton => 'Wisselen';

  @override
  String get walletSwapTitle => 'ZEC wisselen';

  @override
  String get walletSwapUnavailableWallet =>
      'Uw wallet is nu niet gereed. Ga terug en probeer het opnieuw.';

  @override
  String get walletSwapUnavailableOff => 'Wisselen is nu niet beschikbaar.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Dit is een alleen-inzage wallet — deze kan niet wisselen.';

  @override
  String get walletSwapDone => 'Gereed';

  @override
  String get walletSwapBackToWallet => 'Terug naar de wallet';

  @override
  String walletSwapAvailable(String amount) {
    return 'Beschikbaar om te wisselen: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Beschikbaar om te wisselen: $amount ZEC — uw saldo is nog aan het inhalen';
  }

  @override
  String get walletSwapAssetLabel => 'Te ontvangen munt';

  @override
  String get walletSwapAmountLabel => 'Te wisselen bedrag (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Bestemmingsadres';

  @override
  String get walletSwapDestinationHint =>
      'Uw ontvangstadres op het bestemmingsnetwerk';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Uw $chain-ontvangstadres';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Een $chain-adres — waar uw gewisselde munt naartoe wordt gestuurd. Controleer goed of het juiste netwerk is gekozen.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Scan een QR-code van het bestemmingsadres';

  @override
  String get walletSwapTargetAssetHint => 'Selecteer een te ontvangen munt';

  @override
  String get walletSwapQuoteButton => 'Koers opvragen';

  @override
  String get walletSwapQuoting => 'Koers wordt opgevraagd…';

  @override
  String get walletSwapExecuting => 'Uw wissel wordt gestart…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Nog bezig — de wissel wordt gestart. Dit kan tot een minuut duren.';

  @override
  String get walletSwapReviewTitle => 'Wissel bevestigen';

  @override
  String get walletSwapYouSendLabel => 'U verzendt';

  @override
  String get walletSwapYouReceiveLabel => 'U ontvangt minimaal';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Netwerkkosten';

  @override
  String get walletSwapNetworkFeeValue =>
      'Toegevoegd bij het verzenden van de storting';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Koers nog ongeveer $time geldig — bevestig voordat deze verloopt.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Koers nog minder dan een minuut geldig — bevestig voordat deze verloopt.';

  @override
  String get walletSwapQuoteExpired =>
      'Deze koers is verlopen. Ga terug en vraag een nieuwe koers op — de oude wordt niet meer gegarandeerd, en nu verzenden kan leiden tot een terugbetaling.';

  @override
  String get walletCountdownUnderMinute => 'minder dan een minuut';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds sec';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '$hours u $minutes min';
  }

  @override
  String get walletSwapDeshieldTitle => 'Deze wissel is niet privé';

  @override
  String get walletSwapDeshieldBody =>
      'Wisselen vanuit ZEC ontschermt uw ZEC — de storting is een openbare transactie, en de kant van de aanbieder is openbaar op diens netwerk.';

  @override
  String get walletSwapDiscloseTitle => 'Wat de wisselaanbieder zal zien';

  @override
  String get walletSwapDiscloseAmounts => 'De bedragen aan beide zijden';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Dat deze ZEC en de munt die u ontvangt bij dezelfde wissel horen';

  @override
  String get walletSwapDiscloseDestination => 'Uw bestemmingsadres';

  @override
  String get walletSwapDiscloseSource => 'Uw bronadres';

  @override
  String get walletSwapDiscloseIp => 'Uw IP-adres (tenzij u via Tor verbindt)';

  @override
  String get walletSwapDiscloseGeneric => 'Overige gegevens van deze wissel';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'De eigen transacties van de aanbieder zijn openbaar op diens netwerk';

  @override
  String get walletSwapAckLabel =>
      'Ik begrijp dat de aanbieder bovenstaande gegevens zal zien.';

  @override
  String get walletSwapConfirmButton => 'Wissel starten';

  @override
  String get walletSwapBackButton => 'Terug';

  @override
  String get walletSwapStatusPendingTitle => 'Wissel gestart';

  @override
  String get walletSwapStatusCheckingTitle => 'Wisselstatus controleren…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Uw wallet verstuurt de ZEC-storting naar de aanbieder. Bent u kort offline, dan wordt deze automatisch verzonden zodra u weer online bent — maar het verzendvenster is kort, en sluit dit eerder, dan stopt de wissel gewoon en wordt er niets gewisseld. Uw ZEC blijft van u, al kan het tot een uur duren voordat het weer als besteedbaar wordt weergegeven.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'Wachten tot uw storting binnenkomt. Hebt u het geld nog niet vanuit uw andere wallet verzonden, doe dit dan voordat de koers verloopt.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Deze wissel wacht nog op de storting. De stortingsinstructies zijn niet meer beschikbaar op dit apparaat — hebt u het geld al verzonden, dan wordt dit gedetecteerd; zo niet, laat deze wissel dan verlopen en start een nieuwe.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'Het stortingsvenster eindigt $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'Het stortingsvenster is verstreken. Als de storting niet op tijd is verzonden, stopt de wissel en blijft uw ZEC in uw wallet.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'Het stortingsvenster is verstreken. Als u uw storting nog niet hebt verzonden, stopt deze wissel gewoon — vraag een nieuwe koers aan wanneer u zover bent.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Wissels lopen',
      one: 'Wissel loopt',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'Uw ZEC is onderweg naar de aanbieder.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'Wachten tot uw storting de aanbieder bereikt.';

  @override
  String get walletSwapInFlightRowGeneric => 'Er loopt een wissel.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'Het stortingsvenster is verstreken — controleer de status van deze wissel.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Deze wissel heeft hier nog geen bevestigde uitkomst bereikt — open hem om te controleren. ZEC die terugkomt naar deze wallet, verschijnt na een synchronisatie in uw saldo.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Deze wissel heeft hier nog geen bevestigde uitkomst bereikt — open hem om te controleren. ZEC die deze wissel aan deze wallet levert, verschijnt na een synchronisatie in uw saldo.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Wissel voltooid.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Wissel terugbetaald.';

  @override
  String get walletSwapRowOutcomeFailed => 'Wissel niet voltooid.';

  @override
  String get walletSwapRemove => 'Verwijderen';

  @override
  String get walletSwapRemoveTitle => 'Deze wissel uit de lijst verwijderen?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Dit verwijdert de wissel alleen uit deze lijst — de wissel wordt niet geannuleerd, en deze wallet stopt met het volgen van de terugbetaling ervan. Later terugbetaalde ZEC blijft eigendom van deze wallet; een volledige nieuwe scan kan het terugvinden.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Dit verwijdert de wissel alleen uit deze lijst — de wissel wordt niet geannuleerd, en deze wallet stopt met het volgen van binnenkomende ZEC. Later geleverde ZEC blijft eigendom van deze wallet; een volledige nieuwe scan kan het terugvinden. Als de wissel in plaats daarvan wordt terugbetaald, gaat de terugbetaling terug in de munt die u hebt verzonden, buiten deze wallet.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Dit verwijdert de wissel alleen uit deze lijst — de wissel wordt niet geannuleerd, en deze wallet stopt met het volgen van ZEC die er nog vanuit onderweg is. Later binnenkomende ZEC blijft eigendom van deze wallet; een volledige nieuwe scan kan het terugvinden.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Dit verwijdert de afgeronde wissel uit de lijst.';

  @override
  String get walletSwapRemoveCancel => 'Annuleren';

  @override
  String get walletSwapRemoveConfirm => 'Verwijderen';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Gestart: $time';
  }

  @override
  String get walletSwapViewSwap => 'Wissel weergeven';

  @override
  String get walletSwapsInFlightError =>
      'Uw lopende wissels konden nu niet worden geladen.';

  @override
  String get walletSwapsInFlightRetry => 'Opnieuw proberen';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Wordt geprobeerd…';

  @override
  String get walletSwapStartAnother => 'Nog een wissel starten';

  @override
  String get walletSwapStatusUnderTitle => 'Wachten op de volledige storting';

  @override
  String get walletSwapStatusUnderBody =>
      'Een deel van de storting is binnengekomen. De rest wordt voltooid, of de aanbieder betaalt terug.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Een deel van uw storting is binnengekomen. Verzend het ontbrekende bedrag vóór de deadline, anders betaalt de aanbieder het binnengekomen bedrag terug.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Ontvangen: $received; nog $missing ontbrekend. Het stortingsvenster eindigt: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Storting ontvangen';

  @override
  String get walletSwapStatusDetectedBody =>
      'De aanbieder heeft uw storting ontvangen en verwerkt de wissel.';

  @override
  String get walletSwapStatusProcessingTitle => 'Uw wissel wordt verwerkt';

  @override
  String get walletSwapStatusProcessingBody =>
      'De aanbieder rondt uw wissel af.';

  @override
  String get walletSwapStatusSuccessTitle => 'Wissel voltooid';

  @override
  String get walletSwapStatusSuccessBody => 'Uw wissel is succesvol afgerond.';

  @override
  String get walletSwapStatusRefundedTitle => 'Wissel terugbetaald';

  @override
  String get walletSwapStatusRefundedBody =>
      'De wissel is niet voltooid, dus heeft de aanbieder het tegoed teruggestort naar uw terugbetalingsadres.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'De wissel is niet voltooid, dus heeft de aanbieder uw ZEC teruggestuurd naar deze wallet. Dit komt binnen als niet-afgeschermd tegoed en verschijnt in uw saldo na de volgende synchronisatie van de wallet — dit kan even duren.';

  @override
  String get walletSwapStatusFailedTitle => 'Wissel mislukt';

  @override
  String get walletSwapStatusFailedBody =>
      'De wissel kon niet worden voltooid. Eventueel gestorte tegoeden worden bij de aanbieder verrekend of terugbetaald.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Wissel niet gevonden';

  @override
  String get walletSwapStatusNotFoundBody =>
      'De aanbieder heeft geen registratie meer van deze wissel — deze is hoogstwaarschijnlijk verlopen. Als er een storting is gedaan, zou de aanbieder deze moeten terugbetalen naar het terugbetalingsadres. De wissel blijft in uw lijst staan en deze wallet blijft de ZEC ervan volgen voor het geval deze nog aankomt; u kunt hem op elk moment uit de lijst verwijderen.';

  @override
  String get walletSwapStatusUnknownTitle => 'Status niet beschikbaar';

  @override
  String get walletSwapStatusUnknownBody =>
      'De status van deze wissel kan nu niet worden gelezen.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'Volgen niet beschikbaar';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'Wisselen is uitgeschakeld, dus dit kan hier niet worden gevolgd. Eventuele tegoeden worden bij de aanbieder verrekend of terugbetaald.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'Wisselen is hier uitgeschakeld, dus deze wissel kan nu niet worden gevolgd. Als deze is terugbetaald, komt de ZEC terug naar deze wallet — dit verschijnt in uw saldo nadat wisselen weer is ingeschakeld en de wallet synchroniseert.';

  @override
  String get walletSwapTrackingError => 'Deze wissel kon niet worden gevolgd.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Het volgen van deze wissel kon niet worden geopend. De wissel zelf loopt mogelijk nog steeds door — eventueel gestorte tegoeden worden bij de aanbieder verrekend of terugbetaald.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Voer het adres in waar u de gewisselde munt wilt ontvangen.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Dit bestemmingsadres is niet geldig voor deze munt. Controleer het en probeer het opnieuw.';

  @override
  String get walletSwapFaultExpired =>
      'Deze koers is verlopen. Vraag een nieuwe koers op om door te gaan.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'De prijs van de aanbieder is buiten uw limiet bewogen, waardoor de wissel is gestopt voordat er iets is verplaatst. Probeer het opnieuw.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'De slippagelimiet is te hoog voor een veilige wissel. Probeer het opnieuw.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'De wisselaanbieder is nu niet beschikbaar. Probeer het straks opnieuw.';

  @override
  String get walletSwapFaultConnection =>
      'De wisseldienst kon niet worden bereikt. Controleer uw internetverbinding en probeer het opnieuw.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'De wisselaanbieder gaf een onverwachte reactie, waardoor de wissel is gestopt. Probeer het opnieuw.';

  @override
  String get walletSwapFaultSwapOff => 'Wisselen is momenteel uitgeschakeld.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Uw storting kon niet worden verzonden, dus is er niets uit uw wallet gegaan. Vraag een nieuwe koers op om het opnieuw te proberen.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Er loopt al een wissel. U kunt een nieuwe starten nadat deze volledig is verrekend of de koers is verlopen — dit kan een tijdje duren.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Deze wallet kan nog geen terugbetalingsadres instellen — dit betekent meestal alleen dat de eerste synchronisatie nog niet is voltooid. Wacht tot de synchronisatie is voltooid en probeer het daarna opnieuw.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Deze wallet kan nog geen ontvangstadres voor deze wissel instellen — dit betekent meestal alleen dat de eerste synchronisatie nog niet is voltooid. Wacht tot de synchronisatie is voltooid en probeer het daarna opnieuw.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'De wissel kon niet op tijd starten — de verbinding was mogelijk traag, of de wallet was bezet. Vraag een nieuwe koers op en probeer het opnieuw.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'De wallet is even bezig. Probeer het opnieuw.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Deze koers komt niet overeen met de koers die uw wallet heeft afgegeven, dus er is niets verzonden. Vraag een nieuwe koers op en probeer het opnieuw.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Deze wissel vereist ongeveer $needed ZEC inclusief de netwerkkosten, maar er is nu slechts $spendable ZEC besteedbaar.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Deze app beperkt wissels momenteel tot $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Deze wissel vereist ongeveer $needed ZEC inclusief de netwerkkosten, maar er is nu slechts $spendable ZEC besteedbaar. Uw saldo is nog aan het inhalen — er kan binnenkort meer besteedbaar worden.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'De wallet kon deze wissel niet veilig registreren, dus is er niets verplaatst. Probeer het opnieuw.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Dit wisselverzoek kon niet worden verwerkt. Vraag een nieuwe koers op en probeer het opnieuw.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Er kon geen wisselkoers worden opgevraagd. Controleer de gegevens en probeer het opnieuw.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Uw wallet is nu niet gereed. Ga terug en probeer het opnieuw.';

  @override
  String get walletSwapDirectionBuy => 'ZEC kopen';

  @override
  String get walletSwapDirectionSell => 'ZEC verkopen';

  @override
  String get walletSwapRefundLabel => 'Uw terugbetalingsadres';

  @override
  String get walletSwapRefundHint =>
      'Waar uw munten naar teruggaan als de wissel mislukt';

  @override
  String get walletSwapRefundHelper =>
      'Op het netwerk waarvandaan u verzendt — geen Zcash-adres.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Uw $chain-terugbetalingsadres';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Een $chain-adres — waar uw munten naar teruggaan als de wissel mislukt. Geen Zcash-adres.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'Over uw terugbetalingsadres';

  @override
  String get walletSwapRefundInfoBody =>
      'Als de wissel niet kan worden voltooid, stuurt de aanbieder uw munten terug naar dit adres op het netwerk waarvandaan u betaalde. Voer een adres in dat u zelf beheert — de wallet kan een extern adres niet voor u controleren, dus verifieer het zorgvuldig.';

  @override
  String get walletSwapRefundScanTooltip =>
      'Scan een QR-code van het terugbetalingsadres';

  @override
  String get walletSwapScanTitle => 'Adres scannen';

  @override
  String get walletSwapScanInstruction =>
      'Richt uw camera op de QR-code van het adres.';

  @override
  String get walletSwapScanManualEntry => 'Handmatig invoeren';

  @override
  String get walletSwapScanCancel => 'Annuleren';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Camera niet beschikbaar. Voer het adres hieronder handmatig in.';

  @override
  String get walletSwapSourceAssetLabel => 'Munt om vanuit te wisselen';

  @override
  String get walletSwapSourceAssetHint => 'Selecteer een munt';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Te verzenden bedrag ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Te verzenden bedrag';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol op $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Kies een munt om vanuit te wisselen';

  @override
  String get walletSwapPickerTitleReceive => 'Kies een te ontvangen munt';

  @override
  String get walletSwapPickerStale =>
      'De muntenlijst kon niet worden ververst — laatst bekende lijst wordt getoond.';

  @override
  String get walletSwapPickerEmpty =>
      'Er zijn nu geen munten beschikbaar om te wisselen. Probeer het later opnieuw.';

  @override
  String get walletSwapPickerSearchHint => 'Zoek op naam of netwerk';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Geen munten komen overeen met \"$query\".';
  }

  @override
  String get walletSwapPickerError =>
      'De muntenlijst kon niet worden geladen. Controleer uw verbinding en probeer het opnieuw.';

  @override
  String get walletSwapPickerRetry => 'Opnieuw proberen';

  @override
  String get walletSwapSlippageLabel => 'Slippagetolerantie';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Aangepast';

  @override
  String get walletSwapSlippageCustomLabel => 'Aangepaste slippage';

  @override
  String get walletSwapSlippageMayFail =>
      'Zeer laag — de wissel kan mislukken als de prijs verandert.';

  @override
  String get walletSwapSlippageNormal => 'Een veilige tolerantie.';

  @override
  String get walletSwapSlippageRisky =>
      'Hoog — u kunt merkbaar minder ontvangen dan de opgegeven koers.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Te hoog — de wissel wordt geweigerd. Verlaag naar 10% of minder.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'U ontvangt minimaal $zec ZEC — uw ondergrens bij $slippage% slippage. Het uiteindelijke bedrag komt hier niet onder.';
  }

  @override
  String get walletSwapIntoZecShieldTitle => 'U ontvangt ZEC op uw eigen adres';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Totdat u het afschermt — één tik, waarop u bij aankomst wordt geattendeerd — is het ontvangen bedrag kort openbaar en zichtbaar op de blockchain. Een klein bedrag kan openbaar blijven totdat het is opgebouwd.';

  @override
  String get walletSwapRefundVerifyTitle => 'Verifieer uw terugbetalingsadres';

  @override
  String get walletSwapRefundVerifyBody =>
      'Controleer het teken voor teken — hier komen uw munten terug als de wissel mislukt. De wallet kan een extern adres niet voor u verifiëren.';

  @override
  String get walletSwapRefundVerifyAck =>
      'Ik heb gecontroleerd dat mijn terugbetalingsadres juist is.';

  @override
  String get walletSwapPayoutVerifyTitle => 'Verifieer uw ontvangstadres';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Controleer het teken voor teken — hier ontvangt u $asset. De wallet kan een extern adres niet voor u verifiëren.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'Ik heb gecontroleerd dat mijn ontvangstadres juist is.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'Wisselen is hier uitgeschakeld. ZEC die al onderweg is, verschijnt in uw wallet na uw volgende synchronisatie.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Voer het bedrag in dat u wilt wisselen.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Voer uw terugbetalingsadres op het bronnetwerk in.';

  @override
  String get walletSwapDepositTitle => 'Verzend uw betaling';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Verzend precies $amount $asset op $chain naar het onderstaande adres.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Verzend het exacte bedrag. Verzendt u minder, of verzendt u nadat het venster is gesloten, dan betaalt de aanbieder u terug op uw terugbetalingsadres.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Stortingsvenster: nog $time';
  }

  @override
  String get walletSwapDepositExpired =>
      'Dit stortingsvenster is gesloten. Verzend nu geen tegoeden — start een nieuwe wissel. Heeft u al verzonden, dan zou de aanbieder moeten terugbetalen op uw terugbetalingsadres.';

  @override
  String get walletSwapDepositQrLabel => 'QR-code van het stortingsadres';

  @override
  String get walletSwapDepositAddressLabel => 'Stortingsadres';

  @override
  String get walletSwapDepositCopy => 'Stortingsadres kopiëren';

  @override
  String get walletSwapDepositCopied => 'Stortingsadres gekopieerd';

  @override
  String get walletSwapDepositMemoRequired =>
      'Deze storting vereist een memo/tag';

  @override
  String get walletSwapDepositMemoWarning =>
      'U MOET deze exacte memo bij uw storting vermelden. Verzenden zonder memo — of met de verkeerde memo — kan uw tegoeden permanent verloren doen gaan.';

  @override
  String get walletSwapDepositMemoLabel => 'Stortingsmemo/-tag';

  @override
  String get walletSwapDepositMemoCopy => 'Memo kopiëren';

  @override
  String get walletSwapDepositMemoCopied => 'Memo gekopieerd';

  @override
  String get walletSwapDepositSent => 'Ik heb de tegoeden verzonden';

  @override
  String get walletSwapDepositBackTitle => 'Dit scherm verlaten?';

  @override
  String get walletSwapDepositBackBody =>
      'Dit annuleert uw wissel niet — deze gaat op de achtergrond door. Maar u heeft het stortingsadres nodig om te betalen, dus kopieer dit eerst als u dat nog niet heeft gedaan.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Dit annuleert uw wissel niet — deze gaat op de achtergrond door. Het stortingsvenster is gesloten. Verzend nu geen tegoeden naar het stortingsadres. Heeft u al verzonden, dan zou de aanbieder moeten terugbetalen op uw terugbetalingsadres.';

  @override
  String get walletSwapDepositBackStay => 'Blijven';

  @override
  String get walletSwapDepositBackLeave => 'Verlaten';

  @override
  String get walletReceive => 'Ontvangen';

  @override
  String get walletReceiveSubtitle =>
      'Deel dit adres om ZEC te ontvangen. Het is veilig om openbaar te delen.';

  @override
  String get walletReceiveCopy => 'Adres kopiëren';

  @override
  String get walletReceiveCopied => 'Adres gekopieerd';

  @override
  String get walletReceiveUnavailable => 'Uw wallet is nog niet gereed.';

  @override
  String get walletReceiveError =>
      'Uw adres kon niet worden geladen. Probeer het opnieuw.';

  @override
  String get walletReceivePreparing => 'Uw adres wordt voorbereid…';

  @override
  String get walletReceivePreparingHint =>
      'Uw wallet bereidt dit adres voor op uw apparaat — dit kan even duren als de wallet bezig is met ander werk.';

  @override
  String get walletReceiveRetry => 'Opnieuw proberen';

  @override
  String get walletReceiveQrLabel => 'QR-code van uw ontvangstadres';

  @override
  String get walletReceiveTypeShielded => 'Afgeschermd';

  @override
  String get walletReceiveTypeTransparent => 'Openbaar';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Deel dit openbare adres om ZEC te ontvangen van een afzender die niet naar een afgeschermd adres kan betalen.';

  @override
  String get walletReceiveTransparentWarning =>
      'Dit is een openbaar adres: het is zichtbaar op de blockchain en koppelt uw betalingen aan elkaar bij hergebruik. Geef de voorkeur aan uw afgeschermde adres; scherm deze tegoeden na ontvangst af.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'QR-code van uw openbare ontvangstadres';

  @override
  String get walletReceiveFreshAddress => 'Nieuw adres gebruiken';

  @override
  String get walletReceiveFreshCaption =>
      'Nieuw adres — kan niet worden gekoppeld aan uw andere adressen. Betalingen hiernaartoe komen nog steeds in deze wallet aan, en uw eerdere adressen blijven werken. Het wordt hier niet opnieuw getoond — kopieer het nu.';

  @override
  String get walletReceiveFreshError =>
      'Er kon geen nieuw adres worden aangemaakt. Probeer het opnieuw.';

  @override
  String get walletReceiveFreshBusy =>
      'De wallet is op dit moment bezig. Probeer het nieuwe adres over een moment opnieuw.';

  @override
  String get walletReceiveShare => 'Delen';

  @override
  String get walletReceiveRequestAmount => 'Bedrag aanvragen';

  @override
  String get walletReceiveRequestAmountLabel => 'Bedrag (optioneel)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Het wordt hier niet opnieuw getoond — kopieer het nu.';

  @override
  String get walletSecurityMenuItem => 'Beveiliging…';

  @override
  String get securityTitle => 'Beveiliging';

  @override
  String get securityUnavailableBody =>
      'Beveiligingsinstellingen van de wallet worden beheerd door deze app, niet door de wallet zelf.';

  @override
  String get securityCustodySectionTitle => 'Sleutelbeheer';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (hardware)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (hardware)';

  @override
  String get securityCustodyTierTee => 'Hardware-sleutelopslag (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Software-sleutelopslag';

  @override
  String get securityCustodyTierKeychain => 'Keychain (software-versleuteld)';

  @override
  String get securityCustodyTierNone => 'Geen hardware-sleutelopslag';

  @override
  String get securityCustodyTierUnknown => 'Onbekend';

  @override
  String get securityCustodyHardwareKey =>
      'De sleutel die deze wallet vergrendelt, staat in de beveiligde hardware van dit apparaat en wordt met de wallet verwijderd.';

  @override
  String get securityCustodyBestEffort =>
      'Verwijderen haalt uw sleutels naar beste vermogen weg; een kort forensisch hersteltijdvenster kan blijven bestaan totdat het apparaat de opslagruimte opnieuw in gebruik neemt. Gebruik voor volledige zekerheid ook de functie voor het volledig wissen van uw apparaat.';

  @override
  String get securityCustodyProbeError =>
      'De status van het sleutelbeheer kon niet worden gelezen. Ga terug en probeer het opnieuw.';

  @override
  String get securityDeleteWalletButton => 'Wallet verwijderen';

  @override
  String get securityDeleteWalletSubtitle =>
      'Deze wallet en de sleutel ervan van dit apparaat verwijderen. Uw tegoeden blijven op de blockchain en zijn herstelbaar vanaf uw herstelzin.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Deze wallet en de sleutel ervan van dit apparaat verwijderen. Hij bezit geen bestedingssleutels, dus er is niets om een back-up van te maken — voeg hem op elk moment opnieuw toe met zijn inzagesleutel.';

  @override
  String get securityDeleteDialogTitle => 'Deze wallet verwijderen?';

  @override
  String get securityDeleteDialogBody =>
      'Dit verwijdert de wallet en de sleutel ervan van dit apparaat. Zorg dat u een back-up van uw herstelzin heeft gemaakt — dit is de ENIGE manier om uw tegoeden te herstellen.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Dit verwijdert de wallet en de sleutel ervan van dit apparaat. Hij bezit geen bestedingssleutels, dus er is niets om een back-up van te maken — u kunt hem later opnieuw toevoegen met zijn inzagesleutel.';

  @override
  String get securityDeleteDialogConfirm => 'Verwijderen';

  @override
  String get securityDeleteDialogCancel => 'Annuleren';

  @override
  String get securityDeleteFailedSnack =>
      'De wallet kon niet worden verwijderd — uw wallet is ongewijzigd. Probeer het opnieuw.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Rond eerst de serverwissel af — die wordt binnen $seconds seconden voltooid of gestopt. Probeer daarna opnieuw de wallet te verwijderen.';
  }

  @override
  String get walletParkedTitle => 'Opgeslagen & in behandeling';

  @override
  String get walletParkedSubtitle =>
      'Deze betalingen zijn nog niet verzonden. De bedragen maken nog steeds deel uit van uw saldo.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Deze betalingen zijn nog niet verzonden. De bedragen maken nog steeds deel uit van uw saldo — behalve die uw wallet aan het verzenden is, waarvan het bedrag mogelijk al is toegewezen.';

  @override
  String get walletParkedCancel => 'Annuleren';

  @override
  String get walletParkedPausedHint =>
      'Gepauzeerd — deze betaling wordt niet vanzelf verzonden. Uw tegoeden zijn veilig. Verzend deze nu, of annuleer de betaling.';

  @override
  String get walletParkedRetryStale =>
      'Deze betaling wacht niet meer. Controleer uw betalingen in behandeling en activiteit.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Deze betaling wacht niet meer — uw wallet is deze mogelijk al aan het verzenden. Controleer Opgeslagen & in behandeling en uw activiteit.';

  @override
  String get walletReclaimExplainer =>
      'Verzenden via eenmalige adressen zit vast. U kunt dit heropenen — dit verplaatst een klein bedrag tussen uw eigen adressen, dat vervolgens weer terugkomt.';

  @override
  String get walletReclaimButton => 'Verzenden heropenen';

  @override
  String get walletReclaimInProgress => 'Wordt heropend…';

  @override
  String get walletReclaimConfirmTitle =>
      'Verzenden via eenmalige adressen heropenen?';

  @override
  String get walletReclaimConfirmBody =>
      'Dit verplaatst een klein bedrag tussen uw eigen adressen om verzenden via eenmalige adressen weer vrij te maken, en dat keert vervolgens terug. Dit kost u een paar keer de netwerkkosten. Zodra dit is bevestigd, herstelt u het verplaatste bedrag via Nu herstellen.';

  @override
  String get walletReclaimConfirmCancel => 'Niet nu';

  @override
  String get walletReclaimConfirmAction => 'Heropenen';

  @override
  String get walletReclaimStarted =>
      'Heropenen gestart. Zodra dit is bevestigd, verzendt u de gepauzeerde betaling en herstelt u vervolgens het verplaatste bedrag via Nu herstellen.';

  @override
  String get walletReclaimNothing => 'Er is nu niets om te heropenen.';

  @override
  String get walletReclaimNotBroadcast =>
      'Kon niet bevestigen dat dit het netwerk heeft bereikt. Het kan alsnog doorgaan. Probeer het straks opnieuw.';

  @override
  String get walletReclaimNeedsFunds =>
      'U heeft wat afgeschermde ZEC nodig om verzenden te heropenen.';

  @override
  String get walletReclaimFailed =>
      'Kon verzenden nu niet heropenen. Uw tegoeden zijn ongewijzigd. Probeer het opnieuw.';

  @override
  String get walletReclaimUnknown =>
      'Heropenen voltooid. Controleer uw verzendingen via eenmalige adressen en herstel een eventueel verplaatst bedrag via Nu herstellen.';

  @override
  String get walletParkedError =>
      'Uw betalingen in behandeling konden nu niet worden geladen.';

  @override
  String get walletParkedErrorRetry => 'Opnieuw proberen';

  @override
  String get walletParkedErrorRetryInProgress => 'Wordt geprobeerd…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Deze betaling in behandeling annuleren?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Dit verwijdert de opgeslagen betaling. Deze is nog niet verzonden, dus er gaat niets uit uw wallet — maar dit kan niet ongedaan worden gemaakt.';

  @override
  String get walletParkedCancelConfirmKeep => 'Behouden';

  @override
  String get walletParkedCancelConfirmDiscard => 'Betaling verwijderen';

  @override
  String get walletParkedCancelDone => 'Betaling in behandeling geannuleerd.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Deze betaling is mogelijk al onderweg — controleer uw activiteit.';

  @override
  String get walletParkedCancelFailed =>
      'Kon nu niet annuleren. Uw betaling is ongewijzigd. Probeer het opnieuw.';

  @override
  String get walletRecoverNow => 'Nu herstellen';

  @override
  String get walletRecoverConfirmTitle =>
      'Herstellen naar uw afgeschermde saldo?';

  @override
  String get walletRecoverConfirmBody =>
      'Dit controleert uw eenmalige adressen en verplaatst alles wat wordt gevonden naar uw privé, afgeschermde saldo. Dit kan altijd veilig opnieuw worden uitgevoerd.';

  @override
  String get walletRecoverConfirmCancel => 'Niet nu';

  @override
  String get walletRecoverConfirmAction => 'Herstellen';

  @override
  String get walletRecoverInProgress => 'Wordt hersteld…';

  @override
  String walletRecoverDone(String amount) {
    return '$amount wordt hersteld naar uw afgeschermde saldo.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return '$amount wordt hersteld — voor sommige tegoeden is nog een nieuwe poging nodig.';
  }

  @override
  String get walletRecoverRetry =>
      'Voor sommige tegoeden is nog een poging nodig — voer herstel opnieuw uit.';

  @override
  String get walletRecoverTruncated =>
      'Nog niet elk eenmalig adres is gecontroleerd — voer het opnieuw uit om de rest te controleren.';

  @override
  String get walletRecoverNothing => 'Er is nu niets om te herstellen.';

  @override
  String get walletRecoverFailed =>
      'Kon nu niet herstellen. Uw tegoeden zijn ongewijzigd. Probeer het opnieuw.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount opgeslagen & in behandeling · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'De betaling van $amount opgeslagen $time annuleren';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount gepauzeerd · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount wordt verzendklaar gemaakt · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Uw wallet maakt deze betaling verzendklaar — het bedrag kan al zijn toegewezen. Uw tegoeden zijn veilig. Als dit niet lukt, keert de betaling vanzelf terug naar de lijst.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Uw wallet maakt deze betaling verzendklaar — het bedrag kan al zijn toegewezen. Uw tegoeden zijn veilig, maar dit kan pas worden voltooid zodra uw wallet weer synchroniseert.';

  @override
  String get walletParkedSendNow => 'Nu verzenden';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'De betaling van $amount opgeslagen $time wordt verzonden';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'De betaling van $amount opgeslagen $time nu verzenden';
  }

  @override
  String get walletParkedSendNowInProgress => 'Wordt verzonden…';

  @override
  String get walletParkedAuthorizeSent => 'Uw betaling wordt nu verzonden.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Uw betaling wordt nu verzonden. Lukt dit niet, dan kan uw wallet dit pas voltooien zodra deze weer synchroniseert.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Nog niet klaar om te verzenden. Uw betaling is opgeslagen en ongewijzigd.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Nog niet klaar om te verzenden. Uw betaling is opgeslagen en niet meer gepauzeerd — probeer het later opnieuw via Nu verzenden, of annuleer de betaling.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Kon nu niet verzenden. Uw betaling is ongewijzigd. Probeer het opnieuw.';

  @override
  String get walletTransparentFundsMenuItem => 'Openbare tegoeden…';

  @override
  String get walletTransparentFundsTitle => 'Openbare tegoeden';

  @override
  String get walletTransparentFundsIntro =>
      'Openbare tegoeden zijn openbaar zichtbaar op de blockchain — het bedrag, de adressen en de geschiedenis van de munten.';

  @override
  String get walletExpertToggleLabel => 'Geavanceerd: openbare tegoeden';

  @override
  String get walletExpertToggleDescription =>
      'Toon geavanceerde instellingen voor het aanhouden van openbare tegoeden en het uitschakelen van automatisch afschermen.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Toon geavanceerde instellingen voor het aanhouden van openbare tegoeden.';

  @override
  String get walletAutoShieldToggleLabel => 'Automatisch afschermen';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Zodra uw openbare saldo $minZec ZEC bereikt, wordt dit automatisch verplaatst naar uw afgeschermde saldo. Als dit is uitgeschakeld, blijven openbare tegoeden openbaar zichtbaar totdat u ze zelf afschermt.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'De instelling kon niet worden opgeslagen. Probeer het opnieuw.';

  @override
  String get walletAutoShieldIncomplete =>
      'Automatisch afschermen is niet voltooid — deze tegoeden zijn nog steeds openbaar zichtbaar. U kunt ze nu afschermen.';

  @override
  String get walletSendPrivacyShielded =>
      'Afgeschermde betaling — het bedrag en de ontvanger blijven privé op de blockchain.';

  @override
  String get walletSendPrivacyTransparent =>
      'Openbare betaling — het bedrag en de adressen zijn zichtbaar op de blockchain.';

  @override
  String get walletActivityPublicBadge => 'Openbaar zichtbaar op de blockchain';

  @override
  String get walletShieldWalletEnded =>
      'De walletsessie is beëindigd. Sluit de wallet en open deze opnieuw om het nogmaals te proberen.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Nieuwe openbare tegoeden worden automatisch afgeschermd naar uw privésaldo zodra ze $minZec ZEC bereiken.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'Automatisch afschermen is uitgeschakeld — openbare tegoeden blijven openbaar zichtbaar totdat u ze afschermt.';

  @override
  String get walletMoveAutoShieldNote =>
      'Automatisch afschermen staat aan: zodra deze tegoeden binnenkomen, worden ze automatisch weer afgeschermd (er zijn dan opnieuw netwerkkosten aan verbonden). Om ze openbaar te houden, schakelt u eerst automatisch afschermen uit onder Openbare tegoeden.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Na deze verplaatsing is uw openbare saldo $amount ZEC — minder dan de $floor ZEC die nodig is om het weer af te schermen. Het blijft openbaar tot er meer binnenkomt.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'U verplaatst naar uw eigen openbare adres. Deze verplaatsing blijft permanent in het openbare register staan.';

  @override
  String get walletTxDetailVisibility => 'Zichtbaarheid';

  @override
  String get walletTransparentFundsAutoDenied =>
      'Automatisch afschermen is gepauzeerd voor deze sessie — dit is niet goedgekeurd. U kunt nog steeds handmatig afschermen.';

  @override
  String get walletDeepScanMenuItem => 'Oudere wisseladressen controleren…';

  @override
  String get walletMenuSyncNotRunningHint => 'Synchronisatie loopt nu niet.';

  @override
  String get walletDeepScanTitle => 'Oudere wisseladressen controleren';

  @override
  String get walletDeepScanBody =>
      'Als u deze wallet hersteld hebt en deze vroeger veel gebruikmaakte van wisselen, kan geld van de oudste wisseltransacties een extra stap nodig hebben om te worden gevonden. Dit controleert daarop — alles wat wordt gevonden verschijnt in uw saldo terwijl uw wallet synchroniseert.';

  @override
  String get walletDeepScanCoverage =>
      'Uw oudere wisseladressen zijn tot hier gecontroleerd. Ontbreekt er nog geld van een oude wisseltransactie? Controleer dan nog dieper.';

  @override
  String get walletDeepScanCoveragePending =>
      'Het huidige bereik wordt nog gecontroleerd — alles wat wordt gevonden, verschijnt in uw saldo. Dit kan een tijdje duren.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Controleert op geld van de oudste wisseltransacties van uw wallet.';

  @override
  String get walletDeepScanCheckButton => 'Oudere adressen controleren';

  @override
  String get walletDeepScanCheckDeeperButton =>
      'Nog oudere adressen controleren';

  @override
  String get walletDeepScanChecking => 'Bezig met controleren…';

  @override
  String get walletDeepScanClose => 'Sluiten';

  @override
  String get walletDeepScanTorHint =>
      'U bent op dit moment niet verbonden via Tor. Overweeg voor meer privacy te wachten tot Tor actief is voordat u controleert.';

  @override
  String get walletDeepScanRescanBusy =>
      'U kunt oudere wisseladressen controleren zodra de herscan is voltooid.';

  @override
  String get walletDeepScanRan =>
      'Oudere wisseladressen worden gecontroleerd — alles wat wordt gevonden, verschijnt in uw saldo.';

  @override
  String get walletDeepScanFailed =>
      'Kon de controle niet starten. Er is niets veranderd — probeer het opnieuw.';

  @override
  String get walletDeepScanSlow =>
      'Dit duurt langer dan gebruikelijk. Als uw oudere wisseladressen zijn gecontroleerd, verschijnt alles wat wordt gevonden in uw saldo — controleer straks nog eens.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'Wisselen staat momenteel uit, dus dit kan niet worden uitgevoerd. Probeer het opnieuw wanneer wisselen beschikbaar is.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Het laatste bereik wordt nog gecontroleerd — dit kan tot een paar dagen duren, maar meestal veel korter. Dit wordt vanzelf voltooid; controleer later opnieuw.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'We kunnen de privacy van uw verbinding nog niet bevestigen. Overweeg voor meer privacy te wachten tot Tor actief is voordat u controleert.';

  @override
  String get walletDeepScanBannerChecking =>
      'Oudere wisseladressen worden nog gecontroleerd — alles wat wordt gevonden, verschijnt in uw saldo.';

  @override
  String get walletRescanSwapPointer =>
      'Zoekt u geld van een oude wisseltransactie? Een herscan vindt dat niet — gebruik in plaats daarvan “Oudere wisseladressen controleren”.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Een wallet hersteld die wisselen gebruikte?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Als deze wallet een zeer lange wisselgeschiedenis had, kan geld van de oudste wisseltransacties een extra stap nodig hebben om te worden gevonden. De meeste wallets hebben niets nodig.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Nu controleren';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Sluiten';

  @override
  String walletTorHostPath(String transport) {
    return 'Via het privépad van uw app ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Via het privépad van uw app ($transport); verbindingen kunnen door de proxy aan elkaar worden gekoppeld';
  }

  @override
  String get walletTorHostOtherTransport => 'een privépad';

  @override
  String get walletTorHostDirect =>
      'Niet privé (directe verbinding van uw app)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Uw opgeslagen server gebruikt een onversleuteld adres, dat het privépad van uw app niet kan dragen. $host wordt gebruikt.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Meer over $label';
  }

  @override
  String get walletSendPaste => 'Plakken';

  @override
  String get walletSendScanQr => 'QR-code scannen';

  @override
  String get walletSendRecipientGetsLabel => 'Ontvanger krijgt';

  @override
  String get walletSwapDepositCopyAmount => 'Bedrag kopiëren';

  @override
  String get walletSwapDepositAmountCopied => 'Bedrag gekopieerd';

  @override
  String get walletScanOpenSettings => 'Instellingen openen';

  @override
  String get walletScanOpenSettingsFailed => 'Kon de instellingen niet openen.';

  @override
  String get walletSendLeaveTitle => 'Nog bezig met verzenden';

  @override
  String get walletSendLeaveBody =>
      'Je betaling gaat door als je weggaat. Je ziet in je activiteit hoe hij is afgelopen.';

  @override
  String get walletSendLeaveStay => 'Blijven';

  @override
  String get walletSendLeaveConfirm => 'Weggaan';

  @override
  String get walletSheetLeaveBody =>
      'Dit gaat door als je weggaat. Je ziet in je activiteit hoe het is afgelopen.';

  @override
  String get walletLoadingLabel => 'Laden';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle =>
      'Controleer voordat u opnieuw afschermt';

  @override
  String get walletShieldUnknownBody =>
      'We konden deze afscherming niet bevestigen. Kijk bij Activiteit voordat u het opnieuw probeert.';

  @override
  String get walletMoveUnknownTitle =>
      'Controleer voordat u opnieuw verplaatst';

  @override
  String get walletMoveUnknownBody =>
      'We konden deze verplaatsing niet bevestigen. Kijk bij Activiteit voordat u het opnieuw probeert.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
