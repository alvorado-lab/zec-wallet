// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Norwegian Bokmål (`nb`).
class WalletLocalizationsNb extends WalletLocalizations {
  WalletLocalizationsNb([String locale = 'nb']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Innstillinger';

  @override
  String get walletTitle => 'Lommebok';

  @override
  String get walletNotSetUpTitle => 'Lommeboken er ikke satt opp ennå';

  @override
  String get walletNotSetUpBody =>
      'Oppsett av lommebok kommer i en senere versjon. Oppsettet vil lede deg gjennom å skrive ned gjenopprettingsfrasen din før du kan motta midler — slik at ingenting noensinne er i fare uten en sikkerhetskopi.';

  @override
  String get walletStartupFailedTitle => 'Lommeboken kunne ikke starte';

  @override
  String get walletStartupFailedBody =>
      'Noe hindret lommeboken i å lastes på denne enheten. Hvis du allerede har en lommebok, er midlene i den ikke i fare — de finnes på Zcash-nettverket og kan gjenopprettes med gjenopprettingsfrasen din. Prøv igjen. Hvis dette fortsetter å skje, lukk appen og åpne den på nytt.';

  @override
  String get walletBalanceLabel => 'Saldo';

  @override
  String get walletHideBalance => 'Skjul saldo';

  @override
  String get walletShowBalance => 'Vis saldo';

  @override
  String get walletBalanceHiddenAmount => 'Saldo skjult';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Tilgjengelig nå';

  @override
  String get walletArrivingLabel => 'På vei inn';

  @override
  String get walletNotSpendableYetLabel => 'Kan ikke brukes ennå';

  @override
  String get walletActivityTitle => 'Aktivitet';

  @override
  String get walletActivityEmpty => 'Ingen aktivitet ennå';

  @override
  String get walletActivityError => 'Kunne ikke laste aktivitet';

  @override
  String get walletActivityReceived => 'Mottatt';

  @override
  String get walletActivitySent => 'Sendt';

  @override
  String get walletActivityPending => 'Under behandling';

  @override
  String get walletActivityQueued => 'I kø';

  @override
  String get walletActivityRetrying => 'Prøver på nytt';

  @override
  String get walletActivitySaved => 'Lagret';

  @override
  String get walletActivityExpired => 'Utløpt';

  @override
  String get walletActivityFailed => 'Mislyktes';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count bekreftelser',
      one: '1 bekreftelse',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count betalinger mottatt',
      one: 'Betaling mottatt',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Vis transaksjonsdetaljer';

  @override
  String get walletTxDetailStatus => 'Status';

  @override
  String get walletTxDetailFee => 'Nettverksgebyr';

  @override
  String get walletTxDetailDate => 'Dato';

  @override
  String get walletTxDetailHeight => 'Blokkhøyde';

  @override
  String get walletTxDetailMemo => 'Memo';

  @override
  String get walletTxDetailMemoAttached => 'Inkludert';

  @override
  String get walletTxDetailTxid => 'Transaksjons-ID';

  @override
  String get walletTxDetailCopyTxid => 'Kopier transaksjons-ID';

  @override
  String get walletTxDetailCopied => 'Transaksjons-ID kopiert';

  @override
  String get walletTxDetailClose => 'Lukk';

  @override
  String get walletTxFundsKept => 'Ingen midler forlot lommeboken din';

  @override
  String get walletTxExplainQueued =>
      'Lagret på denne enheten, under Lagret og under behandling — der kan du sende den eller avbryte den.';

  @override
  String get walletTxExplainPending =>
      'Sendt til Zcash-nettverket — venter på å bli bekreftet i en blokk.';

  @override
  String get walletTxExplainRetrying =>
      'Lommeboken din har ikke klart å sende dette til Zcash-nettverket ennå. Den beholder den signerte transaksjonen og prøver på nytt ved hver synkronisering til den går gjennom eller utløper.';

  @override
  String get walletTxExplainSaved =>
      'Lommeboken din har beholdt denne signerte transaksjonen, men sender den ikke av seg selv akkurat nå.';

  @override
  String get walletTxExplainConfirmed => 'Bekreftet på Zcash-nettverket.';

  @override
  String get walletTxExplainExpired =>
      'Denne transaksjonen utløp før nettverket bekreftet den, så den ble kansellert. Beløpet er fortsatt ditt å bruke.';

  @override
  String get walletTxExplainFailed =>
      'Nettverket avviste denne transaksjonen, så den gikk ikke gjennom. Beløpet er fortsatt ditt å bruke.';

  @override
  String get walletTxExplainUnknown =>
      'Denne transaksjonens nåværende status kan ikke fastslås. Den oppdateres etter neste synkronisering.';

  @override
  String get walletMenuTooltip => 'Flere valg';

  @override
  String get walletRescanMenuItem => 'Skann historikken på nytt…';

  @override
  String get walletCheckOneTimeMenuItem => 'Sjekk engangsadresser…';

  @override
  String get walletRescanTitle => 'Skann historikken din på nytt';

  @override
  String get walletRescanBody =>
      'Mangler du eldre midler? Skann blokkjeden på nytt fra lenger tilbake for å gjenopprette innskudd en tidligere startdato hoppet over. Midlene og gjenopprettingsfrasen din er aldri i fare.';

  @override
  String get walletRescanRangeTitle => 'Hvor langt tilbake å skanne';

  @override
  String get walletRescanRangeAll =>
      'Skann hele historikken din — tregest, men gjenoppretter alt.';

  @override
  String get walletRescanRangeDefault =>
      'Skanner fra starten av lommeboken din. Mangler du fortsatt eldre midler? Velg en tidligere dato, eller Skann hele historikken.';

  @override
  String get walletRescanRangeResolving => 'Forbereder anbefalt periode…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Omtrent $blocks blokker å skanne.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Skanner fra $date og fremover. Mangler du fortsatt eldre midler? Velg en tidligere dato, eller Skann hele historikken.';
  }

  @override
  String get walletRescanPick => 'Velg en dato';

  @override
  String get walletRescanChange => 'Endre dato';

  @override
  String get walletRescanScanAll => 'Skann hele historikken';

  @override
  String get walletRescanDatePick => 'Tidligste dato å skanne fra';

  @override
  String get walletRescanWarning =>
      'Dette skanner blokkjeden på nytt. Nylige datoer tar minutter; å skanne langt tilbake kan ta timer. Synkroniseringen kjører i bakgrunnen — du kan fortsette å bruke lommeboken.';

  @override
  String get walletRescanSettlingAdvisory =>
      'En betaling fra denne lommeboken blir fremdeles bekreftet. Lommeboken avslår vanligvis skanning på nytt før den er fullført — du kan prøve, men du må regne med å bli avvist.';

  @override
  String get walletRescanConfirm => 'Start ny skanning';

  @override
  String get walletRescanCancel => 'Avbryt';

  @override
  String get walletRescanRunning => 'Bygger opp igjen…';

  @override
  String get walletRescanRebuildingAll =>
      'Bygger opp historikken din igjen — skanner hele kjeden. Saldo og aktivitet fylles inn etter hvert som den tar igjen.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Bygger opp historikken din igjen fra $date — saldo og aktivitet fylles inn etter hvert som den tar igjen.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Bygger opp historikken din igjen fra starten av lommeboken din — saldo og aktivitet fylles inn etter hvert som den tar igjen.';

  @override
  String get walletCatchUpBanner =>
      'Tar igjen — saldo og aktivitet fylles inn mens lommeboken synkroniserer. Alt du har mottatt, er trygt.';

  @override
  String get walletCatchUpRescanBanner =>
      'Bygger opp historikken din igjen etter en ny skanning — saldo og aktivitet fylles inn etter hvert som den tar igjen. Alt du har mottatt, er trygt.';

  @override
  String get walletRescanFailedNotice =>
      'Kunne ikke skanne på nytt akkurat nå — midlene dine er trygge, men saldoen og historikken din kan trenge litt tid på å ta igjen. Prøv igjen om et øyeblikk.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'En betaling blir fremdeles bekreftet, så skanning på nytt er satt på pause for å beskytte midlene dine. Lommeboken din er uendret — prøv igjen om et par timer, og hold appen åpen og tilkoblet i mellomtiden.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'Skanning på nytt gjenoppbygger historikken din mens lommeboken din synkroniserer, og synkronisering kjører ikke akkurat nå. Lommeboken din er uendret — prøv igjen når synkronisering kjører.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Det er ikke nok ledig plass til å gjenoppbygge lommebokhistorikken — midlene dine er trygge, men saldoen og historikken din kan trenge litt tid på å ta igjen. Frigjør litt plass og prøv igjen.';

  @override
  String get walletRescanFailedDismiss => 'Lukk';

  @override
  String get walletActivityRebuilding => 'Bygger opp historikken din igjen…';

  @override
  String get walletActivityCatchingUp =>
      'Tar fortsatt igjen — alt du har mottatt, vises her.';

  @override
  String get walletActivitySyncNotRunning =>
      'Saldoen og historikken din blir ferdig lastet inn når synkronisering kjører.';

  @override
  String get walletActivityLoadMore => 'Last inn mer';

  @override
  String get walletPendingChangeLabel => 'Vekslepenger under behandling';

  @override
  String get walletTransparentLabel => 'Uskjermet (offentlig)';

  @override
  String get walletTransparentNote =>
      'Ikke inkludert i «Tilgjengelig nå» — skjerm disse midlene for å kunne bruke dem. Inntil da forblir de offentlig synlige på kjeden.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Disse midlene er offentlig synlige på kjeden.';

  @override
  String walletPoolShielded(String amount) {
    return 'Skjermet $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Offentlig $amount';
  }

  @override
  String get walletPoolAllShielded => 'Alt skjermet · privat';

  @override
  String get walletPoolTapHint => 'Vis offentlige midler';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount av saldoen din er på en engangsadresse (kan gjenopprettes).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount av saldoen din er på en engangsadresse.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Betalinger på til sammen $amount er reservert og fullføres fortsatt gjennom engangsadresser lommeboken din kontrollerer. Ikke send dem på nytt.',
      one:
          '$amount er reservert til en betaling lommeboken din fortsatt fullfører gjennom en engangsadresse den kontrollerer. Ikke send den på nytt.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Betalinger på til sammen $amount er reservert og er underveis gjennom engangsadresser lommeboken din kontrollerer. De er pauset til lommeboken din synkroniserer igjen. Ikke send dem på nytt.',
      one:
          '$amount er reservert til en betaling som er underveis gjennom en engangsadresse lommeboken din kontrollerer. Den er pauset til lommeboken din synkroniserer igjen. Ikke send den på nytt.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Kunne ikke sjekke om en betaling fortsatt fullføres. Prøver igjen – inntil da, se etter en ventende betaling i aktiviteten din før du sender på nytt.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount av saldoen din er på en engangsadresse (bekreftes fortsatt).';
  }

  @override
  String get walletShieldButton => 'Skjerm';

  @override
  String get walletShieldSheetTitle => 'Skjerm offentlige midler';

  @override
  String get walletShieldNote =>
      'Dette flytter midler fra din offentlige, synlige saldo på kjeden til din private, skjermede saldo.';

  @override
  String get walletShieldPreparing => 'Forbereder…';

  @override
  String get walletShieldAmountLabel => 'Skjermer';

  @override
  String get walletShieldFeeLabel => 'Nettverksgebyr';

  @override
  String get walletShieldNetLabel => 'Ender opp skjermet';

  @override
  String get walletShieldConfirmButton => 'Skjerm nå';

  @override
  String get walletShieldSubmitting => 'Skjermer…';

  @override
  String get walletShieldNothingTitle => 'Ingenting å skjerme ennå';

  @override
  String get walletShieldNothingBody =>
      'Disse midlene er under beløpet det er verdt å skjerme akkurat nå — nettverksgebyret ville overstige fordelen. De kan skjermes når litt mer kommer inn.';

  @override
  String get walletShieldDoneTitle => 'Skjerming sendt inn';

  @override
  String get walletShieldDoneBody =>
      'Midlene dine er på vei inn i den skjermede saldoen. Dette bekreftes på kjeden om kort tid.';

  @override
  String get walletShieldSavedTitle => 'Lagret — vi fullfører skjermingen';

  @override
  String get walletShieldSavedBody =>
      'Vi fikk ikke kontakt med nettverket akkurat nå. Skjermingen din er lagret, og lommeboken din fullfører den ved en senere synkronisering. Ingenting går tapt.';

  @override
  String get walletShieldAlreadyTitle => 'Allerede sendt inn';

  @override
  String get walletShieldFailedTitle => 'Kunne ikke skjerme akkurat nå';

  @override
  String get walletShieldStaleBody =>
      'Lommeboken synkroniserer fortsatt. Prøv å skjerme igjen om et øyeblikk.';

  @override
  String get walletShieldTransientBody =>
      'Kunne ikke klargjøre skjermingen akkurat nå. Prøv igjen om et øyeblikk.';

  @override
  String get walletShieldStorageFullBody =>
      'Det er ikke nok ledig plass til å skjerme akkurat nå. Frigjør litt plass og prøv igjen. Midlene dine er trygge.';

  @override
  String get walletShieldClose => 'Lukk';

  @override
  String get walletShieldRetry => 'Prøv igjen';

  @override
  String get walletMoveMenuItem => 'Flytt til offentlig…';

  @override
  String get walletMoveSheetTitle => 'Flytt til offentlig';

  @override
  String get walletMoveSheetSubtitle =>
      'Send skjermet ZEC til din egen offentlige adresse — nyttig for en børs som ikke godtar skjermede innskudd.';

  @override
  String get walletMoveDestinationLabel => 'Din offentlige adresse';

  @override
  String walletMoveAvailable(String amount) {
    return 'Tilgjengelig å flytte: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Tilgjengelig å flytte: $amount ZEC — saldoen tar fortsatt igjen';
  }

  @override
  String get walletMoveDeshieldTitle =>
      'Denne flyttingen gjør midlene dine offentlige';

  @override
  String get walletMoveDeshieldBody =>
      'Å flytte til en offentlig adresse tar disse midlene ut av den skjermede saldoen din — beløpet og din offentlige adresse blir offentlig synlige på Zcash-blokkjeden.';

  @override
  String get walletMoveWalletEnded =>
      'Lommebok-økten ble avsluttet. Lukk og åpne på nytt for å prøve igjen.';

  @override
  String get walletMoveLoading => 'Forbereder…';

  @override
  String get walletMovePreparing => 'Sjekker beløpet…';

  @override
  String get walletMoveSubmitting => 'Flytter…';

  @override
  String get walletMoveReviewButton => 'Se gjennom';

  @override
  String get walletMoveCancel => 'Avbryt';

  @override
  String get walletMoveReviewTitle => 'Se gjennom flyttingen';

  @override
  String get walletMoveOwnAddressNote =>
      'Du flytter til din egen offentlige adresse. Du kan skjerme disse midlene igjen senere, men denne flyttingen blir stående i det offentlige registeret permanent.';

  @override
  String get walletMoveConfirmButton => 'Flytt til offentlig';

  @override
  String get walletMoveBackButton => 'Tilbake';

  @override
  String get walletMoveDoneTitle => 'Flyttet til offentlig';

  @override
  String get walletMoveDoneBody =>
      'Midlene dine er på vei til din offentlige adresse. De bekreftes på kjeden om kort tid.';

  @override
  String get walletMoveSavedTitle => 'Lagret — vi fullfører flyttingen';

  @override
  String get walletMoveSavedBody =>
      'Denne flyttingen er lagret, og lommeboken din sender den ved en senere synkronisering. Ingenting gikk tapt.';

  @override
  String get walletMoveAlreadyTitle => 'Allerede sendt inn';

  @override
  String get walletMoveAlreadyBody =>
      'Disse midlene ble allerede sendt inn og er på vei til din offentlige adresse.';

  @override
  String get walletMoveFailedTitle => 'Kunne ikke fullføre denne flyttingen';

  @override
  String get walletMoveNothingTitle => 'Ingenting å flytte ennå';

  @override
  String get walletMoveNothingBody =>
      'Du har ingen skjermet saldo tilgjengelig å flytte akkurat nå. Når midler bekreftes, kan du flytte dem til din offentlige adresse.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Lommeboken tar fortsatt igjen — alt du har mottatt, blir tilgjengelig å flytte når synkroniseringen er fullført.';

  @override
  String get walletMoveCouldNotLoad =>
      'Kunne ikke laste din offentlige adresse. Prøv igjen.';

  @override
  String get walletMoveRetry => 'Prøv igjen';

  @override
  String get walletMoveClose => 'Lukk';

  @override
  String get walletSnapshotUnavailable =>
      'Kunne ikke lese lommeboken akkurat nå. Den oppdateres av seg selv.';

  @override
  String get walletBalanceStale =>
      'Kunne ikke oppdatere — viser din sist kjente saldo.';

  @override
  String get walletSyncStartFailed =>
      'Kunne ikke starte synkronisering. Vi fortsetter å prøve.';

  @override
  String get walletSyncRetry => 'Prøv igjen';

  @override
  String get walletSyncTryNow => 'Prøv nå';

  @override
  String get walletSyncIdle => 'Synkroniserer ikke ennå';

  @override
  String get walletSyncIdleDetail => 'Synkronisering starter automatisk.';

  @override
  String get walletSyncDisabled => 'Synkronisering avslått';

  @override
  String get walletSyncDisabledDetail =>
      'Slå på synkronisering i innstillingene for denne appen for å oppdatere saldoen din.';

  @override
  String get walletSyncExplainDisabled =>
      'Synkronisering er slått av i innstillingene for denne appen. Midlene dine er trygge. Saldoen og aktiviteten din viser den sist synkroniserte tilstanden, og oppdateres ikke før synkronisering slås på.';

  @override
  String get walletParkedSyncPausedNote =>
      'Lommeboken din synkroniserer ikke, så disse betalingene sendes ikke av seg selv. Bruk Send nå for å sende en selv.';

  @override
  String get walletSyncPausedMoneyNote =>
      'Pauset til lommeboken din synkroniserer igjen.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Kobler til…';

  @override
  String get walletSyncStartingDetail =>
      'Kobler til Zcash-nettverket og forbereder skanning.';

  @override
  String get walletSyncConnecting => 'Kobler til…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Kobler til… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Skanner $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'Skanner…';

  @override
  String get walletSyncSpendableReady => 'Midlene er klare til bruk.';

  @override
  String get walletSyncCatchingUp =>
      'Tar igjen nettverket — en grundig førstegangssynkronisering kan ta en stund. Du kan fortsette å bruke appen mens den fullføres';

  @override
  String walletSyncScanRemaining(String count) {
    return '$count blokker igjen';
  }

  @override
  String get walletSyncUpToDate => 'Oppdatert';

  @override
  String get walletSyncOffline => 'Frakoblet';

  @override
  String get walletSyncOfflineDetail =>
      'Sendinger i kø forblir lagret under Lagret og under behandling.';

  @override
  String get walletSyncUnknown => 'Synkroniserer…';

  @override
  String get walletSyncStalled => 'Synkronisering satt på pause';

  @override
  String get walletStallEndpoint =>
      'Får ikke kontakt med Zcash-nettverket akkurat nå. Vi fortsetter å prøve automatisk — sjekk internettforbindelsen din, eller serveren kan være midlertidig utilgjengelig.';

  @override
  String get walletStallTor =>
      'Appens private rute er utilgjengelig, så lommeboken kobler ikke til. Sjekk nettverksinnstillingene i appen din, eller slå av den private ruten. Synkroniseringen fortsetter så snart ruten er tilbake.';

  @override
  String get walletStallStorage =>
      'Enhetens lagringsplass er full. Frigjør plass, så fortsetter synkroniseringen.';

  @override
  String get walletStallReorg =>
      'Kjeden ble omorganisert; sjekker nylige blokker på nytt.';

  @override
  String get walletStallInternal =>
      'Et lokalt problem stoppet synkroniseringen. Hvis dette fortsetter å skje, gjenopprett fra gjenopprettingsfrasen din.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Denne serveren sendte data som ikke kan stemme, så synkroniseringen stoppet. Dette er ikke et tilkoblingsproblem – bytt til en annen server. Hvis alle servere avvises, skann historikken på nytt: lommeboken kan sitte med en feil oppføring fra en tidligere server.';

  @override
  String get walletStallBirthdayInFuture =>
      'Denne lommeboken er satt til å starte fra en blokk denne serveren ikke har nådd ennå. Kontroller startblokken denne lommeboken er satt til, eller prøv en annen server.';

  @override
  String get walletStallStorageUnavailable =>
      'Synkronisering satt på pause på denne enheten. Prøver igjen.';

  @override
  String get walletStallUnknown => 'Synkroniseringen stoppet av ukjent årsak.';

  @override
  String get walletSyncBadgeHint => 'Vis synkroniseringsdetaljer';

  @override
  String get walletSyncSheetClose => 'Lukk';

  @override
  String get walletSyncSheetProgress => 'Fremdrift';

  @override
  String get walletSyncSheetBlocksLeft => 'Blokker igjen';

  @override
  String get walletSyncSheetSyncedTo => 'Synkronisert til blokk';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Minst $blocks blokker bak',
      one: 'Minst 1 blokk bak',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'Synkroniseringen har ikke startet ennå — den starter automatisk. Ingen handling nødvendig.';

  @override
  String get walletSyncExplainStartFailed =>
      'Synkroniseringen kunne ikke starte. Midlene dine er trygge — lommeboken sjekker bare ikke etter ny aktivitet akkurat nå. Prøv igjen nedenfor, eller åpne appen på nytt.';

  @override
  String get walletSyncExplainStarting =>
      'Lommeboken kontakter Zcash-nettverket og forbereder skanning. Dette tar vanligvis noen sekunder.';

  @override
  String get walletSyncExplainConnecting =>
      'Etablerer en tilkobling til Zcash-nettverket.';

  @override
  String get walletSyncExplainScanning =>
      'Lommeboken sjekker blokker på blokkjeden for midlene dine. Saldo og aktivitet oppdateres etter hvert som nye transaksjoner blir funnet — du kan fortsette å bruke appen mens den fullføres.';

  @override
  String get walletSyncExplainUpToDate =>
      'Fullt synkronisert med Zcash-nettverket. Saldo og aktivitet er oppdatert.';

  @override
  String get walletSyncExplainStalled =>
      'Synkroniseringen støtte på et problem og er satt på pause. Den prøver på nytt automatisk.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Får ikke kontakt med Zcash-nettverket — det er normalt hvis du er frakoblet, eller serveren kan være midlertidig utilgjengelig. Midlene dine er trygge: saldoen viser den sist synkroniserte tilstanden, og sendinger i kø forblir lagret under Lagret og under behandling. Tilkoblingen prøver på nytt av seg selv.';

  @override
  String get walletSyncExplainOffline =>
      'Ingen nettverkstilkobling. Midlene dine er trygge — saldoen viser den sist synkroniserte tilstanden, og sendinger i kø forblir lagret under Lagret og under behandling.';

  @override
  String get walletSyncExplainUnknown =>
      'Lommeboken synkroniserer. Saldo og aktivitet oppdateres etter hvert som den fremskrider.';

  @override
  String get walletTorOff => 'Tor av';

  @override
  String get walletTorBootstrapping => 'Privat rute starter…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport starter…';
  }

  @override
  String get walletTorActive => 'Tor aktiv';

  @override
  String get walletTorActiveUnverified =>
      'Tor aktiv (ubekreftet kjøretidsmiljø)';

  @override
  String get walletTorActiveUnattested =>
      'Privat rute i bruk (personvern ikke bekreftet)';

  @override
  String get walletTorFellBack =>
      'Tor utilgjengelig — bruker direkte tilkobling';

  @override
  String get walletTorUnavailable =>
      'Privat rute utilgjengelig — ikke tilkoblet';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport utilgjengelig — ikke tilkoblet';
  }

  @override
  String get walletTorUnanswered =>
      'Privat rute tilkoblet — ingenting kommer tilbake';

  @override
  String get walletTorUnansweredUnattested =>
      'Privat rute tilkoblet — ingenting kommer tilbake (personvern ikke bekreftet)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport tilkoblet — ingenting kommer tilbake';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Ikke privat (appens direkte tilkobling) — ingenting kommer tilbake';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Tilkoblet via $transport — ingenting kommer tilbake; tilkoblinger kan knyttes sammen av proxyen';
  }

  @override
  String get walletTorUnknown =>
      'Tor-status ukjent — behandles som ikke beskyttet';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Saldo (per blokk $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Saldo · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Saldo (per blokk $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Tilkobling';

  @override
  String get walletSyncSheetServer => 'Server';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Server, $host, åpner servervelgeren';
  }

  @override
  String get walletSyncServerSheetTitle => 'Synkroniseringsserver';

  @override
  String get walletSyncServerInUse => 'I bruk';

  @override
  String get walletSyncServerAppDefault => 'Appens standard';

  @override
  String get walletSyncServerCustom => 'Egendefinert server…';

  @override
  String get walletSyncServerCustomHint => 'https://vert:port';

  @override
  String get walletSyncServerCheck => 'Sjekk server';

  @override
  String get walletSyncServerChecking => 'Sjekker…';

  @override
  String get walletSyncServerUse => 'Bruk denne serveren';

  @override
  String get walletSyncServerSwitching => 'Bytter…';

  @override
  String get walletSyncServerContinue => 'Fortsett';

  @override
  String get walletSyncServerCancel => 'Avbryt';

  @override
  String get walletSyncServerTrustTitle => 'Stole på denne serveren?';

  @override
  String get walletSyncServerTrustNotice =>
      'Du stoler på at denne serveren rapporterer saldoen og historikken din og videresender betalingene dine. Den ser IP-adressen din med mindre Tor er på, omtrent når lommeboken ble opprettet, de offentlige adressene lommeboken sjekker, transaksjonene den slår opp, og transaksjonene du sender.';

  @override
  String get walletSyncServerKeyLabel => 'Tilgangsnøkkel (valgfritt)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Nøkkelens header';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Skriv inn headeren serveren din forventer';

  @override
  String get walletSyncServerKeyInvalid =>
      'Denne nøkkelen eller headeren kan ikke brukes';

  @override
  String get walletSyncServerKeySaved => 'Nøkkel lagret';

  @override
  String get walletSyncServerKeyShow => 'Vis';

  @override
  String get walletSyncServerKeyHide => 'Skjul';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Nøkkelen din identifiserer deg overfor denne serveren. Den kan knytte betalingene dine til lommeboken din, også over Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Bytte starter den pågående synkroniseringen på nytt. Saldo og historikk beholdes. Midler kan vises som på vei inn til skanningen på den nye serveren har tatt igjen.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Bytte kobler til den nye serveren på nytt. Saldo og historikk beholdes.';

  @override
  String get walletSyncServerUnreachable =>
      'Fikk ikke kontakt med denne serveren. Sjekk adressen — og hvis den stemmer, svarer enten ikke denne serveren, eller appen din når den ikke akkurat nå. Prøv igjen, eller velg en annen server.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Fikk ikke kontakt med denne serveren. Lommeboken kan ikke skille mellom at denne serveren ikke svarer og at appen din ikke når den akkurat nå. Velg en annen server, eller prøv igjen senere.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Denne serveren er på et annet Zcash-nettverk.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Dette ser ikke ut som en serveradresse. Bruk https://vert:port.';

  @override
  String get walletSyncServerNotOffered =>
      'Denne serveren tilbys ikke av denne appen.';

  @override
  String get walletSyncServerBusy =>
      'Lommeboken er opptatt akkurat nå. Prøv igjen om et øyeblikk.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'Serveren du valgte tilbys ikke lenger av denne appen. Bruker $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'Det lagrede servervalget kunne ikke leses. Bruker $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Kunne ikke bytte – bruker fortsatt $host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Lommebok-trafikken kobler direkte til serveren. Serveren kan se IP-adressen din.';

  @override
  String get walletTransportExplainTor =>
      'Lommebok-trafikken rutes gjennom Tor-nettverket, som skjuler IP-adressen din for serveren.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Appens private rute starter opp. Lommebok-trafikken venter på dette før den kobler til.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport starter opp. Lommebok-trafikken venter på dette før den kobler til.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Fikk ikke kontakt med Tor, så trafikken falt tilbake til en direkte tilkobling. Serveren kan se IP-adressen din.';

  @override
  String get walletTransportExplainUnavailable =>
      'Appens private rute er utilgjengelig, så lommeboken kobler ikke til. Slå av den private ruten, eller sjekk nettverksinnstillingene i appen din.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport er utilgjengelig, så lommeboken kobler ikke til. Slå det av, eller sjekk nettverksinnstillingene i appen din.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'Den private ruten tok imot tilkoblingen, men det har ikke kommet noe tilbake på et minutt. Det kan være ruten eller lommebokserveren — lommeboken kan ikke skille dem. Den fortsetter å prøve; hvis det ikke gir seg, prøv en annen server eller sjekk nettverksinnstillingene i appen din.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport tok imot tilkoblingen, men det har ikke kommet noe tilbake på et minutt. Det kan være ruten eller lommebokserveren — lommeboken kan ikke skille dem. Den fortsetter å prøve; hvis det ikke gir seg, prøv en annen server eller sjekk nettverksinnstillingene i appen din.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Lommebok-trafikken kobler direkte til serveren. Serveren kan se IP-adressen din. Tilkoblingen ble tatt imot, men det har ikke kommet noe tilbake på et minutt. Det kan være ruten eller lommebokserveren — lommeboken kan ikke skille dem. Den fortsetter å prøve; hvis det ikke gir seg, prøv en annen server eller sjekk nettverksinnstillingene i appen din.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'Personvernet til denne tilkoblingen kan ikke bekreftes — behandle den som ikke privat. Tilkoblingen ble tatt imot, men det har ikke kommet noe tilbake på et minutt. Det kan være ruten eller lommebokserveren — lommeboken kan ikke skille dem. Den fortsetter å prøve; hvis det ikke gir seg, prøv en annen server eller sjekk nettverksinnstillingene i appen din.';

  @override
  String get walletTransportExplainUnverified =>
      'Personvernet til denne tilkoblingen kan ikke bekreftes — behandle den som ikke privat.';

  @override
  String get walletTransportExplainHostProxy =>
      'Lommebok-trafikken rutes gjennom denne appens personverntransport, som skjuler IP-adressen din for serveren.';

  @override
  String get walletOnboardingWelcomeTitle => 'Sett opp lommeboken din';

  @override
  String get walletOnboardingWelcomeBody =>
      'Opprett en ny lommebok for å motta og oppbevare ZEC. Vi genererer en gjenopprettingsfrase og leder deg gjennom å ta sikkerhetskopi av den før midler kan komme inn — slik at ingenting noensinne er i fare uten en sikkerhetskopi.';

  @override
  String get walletCreateButton => 'Opprett en ny lommebok';

  @override
  String get walletRestoreButton => 'Gjenopprett fra en gjenopprettingsfrase';

  @override
  String get walletWatchOnlyButton => 'Se på en lommebok (kun innsyn)';

  @override
  String get walletWatchOnlyTitle => 'Se på en lommebok';

  @override
  String get walletWatchOnlyBody =>
      'Lim inn en innsynsnøkkel for å se på en lommebok uten sendenøklene. Du vil se saldoen og historikken, men du vil ikke kunne sende midler. Velg lommebokens omtrentlige startdato, så vi vet hvor langt tilbake vi skal se.';

  @override
  String get walletWatchOnlyKeyLabel => 'Innsynsnøkkel';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => 'Skann en QR-kode for innsynsnøkkel';

  @override
  String get walletWatchOnlyScanTitle => 'Skann innsynsnøkkel';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Rett kameraet mot QR-koden for innsynsnøkkelen.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Kameraet er utilgjengelig. Lim inn nøkkelen manuelt i stedet.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Lim inn i stedet';

  @override
  String get walletWatchOnlyScanHint =>
      'Eller trykk på skann-knappen for å lese en QR-kode for innsynsnøkkel.';

  @override
  String get walletWatchOnlyScanFilled => 'Innsynsnøkkel skannet.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Lommebokens startdato';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Skanner fra $date og fremover — midler mottatt før dette vises ikke. Eldre lommebok? Velg en tidligere dato.';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'Velg lommebokens startdato';

  @override
  String get walletWatchOnlyBirthdayChange => 'Endre dato';

  @override
  String get walletWatchOnlySubmit => 'Se på denne lommeboken';

  @override
  String get walletWatchOnlyBack => 'Tilbake';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Dette ser ikke ut som en gyldig innsynsnøkkel. Sjekk den, og prøv igjen.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Denne innsynsnøkkelen er for et annet nettverk. Den kan ikke brukes her.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'En lommebok finnes allerede på denne enheten. Gå tilbake for å åpne den i stedet.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Denne startdatoen er for ny. Velg en tidligere dato.';

  @override
  String get walletRestoreTitle => 'Gjenopprett lommeboken din';

  @override
  String get walletRestoreBody =>
      'Skriv inn gjenopprettingsfrasen din for å gjenopprette lommeboken — skriv eller lim inn ordene i riktig rekkefølge, atskilt med mellomrom. Kun standardfraser: hvis lommeboken din brukte en ekstra passordfrase (et «25. ord»), kan ikke denne appen gjenopprette den ennå — du vil se en tom lommebok, ikke en feilmelding.';

  @override
  String get walletRestorePhraseHint => 'ord én  ord to  ord tre  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count ord',
      one: '1 ord',
      zero: 'Ingen ord ennå',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'gjenopprettingsfraser har 12, 15, 18, 21 eller 24 ord';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count ord er ikke gjenopprettingsord — rett de uthevede',
      one: '1 ord er ikke et gjenopprettingsord — rett det uthevede',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'ord $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'ord $index: ikke et gjenopprettingsord';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Fjern ord $index';
  }

  @override
  String get walletRestoreSubmit => 'Gjenopprett lommebok';

  @override
  String get walletRestoreBack => 'Tilbake';

  @override
  String get walletRestoreBirthdayTitle => 'Hvor langt tilbake å skanne';

  @override
  String get walletRestoreBirthdayNone =>
      'Vi skanner hele historikken din — tregere, men ingenting hoppes over.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Skanner fra $date og fremover — midler mottatt før dette vises ikke. Eldre lommebok? Velg en tidligere dato, eller Skann hele historikken.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Velg en dato';

  @override
  String get walletRestoreBirthdayChange => 'Endre dato';

  @override
  String get walletRestoreBirthdayClear => 'Skann hele historikken';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'Ord $index er ikke et gjenopprettingsord. Sjekk frasen for skrivefeil, og prøv igjen.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Denne gjenopprettingsfrasen er ikke gyldig. Sjekk ordene og rekkefølgen deres, og prøv igjen.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Denne frasen samsvarer ikke med lommeboken på denne enheten. Dobbeltsjekk den og prøv igjen.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'En lommebok finnes allerede på denne enheten. Gå tilbake for å åpne den.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Denne datoen er for ny. Velg en tidligere dato, eller skann alt.';

  @override
  String get walletGeneratingLabel => 'Oppretter lommeboken din…';

  @override
  String get walletOpeningLabel => 'Åpner lommeboken din…';

  @override
  String get walletBackupTitle =>
      'Ta sikkerhetskopi av gjenopprettingsfrasen din';

  @override
  String get walletBackupBody =>
      'Disse ordene er den ENESTE måten å gjenopprette lommeboken og midlene dine på. Skriv dem ned i riktig rekkefølge, og oppbevar dem på et trygt og privat sted. Del dem aldri, og lagre dem aldri på nett — hvem som helst med disse ordene kan ta midlene dine.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Skjermbilder er slått av på denne skjermen.';

  @override
  String get walletBackupSecureNoteOther =>
      'Sørg for at ingen andre kan se skjermen din.';

  @override
  String get walletBackupReveal => 'Vis gjenopprettingsfrase';

  @override
  String get walletBackupRevealing => 'Forbereder gjenopprettingsfrasen din…';

  @override
  String get walletBackupRevealFailed =>
      'Kunne ikke vise gjenopprettingsfrasen din akkurat nå. Sørg for at enheten er låst opp, og prøv igjen.';

  @override
  String get walletBackupRetryReveal => 'Prøv igjen';

  @override
  String get walletBackupReauthFailed =>
      'Kunne ikke bekrefte identiteten din. Prøv igjen.';

  @override
  String get walletBackupConfirmCheckbox =>
      'Jeg har skrevet ned gjenopprettingsfrasen min og oppbevart den trygt.';

  @override
  String get walletBackupContinue => 'Fortsett';

  @override
  String get walletBackupSaveFailed =>
      'Kunne ikke lagre bekreftelsen din. Prøv igjen.';

  @override
  String get walletBackupStartOver => 'Start på nytt';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Starte på nytt uten denne lommeboken?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Dette sletter denne lommeboken fra enheten og fører deg tilbake til starten. Ingenting kan settes inn via denne appen før oppsettet er fullført.\n\nHvis denne lommeboken noen gang har inneholdt midler — eller ble gjenopprettet fra en gjenopprettingsfrase — kan bare den frasen gjenopprette den.';

  @override
  String get walletBackupStartOverConfirm => 'Slett og start på nytt';

  @override
  String get walletBackupStartOverKeep => 'Behold denne lommeboken';

  @override
  String get walletBackupSectionTitle => 'Gjenopprettingsfrase';

  @override
  String get walletBackupTileTitle =>
      'Ta sikkerhetskopi av gjenopprettingsfrasen din';

  @override
  String get walletBackupTileSubtitle =>
      'Vis ordene som kan gjenopprette lommeboken og midlene dine.';

  @override
  String get walletBackupScreenTitle => 'Gjenopprettingsfrase';

  @override
  String get walletBackupDone => 'Ferdig';

  @override
  String get walletBackupManagedTitle => 'Ingen egen gjenopprettingsfrase';

  @override
  String get walletBackupManagedBody =>
      'Denne lommeboken ble satt opp med kontoen din fra appen som installerte den, så den har ingen egen gjenopprettingsfrase. Midlene dine gjenopprettes sammen med den kontoen — bruk kontoens sikkerhetskopi for å holde dem trygge.';

  @override
  String get walletExportViewingKeyTitle => 'Eksporter innsynsnøkkelen din';

  @override
  String get walletExportViewingKeyTileTitle => 'Eksporter innsynsnøkkelen din';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Del en innsynskopi av lommeboken din — den kan se historikken din, men kan ikke sende midler.';

  @override
  String get walletExportViewingKeyWarning =>
      'Denne nøkkelen lar den som har den, se alt denne lommeboken noensinne har mottatt og sendt — og alt den kommer til å motta og sende i fremtiden. Den kan ikke bruke midlene dine, og den kan ikke gjenopprette lommeboken din. Del den bare med noen du stoler på til å se hele historikken din, for eksempel en regnskapsfører eller din egen andre enhet. Den eneste måten å oppheve delingen senere på, er å flytte midlene dine til en ny lommebok.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Denne nøkkelen lar den som har den, se alt denne lommeboken noensinne har mottatt og sendt — og alt den kommer til å motta og sende i fremtiden. Den kan ikke bruke midlene dine, og den kan ikke gjenopprette lommeboken din. Del den bare med noen du stoler på til å se hele historikken din, for eksempel en regnskapsfører eller din egen andre enhet. Når den først er delt, kan delingen ikke oppheves.';

  @override
  String get walletExportViewingKeyReveal => 'Vis innsynsnøkkel';

  @override
  String get walletExportViewingKeyRetry => 'Prøv igjen';

  @override
  String get walletExportViewingKeyRevealing =>
      'Forbereder innsynsnøkkelen din…';

  @override
  String get walletExportViewingKeyFailed =>
      'Kunne ikke vise innsynsnøkkelen din akkurat nå. Prøv igjen om et øyeblikk.';

  @override
  String get walletExportViewingKeyQrLabel => 'QR-kode for innsynsnøkkelen';

  @override
  String get walletExportViewingKeyCopy => 'Kopier innsynsnøkkel';

  @override
  String get walletExportViewingKeyCopied => 'Innsynsnøkkel kopiert';

  @override
  String get walletExportViewingKeyDone => 'Ferdig';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Skjermbilder er slått av på denne skjermen.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Sørg for at ingen andre kan se skjermen din.';

  @override
  String get walletWatchOnlySectionTitle => 'Om denne innsynslommeboken';

  @override
  String get walletWatchOnlyAboutBody =>
      'Dette er en innsynslommebok. Den ble satt opp fra en innsynsnøkkel, så den kan se saldoen og historikken din, men har ingen sendenøkler — det er ingenting å ta sikkerhetskopi av her, og den kan ikke sende midler.';

  @override
  String get walletWatchOnlyBadge => 'Kun innsyn';

  @override
  String get walletOnboardingFailedTitle =>
      'Oppsett av lommebok kunne ikke fullføres';

  @override
  String get walletOnboardingRetry => 'Prøv igjen';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'Den sikre lagringen på telefonen din svarer ikke. Lås opp enheten og prøv igjen. Hvis dette fortsetter, start telefonen på nytt.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Denne lommeboken er åpen i et annet vindu eller en annen app, eller fullfører fortsatt en tidligere operasjon. Lukk andre vinduer som bruker den — eller vent et øyeblikk — og prøv igjen.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'Denne lommebokens sikre nøkkel er ikke lenger tilgjengelig, så den kan ikke åpnes på denne enheten. Midlene dine er trygge — gjenopprett fra gjenopprettingsfrasen din for å få dem tilbake.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Gjenopprett fra gjenopprettingsfrase';

  @override
  String get walletOnboardingRecoverConfirmTitle =>
      'Gjenopprette denne lommeboken?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Sørg for at du har gjenopprettingsfrasen din før du fortsetter — du trenger den på neste skjerm for å få tilbake midlene dine. Midlene dine er trygge på blokkjeden og kontrolleres av denne frasen. Dette fjerner den uleselige lommebokdataen fra denne enheten, slik at den kan bygges opp igjen.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Avbryt';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Det er ikke nok ledig plass til å sette opp lommeboken. Frigjør litt plass og prøv igjen.';

  @override
  String get walletOnboardingFailedNoVault =>
      'Denne enheten har ikke et sikkert nøkkellager, så lommeboken kan ikke beskytte gjenopprettingsfrasen din her.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Fikk ikke kontakt med nettverket under oppsettet. Sjekk tilkoblingen din og prøv igjen.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'Oppsettet av lommeboken ble ikke fullført. Prøv igjen for å fullføre det — ingenting gikk tapt.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Noe gikk galt under oppsettet av lommeboken. Prøv igjen.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'Denne appens lommebokoppsett er feilkonfigurert, så lommeboken kan ikke starte. Å prøve på nytt vil ikke hjelpe — rapporter dette til appens utvikler. Midlene dine er ikke i fare.';

  @override
  String get walletSendButton => 'Send';

  @override
  String get walletSendSyncNotRunning =>
      'Synkronisering kjører ikke — den tilgjengelige saldoen din kan ikke oppdateres';

  @override
  String get walletSendWaitingForFunds =>
      'Synkroniserer fortsatt — du kan sende når du har tilgjengelig saldo';

  @override
  String get walletSendNoSpendableYet => 'Ingen brukbar saldo ennå';

  @override
  String get walletSendSyncUnavailable =>
      'Du kan sende når synkroniseringen fortsetter igjen';

  @override
  String get walletSendTitle => 'Send';

  @override
  String get walletSendUnavailable =>
      'Lommeboken din er ikke klar akkurat nå. Gå tilbake og prøv igjen.';

  @override
  String get walletSendWatchOnly =>
      'Dette er en lommebok med kun innsyn. Den kan vise saldo og motta betalinger, men den har ingen sendenøkler — så den kan ikke sende.';

  @override
  String get walletSendExpiredTitle => 'Denne betalingsforespørselen utløp';

  @override
  String get walletSendExpiredBody =>
      'Sendeskjermen brukte mer enn fem sekunder på å åpne, så appen fikk beskjed om at ingenting ble sendt. Det svaret er endelig: denne forespørselen kan ikke betales herfra. For å betale, start på nytt fra appen.';

  @override
  String get walletSendFaultWatchOnly =>
      'Dette er en lommebok med kun innsyn — den har ingen sendenøkler, så den kan ikke sende.';

  @override
  String walletSendAvailable(String amount) {
    return 'Tilgjengelig å sende: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Tilgjengelig å sende: $amount ZEC — saldoen tar fortsatt igjen';
  }

  @override
  String get walletSendRecipientLabel => 'Mottakeradresse';

  @override
  String get walletSendRecipientHint =>
      'Zcash-adresse (starter med u, z eller t)';

  @override
  String get walletSendRecipientLocked => 'Mottakeren kan ikke endres her';

  @override
  String get walletSendAmountLabel => 'Beløp (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Memo (valgfritt)';

  @override
  String get walletSendMemoHint =>
      'Leveres kun til skjermede (private) mottakere';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Memoer krever en skjermet mottaker. Denne offentlige adressen kan ikke motta et.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Denne betalingen har allerede en referanse fra appen, så den kan ikke også ha en skrevet melding.';

  @override
  String get walletSendMachineMemoTitle => 'Appen legger ved en referanse';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Den oppgir at dette er til: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Den blir liggende med transaksjonen og kan ikke fjernes senere. Lommeboken kan ikke kontrollere hva den inneholder.';

  @override
  String get walletSendRecipientShielded => 'Skjermet · privat';

  @override
  String get walletSendRecipientTransparent => 'Offentlig';

  @override
  String get walletSendRecipientInvalid =>
      'Dette ser ikke ut som en gyldig Zcash-adresse.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Denne adressen er for et annet Zcash-nettverk.';

  @override
  String get walletSendReviewButton => 'Se gjennom betaling';

  @override
  String get walletSendQueueButton => 'Sett i kø for å sende senere';

  @override
  String get walletSendQueueHint =>
      'En betaling i kø venter under Lagret og under behandling, hvor du kan sende den eller avbryte den. Nettverksgebyret beregnes når den sendes.';

  @override
  String get walletSendPreparing => 'Forbereder betalingen din…';

  @override
  String get walletSendSubmitting => 'Sender…';

  @override
  String get walletSendQueuing => 'Setter i kø…';

  @override
  String get walletSendReviewTitle => 'Bekreft betaling';

  @override
  String get walletSendTotalLabel => 'Totalt';

  @override
  String get walletSendFeeLabel => 'Nettverksgebyr';

  @override
  String get walletSendChangeLabel => 'Vekslepenger returnert';

  @override
  String get walletSendDeshieldTitle => 'Denne betalingen er ikke privat';

  @override
  String get walletSendDeshieldBody =>
      'Den sendes til en offentlig adresse, så beløpet og mottakeren blir offentlig synlige på Zcash-blokkjeden.';

  @override
  String get walletSendPublicAckLabel =>
      'Jeg forstår at denne betalingen blir offentlig.';

  @override
  String get walletSendConfirmButton => 'Send nå';

  @override
  String get walletSendBackButton => 'Tilbake';

  @override
  String get walletSendSelfSendNote =>
      'Du sender til din egen lommebok. Nettverksgebyret gjelder fortsatt.';

  @override
  String get walletSendLargeConfirmTitle => 'Sende et stort beløp?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Dette er nesten hele saldoen din. En sendt betaling kan ikke reverseres.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Dette er en stor betaling. En sendt betaling kan ikke reverseres.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Dette er en stor betaling — nesten hele saldoen din. En sendt betaling kan ikke reverseres.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Send $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Gå tilbake';

  @override
  String get walletSendSentTitle => 'Betaling sendt';

  @override
  String get walletSendSentBody => 'Betalingen din er sendt til nettverket.';

  @override
  String get walletSendSavedTitle => 'Lagret — vi fullfører sendingen';

  @override
  String get walletSendSavedBody =>
      'Betalingen din kunne ikke sendes akkurat nå, så den er lagret — lommeboken din sender den ved en senere synkronisering. Ingenting går tapt.';

  @override
  String get walletSendKeptTitle => 'Lagret';

  @override
  String get walletSendKeptBody =>
      'Lommeboken din har beholdt denne transaksjonen, men har ikke lovet å sende den av seg selv. Se Aktivitet for å følge med på hvor den står.';

  @override
  String get walletSendPartialBody =>
      'En del av betalingen din ble sendt; lommeboken din fullfører resten ved en senere synkronisering. Ingenting går tapt.';

  @override
  String get walletSendInMotionTitle => 'Betaling pågår';

  @override
  String get walletSendInMotionBody =>
      'Betalingen din har startet og beveger seg gjennom en engangsadresse lommeboken din kontrollerer. Ikke send den på nytt. Hvis den ikke fullføres, kan du gjenopprette midlene fra lommebokskjermen.';

  @override
  String get walletSendAlreadyTitle => 'Allerede sendt inn';

  @override
  String get walletSendAlreadyBody =>
      'Denne betalingen ble allerede sendt inn — den sendes ikke to ganger.';

  @override
  String get walletSendFailedTitle => 'Kunne ikke fullføre betalingen';

  @override
  String get walletSendFailedBody =>
      'Noe gikk galt under fullføringen av denne betalingen, og ingenting ble sendt. Du kan prøve igjen.';

  @override
  String get walletSendTryAgain => 'Prøv igjen';

  @override
  String get walletSendDone => 'Ferdig';

  @override
  String get walletSendAnother => 'Send en til';

  @override
  String get walletSendQueuedTitle => 'Satt i kø for sending';

  @override
  String get walletSendQueuedBody =>
      'Denne betalingen er lagret. Du finner den under Lagret og under behandling, hvor du kan sende den nå eller avbryte den.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Ikke nok brukbar saldo — du har $available ZEC, og dette krever $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Zcash-nettverket ble oppgradert, og denne appen trenger en oppdatering før den kan sende. Midlene dine er trygge.';

  @override
  String get walletSyncUpToDateLimited =>
      'Oppdatert så langt denne versjonen kan lese';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Zcash-nettverket ble oppgradert. Denne versjonen har skannet alt den kan lese, men nyere blokker kan inneholde midler den ennå ikke kan vise, og notater på nylige betalinger er utilgjengelige. Oppdater appen for å se alt.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Oppdatert, men denne serveren betjener ikke alle pooler';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Denne serveren avviser, holder tilbake eller rapporterer feil om en av Zcashs skjermede pooler. Midler mottatt i den poolen kan ikke brukes via denne serveren, og saldoen som vises er et minimum. Bytt til en annen server for å bruke dem – dette er ikke et tilkoblingsproblem.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: denne serveren nekter å levere den';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: denne serveren holder tilbake en del av den';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: denne serveren rapporterer den feil';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: det er ukjent om denne serveren leverer den';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Oppdatert mot denne serveren, men serveren ligger bak nettverket';

  @override
  String get walletSyncExplainEndpointBehind =>
      'Kjeden til denne serveren stopper ved en blokk nettverket allerede hadde passert før denne versjonen av appen ble bygget, så saldoen din er bare oppdatert til den blokken. Nye betalinger til deg vises kanskje ikke ennå, og en betaling sendt herfra kommer kanskje ikke fram. Bytt til en annen server for å komme à jour – dette er ikke et tilkoblingsproblem.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Venter på en appoppdatering — midlene dine er trygge, og ingenting er sendt.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Venter på en server som oppgir nettverksversjonen — bytt server. Midlene dine er trygge, og ingenting er sendt.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Venter på en server som oppgir nettverksversjonen. Hvis dato og klokkeslett på denne enheten er feil, retter du dem først – og bytter deretter server. Midlene dine er trygge, og ingenting er sendt.';

  @override
  String get walletSyncUnverified =>
      'Oppdatert, men denne serveren oppgir ikke nettverksversjonen';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'Sending fungerer fortsatt i omtrent $hours timer til — bytt server etterpå.',
      one:
          'Sending fungerer fortsatt i omtrent 1 time til — bytt server etterpå.',
      zero: 'Sending fungerer fortsatt i under en time — bytt server etterpå.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'Sending fungerer fortsatt i omtrent $blocks blokker til — bytt server etterpå.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Denne serveren har ikke oppgitt nettverksversjonen på $blocks blokker, så denne appen kan ikke bekrefte at det er trygt å sende. Bytt til en annen server.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Denne serveren har ikke oppgitt nettverksversjonen på en dag, så denne appen kan ikke bekrefte at det er trygt å sende. Hvis dato og klokkeslett på denne enheten er feil, retter du dem først – og bytter deretter til en server som oppgir nettverksversjonen.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Denne serveren har aldri oppgitt nettverksversjonen, så denne appen kan ikke bekrefte at det er trygt å sende. Bytt til en annen server.';

  @override
  String get walletSyncExplainUnverified =>
      'Denne serveren sier ikke hvilken versjon av Zcash-nettverket den er på, så denne appen kan ikke bekrefte at en betaling den signerer blir godtatt. Saldoen din er oppdatert. Bytt til en annen server — dette er ikke et tilkoblingsproblem.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Denne serveren sier ikke hvilken versjon av Zcash-nettverket den er på, så denne appen kan ikke bekrefte at en betaling den signerer blir godtatt. Den har også fortsatt å levere blokker som denne lommeboken siden måtte angre, så saldoen din er kanskje ikke oppdatert. Bytt til en annen server — dette er ikke et tilkoblingsproblem.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Denne serveren fortsetter også å levere blokker som denne lommeboken siden må angre — bytt server.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Saldoen tar fortsatt igjen — mer kan bli tilgjengelig mens lommeboken synkroniserer.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC er fortsatt på vei inn og blir brukbar når lommeboken har kommet à jour.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Skriv inn et beløp å sende.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Skriv inn beløpet som et tall, for eksempel 0.25.';

  @override
  String get walletSendFaultAmountDecimals => 'ZEC har maks 8 desimaler.';

  @override
  String get walletSendFaultAmountNotPositive =>
      'Skriv inn et beløp større enn null.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Dette beløpet er større enn det totale ZEC-tilbudet.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Denne appen begrenser for øyeblikket sendinger til $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Dette ser ikke ut som en gyldig Zcash-adresse for dette nettverket. Sjekk den og prøv igjen.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Denne mottakeren kan ikke motta et memo. Fjern memoet, eller send til en skjermet (privat) adresse.';

  @override
  String get walletSendFaultMemoTooLong =>
      'Memoet ditt er for langt. Korte det ned og prøv igjen.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Dette memoet kan ikke sendes. Fjern det og prøv igjen.';

  @override
  String get walletSendFaultMemoConflict =>
      'Kunne ikke sende denne betalingen – appen la ved to notater. Ingenting ble sendt.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Denne adressen er for et annet nettverk.';

  @override
  String get walletSendFaultUriInvalid =>
      'Kunne ikke opprette denne betalingen. Sjekk adressen og beløpet.';

  @override
  String get walletSendFaultNotSynced =>
      'Lommeboken din er ikke synkronisert langt nok ennå. Vent til synkroniseringen tar igjen, eller sett denne i kø for å sende senere.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Lommeboken din er ikke synkronisert langt nok ennå. Vent til synkroniseringen tar igjen.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Lommeboken din er ikke synkronisert langt nok ennå, og synkronisering kjører ikke akkurat nå. Sjekk synkroniseringsstatusen på lommebokskjermen.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Beløpene utløp mens du så gjennom betalingen. Se gjennom betalingen på nytt.';

  @override
  String get walletSendFaultQueueFull =>
      'For mange sendinger venter på å bli sendt. La dem fullføres først, og prøv igjen.';

  @override
  String get walletSendFaultWalletBusy =>
      'Lommeboken er opptatt akkurat nå. Prøv igjen om et øyeblikk.';

  @override
  String get walletSendFaultStorageFull =>
      'Det er ikke nok ledig plass til å fullføre denne sendingen. Frigjør litt plass og prøv igjen.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'For mange engangsadresser er i bruk akkurat nå. Noen kan bli frigjort etter hvert som overføringer bekreftes, men dette går kanskje ikke over av seg selv. Midlene dine er trygge.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Kunne ikke forberede denne betalingen. Sjekk detaljene og prøv igjen.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Kunne ikke klargjøre denne betalingen akkurat nå. Prøv igjen om et øyeblikk.';

  @override
  String get walletSwapButton => 'Bytt';

  @override
  String get walletSwapTitle => 'Bytt ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'Lommeboken din er ikke klar akkurat nå. Gå tilbake og prøv igjen.';

  @override
  String get walletSwapUnavailableOff =>
      'Bytte er ikke tilgjengelig akkurat nå.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Dette er en lommebok med kun innsyn — den kan ikke bytte.';

  @override
  String get walletSwapDone => 'Ferdig';

  @override
  String get walletSwapBackToWallet => 'Tilbake til lommeboken';

  @override
  String walletSwapAvailable(String amount) {
    return 'Tilgjengelig å bytte: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Tilgjengelig å bytte: $amount ZEC — saldoen tar fortsatt igjen';
  }

  @override
  String get walletSwapAssetLabel => 'Eiendel å motta';

  @override
  String get walletSwapAmountLabel => 'Beløp å bytte (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Destinasjonsadresse';

  @override
  String get walletSwapDestinationHint =>
      'Mottakeradressen din på destinasjonskjeden';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Din $chain-mottakeradresse';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'En $chain-adresse — dit den byttede eiendelen din sendes. Dobbeltsjekk at kjeden stemmer.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Skann en QR-kode for destinasjonsadresse';

  @override
  String get walletSwapTargetAssetHint => 'Velg en eiendel å motta';

  @override
  String get walletSwapQuoteButton => 'Hent kurs';

  @override
  String get walletSwapQuoting => 'Henter kurs…';

  @override
  String get walletSwapExecuting => 'Starter byttet ditt…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Fortsatt i gang — byttet starter. Dette kan ta opptil ett minutt.';

  @override
  String get walletSwapReviewTitle => 'Bekreft bytte';

  @override
  String get walletSwapYouSendLabel => 'Du sender';

  @override
  String get walletSwapYouReceiveLabel => 'Du mottar minst';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Nettverksgebyr';

  @override
  String get walletSwapNetworkFeeValue => 'Legges til når innskuddet sendes';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Kursen er gyldig i omtrent $time til — bekreft før den utløper.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Kursen er gyldig i mindre enn ett minutt til — bekreft før den utløper.';

  @override
  String get walletSwapQuoteExpired =>
      'Denne kursen har utløpt. Gå tilbake og hent en ny — kursen er ikke lenger garantert, og å sende nå kan føre til refusjon.';

  @override
  String get walletCountdownUnderMinute => 'mindre enn ett minutt';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes min';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds sek';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '$hours t $minutes min';
  }

  @override
  String get walletSwapDeshieldTitle => 'Dette byttet er ikke privat';

  @override
  String get walletSwapDeshieldBody =>
      'Å bytte ut avskjermer ZEC-en din — innskuddet er en offentlig transaksjon, og leverandørens side er offentlig på sitt nettverk.';

  @override
  String get walletSwapDiscloseTitle => 'Hva byttleverandøren vil se';

  @override
  String get walletSwapDiscloseAmounts => 'Beløpene på begge sider';

  @override
  String get walletSwapDiscloseCrossLink =>
      'At denne ZEC-en og eiendelen du mottar hører til ett bytte';

  @override
  String get walletSwapDiscloseDestination => 'Destinasjonsadressen din';

  @override
  String get walletSwapDiscloseSource => 'Kildeadressen din';

  @override
  String get walletSwapDiscloseIp =>
      'IP-adressen din (med mindre du ruter gjennom Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'Andre detaljer om dette byttet';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'Leverandørens egne transaksjoner er offentlige på sitt nettverk';

  @override
  String get walletSwapAckLabel =>
      'Jeg forstår at leverandøren vil se informasjonen ovenfor.';

  @override
  String get walletSwapConfirmButton => 'Start bytte';

  @override
  String get walletSwapBackButton => 'Tilbake';

  @override
  String get walletSwapStatusPendingTitle => 'Bytte startet';

  @override
  String get walletSwapStatusCheckingTitle => 'Sjekker byttestatus…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Lommeboken din sender ZEC-innskuddet til leverandøren. Er du kort offline, sendes det automatisk så snart du er tilkoblet igjen — men sendevinduet er kort, og lukkes det først, avsluttes byttet rett og slett, og ingenting byttes. ZEC-en din forblir din, og det kan ta opptil en time før den vises som tilgjengelig igjen.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'Venter på at innskuddet ditt skal komme inn. Hvis du ikke har sendt midlene fra den andre lommeboken din ennå, send dem før kursen utløper.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Dette byttet venter fortsatt på innskuddet sitt. Innskuddsinstruksjonene er ikke lenger tilgjengelige på denne enheten — hvis du allerede har sendt midlene, blir de oppdaget; hvis ikke, la dette byttet utløpe og start et nytt.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'Innskuddsvinduet avsluttes $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'Innskuddsvinduet er utløpt. Hvis innskuddet ikke ble sendt i tide, avsluttes byttet, og ZEC-en din blir værende i lommeboken din.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'Innskuddsvinduet er utløpt. Hvis du ikke har sendt innskuddet ditt, avsluttes dette byttet rett og slett — hent en ny kurs når du er klar.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Bytter pågår',
      one: 'Bytte pågår',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'ZEC-en din er på vei til leverandøren.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'Venter på at innskuddet ditt skal nå leverandøren.';

  @override
  String get walletSwapInFlightRowGeneric => 'Et bytte pågår.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'Innskuddsvinduet er utløpt — sjekk statusen til dette byttet.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Dette byttet har ikke nådd et bekreftet utfall her ennå — åpne det for å sjekke. ZEC som kommer tilbake til denne lommeboken, vises i saldoen din etter en synkronisering.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Dette byttet har ikke nådd et bekreftet utfall her ennå — åpne det for å sjekke. ZEC som dette byttet leverer til denne lommeboken, vises i saldoen din etter en synkronisering.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Bytte fullført.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Bytte refundert.';

  @override
  String get walletSwapRowOutcomeFailed => 'Bytte ikke fullført.';

  @override
  String get walletSwapRemove => 'Fjern';

  @override
  String get walletSwapRemoveTitle => 'Fjerne dette byttet fra listen?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Dette fjerner bare byttet fra denne listen — det avbryter ikke byttet, og denne lommeboken slutter å spore refusjonen. ZEC som refunderes senere, tilhører fortsatt denne lommeboken; en full ny skanning kan finne den.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Dette fjerner bare byttet fra denne listen — det avbryter ikke byttet, og denne lommeboken slutter å spore innkommende ZEC. ZEC som leveres senere, tilhører fortsatt denne lommeboken; en full ny skanning kan finne den. Hvis byttet i stedet refunderes, går refusjonen tilbake i eiendelen du sendte, utenfor denne lommeboken.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Dette fjerner bare byttet fra denne listen — det avbryter ikke byttet, og denne lommeboken slutter å spore ZEC som fortsatt kommer inn fra det. ZEC som kommer inn senere, tilhører fortsatt denne lommeboken; en full ny skanning kan finne den.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Dette fjerner det fullførte byttet fra listen.';

  @override
  String get walletSwapRemoveCancel => 'Avbryt';

  @override
  String get walletSwapRemoveConfirm => 'Fjern';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Startet $time';
  }

  @override
  String get walletSwapViewSwap => 'Vis bytte';

  @override
  String get walletSwapsInFlightError =>
      'Kunne ikke laste de pågående byttene dine akkurat nå.';

  @override
  String get walletSwapsInFlightRetry => 'Prøv igjen';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Prøver…';

  @override
  String get walletSwapStartAnother => 'Start et bytte til';

  @override
  String get walletSwapStatusUnderTitle => 'Venter på hele innskuddet';

  @override
  String get walletSwapStatusUnderBody =>
      'En del av innskuddet har kommet inn. Resten fullføres, eller leverandøren refunderer.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'En del av innskuddet ditt har kommet inn. Send det manglende beløpet før fristen, ellers refunderer leverandøren det som kom inn.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Mottatt $received; $missing mangler fortsatt. Innskuddsvinduet avsluttes: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Innskudd mottatt';

  @override
  String get walletSwapStatusDetectedBody =>
      'Leverandøren mottok innskuddet ditt og vil behandle byttet.';

  @override
  String get walletSwapStatusProcessingTitle => 'Behandler byttet ditt';

  @override
  String get walletSwapStatusProcessingBody =>
      'Leverandøren fullfører byttet ditt.';

  @override
  String get walletSwapStatusSuccessTitle => 'Bytte fullført';

  @override
  String get walletSwapStatusSuccessBody => 'Byttet ditt ble fullført.';

  @override
  String get walletSwapStatusRefundedTitle => 'Bytte refundert';

  @override
  String get walletSwapStatusRefundedBody =>
      'Byttet ble ikke fullført, så leverandøren sendte midlene tilbake til refusjonsadressen din.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'Byttet ble ikke fullført, så leverandøren sendte ZEC-en din tilbake til denne lommeboken. Den kommer inn som uskjermede midler og vises i saldoen din etter at lommeboken synkroniserer neste gang — dette kan ta litt tid.';

  @override
  String get walletSwapStatusFailedTitle => 'Bytte mislyktes';

  @override
  String get walletSwapStatusFailedBody =>
      'Byttet kunne ikke fullføres. Eventuelle innskutte midler gjøres opp eller refunderes hos leverandøren.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Bytte ikke funnet';

  @override
  String get walletSwapStatusNotFoundBody =>
      'Leverandøren har ikke lenger noen registrering av dette byttet — det har mest sannsynlig utløpt. Hvis et innskudd ble gjort, bør leverandøren refundere det til refusjonsadressen. Byttet blir værende i listen din, og denne lommeboken fortsetter å spore ZEC-en dens i tilfelle den fortsatt kommer fram; du kan fjerne det fra listen når som helst.';

  @override
  String get walletSwapStatusUnknownTitle => 'Status utilgjengelig';

  @override
  String get walletSwapStatusUnknownBody =>
      'Vi kan ikke lese statusen til dette byttet akkurat nå.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'Sporing utilgjengelig';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'Bytte er slått av, så vi kan ikke spore dette her. Eventuelle midler gjøres opp eller refunderes hos leverandøren.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'Bytte er slått av her, så dette byttet kan ikke spores akkurat nå. Hvis det ble refundert, kommer ZEC-en tilbake til denne lommeboken — den vises i saldoen din etter at bytte slås på igjen og lommeboken synkroniseres.';

  @override
  String get walletSwapTrackingError => 'Vi kunne ikke spore dette byttet.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Vi kunne ikke åpne sporing for dette byttet. Selve byttet kan fortsatt være i gang — eventuelle innskutte midler gjøres opp eller refunderes hos leverandøren.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Skriv inn adressen der du vil motta den byttede eiendelen.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Denne destinasjonsadressen er ikke gyldig for denne eiendelen. Sjekk den og prøv igjen.';

  @override
  String get walletSwapFaultExpired =>
      'Denne kursen utløp. Hent en ny kurs for å fortsette.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'Leverandørens pris beveget seg utenfor grensen din, så byttet ble stoppet før noe ble flyttet. Prøv igjen.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'Slippasjegrensen er for høy for et trygt bytte. Prøv igjen.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'Byttleverandøren er utilgjengelig akkurat nå. Prøv igjen om et øyeblikk.';

  @override
  String get walletSwapFaultConnection =>
      'Fikk ikke kontakt med byttetjenesten. Sjekk internettforbindelsen din og prøv igjen.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'Byttleverandøren returnerte et uventet svar, så byttet ble stoppet. Prøv igjen.';

  @override
  String get walletSwapFaultSwapOff => 'Bytte er slått av akkurat nå.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Vi kunne ikke sende innskuddet ditt, så ingenting forlot lommeboken. Hent en ny kurs for å prøve igjen.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Et bytte pågår allerede. Du kan starte et nytt etter at det er fullstendig gjort opp eller kursen utløper – dette kan ta en stund.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Denne lommeboken kan ikke sette opp en refusjonsadresse ennå — det betyr som regel bare at den første synkroniseringen ikke er ferdig. Vent til synkroniseringen er fullført, og prøv igjen.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Denne lommeboken kan ikke sette opp en mottaksadresse for dette byttet ennå — det betyr som regel bare at den første synkroniseringen ikke er ferdig. Vent til synkroniseringen er fullført, og prøv igjen.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'Byttet kunne ikke starte i tide — tilkoblingen kan ha vært treg, eller lommeboken var opptatt. Hent en ny kurs og prøv igjen.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'Lommeboken er opptatt et øyeblikk. Prøv igjen.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Denne kursen stemmer ikke med den lommeboken din utstedte, så ingenting ble sendt. Hent en ny kurs og prøv igjen.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Dette byttet krever omtrent $needed ZEC inkludert nettverksgebyret, men bare $spendable ZEC er tilgjengelig akkurat nå.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Denne appen begrenser for øyeblikket bytter til $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Dette byttet krever omtrent $needed ZEC inkludert nettverksgebyret, men bare $spendable ZEC er tilgjengelig akkurat nå. Saldoen din tar fortsatt igjen — mer kan bli tilgjengelig snart.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'Lommeboken kunne ikke registrere dette byttet trygt, så ingenting ble flyttet. Prøv igjen.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Denne bytteforespørselen kunne ikke behandles. Hent en ny kurs og prøv igjen.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Kunne ikke hente en byttekurs. Sjekk detaljene og prøv igjen.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Lommeboken din er ikke klar akkurat nå. Gå tilbake og prøv igjen.';

  @override
  String get walletSwapDirectionBuy => 'Kjøp ZEC';

  @override
  String get walletSwapDirectionSell => 'Selg ZEC';

  @override
  String get walletSwapRefundLabel => 'Refusjonsadressen din';

  @override
  String get walletSwapRefundHint =>
      'Hvor myntene dine returneres hvis byttet mislykkes';

  @override
  String get walletSwapRefundHelper =>
      'På kjeden du sender fra — ikke en Zcash-adresse.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Din $chain-refusjonsadresse';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'En $chain-adresse — dit myntene dine returneres hvis byttet mislykkes. Ikke en Zcash-adresse.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'Om refusjonsadressen din';

  @override
  String get walletSwapRefundInfoBody =>
      'Hvis byttet ikke kan fullføres, sender leverandøren myntene dine tilbake til denne adressen på kjeden du betalte fra. Skriv inn en adresse du kontrollerer — lommeboken kan ikke sjekke en fremmed adresse for deg, så verifiser den nøye.';

  @override
  String get walletSwapRefundScanTooltip =>
      'Skann en QR-kode for refusjonsadresse';

  @override
  String get walletSwapScanTitle => 'Skann adresse';

  @override
  String get walletSwapScanInstruction =>
      'Rett kameraet mot QR-koden for adressen.';

  @override
  String get walletSwapScanManualEntry => 'Skriv inn manuelt';

  @override
  String get walletSwapScanCancel => 'Avbryt';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Kameraet er utilgjengelig. Skriv inn adressen manuelt nedenfor.';

  @override
  String get walletSwapSourceAssetLabel => 'Eiendel å bytte fra';

  @override
  String get walletSwapSourceAssetHint => 'Velg en eiendel';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Beløp å sende ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Beløp å sende';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol på $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Velg en eiendel å bytte fra';

  @override
  String get walletSwapPickerTitleReceive => 'Velg en eiendel å motta';

  @override
  String get walletSwapPickerStale =>
      'Kunne ikke oppdatere eiendelslisten — viser den sist kjente listen.';

  @override
  String get walletSwapPickerEmpty =>
      'Ingen eiendeler er tilgjengelige for bytte akkurat nå. Prøv igjen senere.';

  @override
  String get walletSwapPickerSearchHint => 'Søk etter navn eller kjede';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Ingen eiendeler samsvarer med «$query».';
  }

  @override
  String get walletSwapPickerError =>
      'Kunne ikke laste eiendelslisten. Sjekk tilkoblingen din og prøv igjen.';

  @override
  String get walletSwapPickerRetry => 'Prøv igjen';

  @override
  String get walletSwapSlippageLabel => 'Slippasjetoleranse';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Egendefinert';

  @override
  String get walletSwapSlippageCustomLabel => 'Egendefinert slippasje';

  @override
  String get walletSwapSlippageMayFail =>
      'Svært lav — byttet kan mislykkes hvis prisen endrer seg.';

  @override
  String get walletSwapSlippageNormal => 'En trygg toleranse.';

  @override
  String get walletSwapSlippageRisky =>
      'Høy — du kan motta merkbart mindre enn oppgitt.';

  @override
  String get walletSwapSlippageTooHigh =>
      'For høy — byttet vil bli avvist. Sett den til 10 % eller lavere.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Du vil motta minst $zec ZEC — din slippasjegrense på $slippage%. Sluttbeløpet vil ikke falle under dette.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'Du mottar ZEC til din egen adresse';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Inntil du skjermer det — ett trykk, du blir varslet ved ankomst — er det mottatte beløpet kortvarig offentlig og synlig på kjeden. En liten levering kan forbli offentlig til den akkumuleres.';

  @override
  String get walletSwapRefundVerifyTitle => 'Verifiser refusjonsadressen din';

  @override
  String get walletSwapRefundVerifyBody =>
      'Sjekk den tegn for tegn — dette er hvor myntene dine returneres hvis byttet mislykkes. Lommeboken kan ikke verifisere en fremmed adresse for deg.';

  @override
  String get walletSwapRefundVerifyAck =>
      'Jeg har sjekket at refusjonsadressen min er riktig.';

  @override
  String get walletSwapPayoutVerifyTitle => 'Verifiser mottaksadressen din';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Sjekk den tegn for tegn — det er her du mottar $asset. Lommeboken kan ikke verifisere en fremmed adresse for deg.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'Jeg har sjekket at mottaksadressen min er riktig.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'Bytte er slått av her. Eventuell ZEC som allerede er på vei, vises i lommeboken din etter neste synkronisering.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Skriv inn beløpet du vil bytte.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Skriv inn refusjonsadressen din på kildekjeden.';

  @override
  String get walletSwapDepositTitle => 'Send betalingen din';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Send nøyaktig $amount $asset på $chain til adressen nedenfor.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Send det nøyaktige beløpet. Sender du mindre, eller sender du etter at vinduet er stengt, refunderer leverandøren deg til refusjonsadressen din.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Innskuddsvindu: $time igjen';
  }

  @override
  String get walletSwapDepositExpired =>
      'Dette innskuddsvinduet er stengt. Ikke send midler nå — start et nytt bytte. Hvis du allerede har sendt, bør leverandøren refundere til refusjonsadressen din.';

  @override
  String get walletSwapDepositQrLabel => 'QR-kode for innskuddsadressen';

  @override
  String get walletSwapDepositAddressLabel => 'Innskuddsadresse';

  @override
  String get walletSwapDepositCopy => 'Kopier innskuddsadresse';

  @override
  String get walletSwapDepositCopied => 'Innskuddsadresse kopiert';

  @override
  String get walletSwapDepositMemoRequired =>
      'Dette innskuddet krever et memo/en tag';

  @override
  String get walletSwapDepositMemoWarning =>
      'Du MÅ inkludere nøyaktig dette memoet med innskuddet ditt. Å sende uten det — eller med feil memo — kan føre til at du mister midlene dine permanent.';

  @override
  String get walletSwapDepositMemoLabel => 'Innskuddsmemo/tag';

  @override
  String get walletSwapDepositMemoCopy => 'Kopier memo';

  @override
  String get walletSwapDepositMemoCopied => 'Memo kopiert';

  @override
  String get walletSwapDepositSent => 'Jeg har sendt midlene';

  @override
  String get walletSwapDepositBackTitle => 'Forlate denne skjermen?';

  @override
  String get walletSwapDepositBackBody =>
      'Dette avbryter ikke byttet ditt — det fortsetter i bakgrunnen. Men du trenger innskuddsadressen for å betale, så kopier den først hvis du ikke allerede har gjort det.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Dette avbryter ikke byttet ditt — det fortsetter i bakgrunnen. Innskuddsvinduet er stengt. Ikke send midler til innskuddsadressen nå. Hvis du allerede har sendt, bør leverandøren refundere til refusjonsadressen din.';

  @override
  String get walletSwapDepositBackStay => 'Bli';

  @override
  String get walletSwapDepositBackLeave => 'Forlat';

  @override
  String get walletReceive => 'Motta';

  @override
  String get walletReceiveSubtitle =>
      'Del denne adressen for å motta ZEC. Det er trygt å dele den offentlig.';

  @override
  String get walletReceiveCopy => 'Kopier adresse';

  @override
  String get walletReceiveCopied => 'Adresse kopiert';

  @override
  String get walletReceiveUnavailable => 'Lommeboken din er ikke klar ennå.';

  @override
  String get walletReceiveError =>
      'Vi kunne ikke laste adressen din. Prøv igjen.';

  @override
  String get walletReceivePreparing => 'Forbereder adressen din…';

  @override
  String get walletReceivePreparingHint =>
      'Lommeboken din forbereder denne adressen på enheten din — dette kan ta et øyeblikk hvis lommeboken er opptatt med annet arbeid.';

  @override
  String get walletReceiveRetry => 'Prøv igjen';

  @override
  String get walletReceiveQrLabel => 'QR-kode for mottaksadressen din';

  @override
  String get walletReceiveTypeShielded => 'Skjermet';

  @override
  String get walletReceiveTypeTransparent => 'Offentlig';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Del denne offentlige adressen for å motta ZEC fra en avsender som ikke kan betale til en skjermet adresse.';

  @override
  String get walletReceiveTransparentWarning =>
      'Dette er en offentlig adresse: den er synlig på kjeden og kobler sammen betalingene dine hvis den gjenbrukes. Foretrekk den skjermede adressen din; skjerm disse midlene etter mottak.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'QR-kode for din offentlige mottaksadresse';

  @override
  String get walletReceiveFreshAddress => 'Bruk en ny adresse';

  @override
  String get walletReceiveFreshCaption =>
      'Ny adresse — kan ikke kobles til dine andre adresser. Betalinger hit havner fortsatt i denne lommeboken, og de tidligere adressene dine fungerer fremdeles. Den vises ikke her igjen — kopier den nå.';

  @override
  String get walletReceiveFreshError =>
      'Vi kunne ikke opprette en ny adresse. Prøv igjen.';

  @override
  String get walletReceiveFreshBusy =>
      'Lommeboken er opptatt akkurat nå. Prøv den nye adressen igjen om et øyeblikk.';

  @override
  String get walletReceiveShare => 'Del';

  @override
  String get walletReceiveRequestAmount => 'Be om beløp';

  @override
  String get walletReceiveRequestAmountLabel => 'Beløp (valgfritt)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Den vises ikke her igjen — kopier den nå.';

  @override
  String get walletSecurityMenuItem => 'Sikkerhet…';

  @override
  String get securityTitle => 'Sikkerhet';

  @override
  String get securityUnavailableBody =>
      'Sikkerhetsinnstillinger for lommeboken administreres av denne appen, ikke av lommeboken selv.';

  @override
  String get securityCustodySectionTitle => 'Nøkkelforvaring';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (maskinvare)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (maskinvare)';

  @override
  String get securityCustodyTierTee => 'Maskinvarebasert nøkkellager (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Programvarebasert nøkkellager';

  @override
  String get securityCustodyTierKeychain => 'Keychain (programvarekryptert)';

  @override
  String get securityCustodyTierNone => 'Ingen maskinvarebasert nøkkellager';

  @override
  String get securityCustodyTierUnknown => 'Ukjent';

  @override
  String get securityCustodyHardwareKey =>
      'Nøkkelen som låser denne lommeboken, ligger i den sikre maskinvaren på denne enheten og slettes sammen med lommeboken.';

  @override
  String get securityCustodyBestEffort =>
      'Sletting fjerner nøklene dine etter beste evne; et kort tidsvindu for rettsteknisk gjenoppretting kan gjenstå til enheten gjenbruker lagringsplassen. For full sikkerhet, bruk også enhetens «Slett alt innhold og alle innstillinger».';

  @override
  String get securityCustodyProbeError =>
      'Kunne ikke lese forvaringsstatusen. Gå tilbake og prøv igjen.';

  @override
  String get securityDeleteWalletButton => 'Slett lommebok';

  @override
  String get securityDeleteWalletSubtitle =>
      'Slett denne lommeboken og nøkkelen dens fra denne enheten. Midlene dine forblir på kjeden og kan gjenopprettes fra gjenopprettingsfrasen din.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Slett denne lommeboken og nøkkelen dens fra denne enheten. Den har ingen sendenøkler, så det er ingenting å ta sikkerhetskopi av — legg den til igjen når som helst med innsynsnøkkelen.';

  @override
  String get securityDeleteDialogTitle => 'Slette denne lommeboken?';

  @override
  String get securityDeleteDialogBody =>
      'Dette fjerner lommeboken og nøkkelen dens fra denne enheten. Sørg for at du har tatt sikkerhetskopi av gjenopprettingsfrasen din — det er den ENESTE måten å gjenopprette midlene dine på.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Dette fjerner lommeboken og nøkkelen dens fra denne enheten. Den har ingen sendenøkler, så ingenting trenger sikkerhetskopiering — du kan legge den til igjen senere med innsynsnøkkelen.';

  @override
  String get securityDeleteDialogConfirm => 'Slett';

  @override
  String get securityDeleteDialogCancel => 'Avbryt';

  @override
  String get securityDeleteFailedSnack =>
      'Kunne ikke slette lommeboken — lommeboken din er uendret. Prøv igjen.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Fullfør byttet av server først — det fullføres eller stopper innen $seconds sekunder. Prøv deretter å slette lommeboken igjen.';
  }

  @override
  String get walletParkedTitle => 'Lagret og under behandling';

  @override
  String get walletParkedSubtitle =>
      'Disse betalingene er ikke sendt ennå. Beløpene deres er fortsatt en del av saldoen din.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Disse betalingene er ikke sendt ennå. Beløpene deres er fortsatt en del av saldoen din — bortsett fra dem lommeboken din holder på å sende, der beløpet kan allerede være reservert.';

  @override
  String get walletParkedCancel => 'Avbryt';

  @override
  String get walletParkedPausedHint =>
      'Pauset — denne betalingen sendes ikke av seg selv. Midlene dine er trygge. Send den nå, eller avbryt den.';

  @override
  String get walletParkedRetryStale =>
      'Denne betalingen venter ikke lenger. Sjekk betalingene dine under behandling og aktiviteten din.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Denne betalingen venter ikke lenger — lommeboken din sender den kanskje allerede. Sjekk Lagret og under behandling og aktiviteten din.';

  @override
  String get walletReclaimExplainer =>
      'Sendinger gjennom engangsadresse sitter fast. Du kan gjenåpne dem — det flytter et lite beløp mellom dine egne adresser og fører det tilbake.';

  @override
  String get walletReclaimButton => 'Gjenåpne sending';

  @override
  String get walletReclaimInProgress => 'Gjenåpner…';

  @override
  String get walletReclaimConfirmTitle =>
      'Gjenåpne sending gjennom engangsadresse?';

  @override
  String get walletReclaimConfirmBody =>
      'Dette flytter et lite beløp mellom dine egne adresser for å frigjøre sending gjennom engangsadresse, og fører det så tilbake. Det koster et par nettverksgebyrer. Når det er bekreftet, kan du gjenopprette det flyttede beløpet med Gjenopprett nå.';

  @override
  String get walletReclaimConfirmCancel => 'Ikke nå';

  @override
  String get walletReclaimConfirmAction => 'Gjenåpne';

  @override
  String get walletReclaimStarted =>
      'Gjenåpning startet. Når det er bekreftet, send den pausede betalingen, og gjenopprett deretter det flyttede beløpet med Gjenopprett nå.';

  @override
  String get walletReclaimNothing => 'Ingenting å gjenåpne akkurat nå.';

  @override
  String get walletReclaimNotBroadcast =>
      'Kunne ikke bekrefte at sendingen nådde nettverket. Den kan likevel ha gått gjennom — vent litt før du prøver igjen.';

  @override
  String get walletReclaimNeedsFunds =>
      'Du trenger noe skjermet ZEC for å gjenåpne sending.';

  @override
  String get walletReclaimFailed =>
      'Kunne ikke gjenåpne sending akkurat nå. Midlene dine er uendret. Prøv igjen.';

  @override
  String get walletReclaimUnknown =>
      'Gjenåpning avsluttet. Sjekk sendingene dine gjennom engangsadresse, og gjenopprett et eventuelt flyttet beløp med Gjenopprett nå.';

  @override
  String get walletParkedError =>
      'Kunne ikke laste betalingene dine under behandling akkurat nå.';

  @override
  String get walletParkedErrorRetry => 'Prøv igjen';

  @override
  String get walletParkedErrorRetryInProgress => 'Prøver…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Avbryte denne betalingen under behandling?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Dette forkaster den lagrede betalingen. Den er ikke sendt, så ingenting forlater lommeboken — men dette kan ikke angres.';

  @override
  String get walletParkedCancelConfirmKeep => 'Behold den';

  @override
  String get walletParkedCancelConfirmDiscard => 'Forkast betaling';

  @override
  String get walletParkedCancelDone => 'Betaling under behandling avbrutt.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Denne betalingen kan allerede være på vei — sjekk aktiviteten din.';

  @override
  String get walletParkedCancelFailed =>
      'Kunne ikke avbryte akkurat nå. Betalingen din er uendret. Prøv igjen.';

  @override
  String get walletRecoverNow => 'Gjenopprett nå';

  @override
  String get walletRecoverConfirmTitle =>
      'Gjenopprette til den skjermede saldoen din?';

  @override
  String get walletRecoverConfirmBody =>
      'Dette sjekker engangsadressene dine og flytter alt som blir funnet inn i den private, skjermede saldoen din. Det er trygt å kjøre dette igjen når som helst.';

  @override
  String get walletRecoverConfirmCancel => 'Ikke nå';

  @override
  String get walletRecoverConfirmAction => 'Gjenopprett';

  @override
  String get walletRecoverInProgress => 'Gjenoppretter…';

  @override
  String walletRecoverDone(String amount) {
    return 'Gjenoppretter $amount til den skjermede saldoen din.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'Gjenoppretter $amount — noen midler trenger fortsatt et nytt forsøk.';
  }

  @override
  String get walletRecoverRetry =>
      'Noen midler trenger et nytt forsøk — kjør gjenoppretting på nytt.';

  @override
  String get walletRecoverTruncated =>
      'Ikke alle engangsadresser ble sjekket ennå — kjør det på nytt for å sjekke resten.';

  @override
  String get walletRecoverNothing => 'Ingenting å gjenopprette akkurat nå.';

  @override
  String get walletRecoverFailed =>
      'Kunne ikke gjenopprette akkurat nå. Midlene dine er uendret. Prøv igjen.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount lagret og under behandling · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Avbryt betalingen på $amount lagret $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount pauset · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount klargjøres for sending · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Lommeboken din forbereder denne betalingen — beløpet kan allerede være reservert. Midlene dine er trygge. Hvis den ikke fullføres, går den tilbake til listen av seg selv.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Lommeboken din forbereder denne betalingen — beløpet kan allerede være reservert. Midlene dine er trygge, men betalingen kan først fullføres når lommeboken din synkroniserer igjen.';

  @override
  String get walletParkedSendNow => 'Send nå';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Sender betalingen på $amount lagret $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Send nå betalingen på $amount lagret $time';
  }

  @override
  String get walletParkedSendNowInProgress => 'Sender…';

  @override
  String get walletParkedAuthorizeSent => 'Sender betalingen din nå.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Sender betalingen din nå. Hvis den ikke går gjennom, kan lommeboken først fullføre den når den synkroniserer igjen.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Ikke klar til å sendes ennå. Betalingen din er lagret og uendret.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Ikke klar til å sendes ennå. Betalingen din er lagret og ikke lenger pauset — prøv Send nå på nytt senere, eller avbryt den.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Kunne ikke sende akkurat nå. Betalingen din er uendret. Prøv igjen.';

  @override
  String get walletTransparentFundsMenuItem => 'Offentlige midler…';

  @override
  String get walletTransparentFundsTitle => 'Offentlige midler';

  @override
  String get walletTransparentFundsIntro =>
      'Offentlige midler er offentlig synlige på blokkjeden — beløpet, adressene og myntenes historie.';

  @override
  String get walletExpertToggleLabel => 'Avansert: offentlige midler';

  @override
  String get walletExpertToggleDescription =>
      'Vis avanserte kontroller for å holde offentlige midler og slå av automatisk skjerming.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Vis avanserte kontroller for å holde offentlige midler.';

  @override
  String get walletAutoShieldToggleLabel => 'Skjerm automatisk';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Når den offentlige saldoen din når $minZec ZEC, flyttes den automatisk inn i den skjermede saldoen din. Når dette er av, forblir offentlige midler offentlig synlige til du skjermer dem selv.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Kunne ikke lagre innstillingen. Prøv igjen.';

  @override
  String get walletAutoShieldIncomplete =>
      'Automatisk skjerming ble ikke fullført — disse midlene er fortsatt offentlig synlige. Du kan skjerme dem nå.';

  @override
  String get walletSendPrivacyShielded =>
      'Skjermet betaling — beløpet og mottakeren forblir private på kjeden.';

  @override
  String get walletSendPrivacyTransparent =>
      'Offentlig betaling — beløpet og adressene er synlige på blokkjeden.';

  @override
  String get walletActivityPublicBadge => 'Offentlig synlig på kjeden';

  @override
  String get walletShieldWalletEnded =>
      'Lommebok-økten ble avsluttet. Lukk og åpne på nytt for å prøve igjen.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Nye offentlige midler skjermes automatisk inn i din private saldo når de når $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'Automatisk skjerming er av — offentlige midler forblir offentlig synlige til du skjermer dem.';

  @override
  String get walletMoveAutoShieldNote =>
      'Automatisk skjerming er på: etter at disse midlene har kommet inn, blir de skjermet igjen automatisk (mot et nytt gebyr). For å holde dem offentlige, slå først av automatisk skjerming under Offentlige midler.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Etter denne flyttingen blir den offentlige saldoen din $amount ZEC — under de $floor ZEC som trengs for å skjerme den igjen. Den forblir offentlig til mer kommer inn.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Du flytter til din egen offentlige adresse. Denne flyttingen blir stående i det offentlige registeret permanent.';

  @override
  String get walletTxDetailVisibility => 'Synlighet';

  @override
  String get walletTransparentFundsAutoDenied =>
      'Automatisk skjerming er satt på pause for denne økten — den ble ikke godkjent. Du kan fortsatt skjerme midlene manuelt.';

  @override
  String get walletDeepScanMenuItem => 'Sjekk eldre bytteadresser…';

  @override
  String get walletMenuSyncNotRunningHint =>
      'Synkronisering kjører ikke akkurat nå.';

  @override
  String get walletDeepScanTitle => 'Sjekk eldre bytteadresser';

  @override
  String get walletDeepScanBody =>
      'Hvis du gjenopprettet denne lommeboken og den tidligere brukte bytte mye, kan midler fra de eldste byttene ta et ekstra steg å finne. Dette sjekker for det — alt som blir funnet, vises i saldoen din etter hvert som lommeboken synkroniserer.';

  @override
  String get walletDeepScanCoverage =>
      'Dine eldre bytteadresser er sjekket fram til her. Hvis midler fra et gammelt bytte fortsatt mangler, kan du sjekke enda dypere.';

  @override
  String get walletDeepScanCoveragePending =>
      'Sjekker fortsatt det gjeldende intervallet — alt som blir funnet, vises i saldoen din. Dette kan ta en liten stund.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Sjekker etter midler fra lommebokens eldste bytter.';

  @override
  String get walletDeepScanCheckButton => 'Sjekk eldre adresser';

  @override
  String get walletDeepScanCheckDeeperButton => 'Sjekk enda eldre adresser';

  @override
  String get walletDeepScanChecking => 'Sjekker…';

  @override
  String get walletDeepScanClose => 'Lukk';

  @override
  String get walletDeepScanTorHint =>
      'Du er ikke koblet til via Tor akkurat nå. For mer personvern kan du vurdere å vente til Tor er aktivt før du sjekker.';

  @override
  String get walletDeepScanRescanBusy =>
      'Du kan sjekke eldre bytteadresser når den nye skanningen er ferdig.';

  @override
  String get walletDeepScanRan =>
      'Sjekker eldre bytteadresser — alt som blir funnet, vises i saldoen din.';

  @override
  String get walletDeepScanFailed =>
      'Kunne ikke starte sjekken. Ingenting ble endret — prøv igjen.';

  @override
  String get walletDeepScanSlow =>
      'Dette tar lengre tid enn vanlig. Hvis dine eldre bytteadresser ble sjekket, vises alt som blir funnet i saldoen din — sjekk igjen om en liten stund.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'Bytte er slått av akkurat nå, så dette kan ikke kjøres. Prøv igjen når bytte er tilgjengelig.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Sjekker fortsatt det siste intervallet — dette kan ta opptil et par dager, men vanligvis mye mindre. Det fullføres av seg selv — sjekk igjen senere.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'Vi kan ikke bekrefte personvernet til tilkoblingen din ennå. For mer personvern kan du vurdere å sjekke når Tor er aktivt.';

  @override
  String get walletDeepScanBannerChecking =>
      'Sjekker fortsatt eldre bytteadresser — alt som blir funnet, vises i saldoen din.';

  @override
  String get walletRescanSwapPointer =>
      'Leter du etter midler fra et gammelt bytte? En ny skanning finner ikke det — bruk «Sjekk eldre bytteadresser» i stedet.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Gjenopprettet du en lommebok som brukte bytte?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Hvis denne lommeboken hadde en svært lang byttehistorikk, kan midler fra de eldste byttene ta et ekstra steg å finne. De fleste lommebøker trenger ingenting.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Sjekk nå';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Lukk';

  @override
  String walletTorHostPath(String transport) {
    return 'Via appens private rute ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Via appens private rute ($transport); tilkoblinger kan knyttes sammen av proxyen';
  }

  @override
  String get walletTorHostOtherTransport => 'en privat rute';

  @override
  String get walletTorHostDirect => 'Ikke privat (appens direkte tilkobling)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Den lagrede serveren bruker en ukryptert adresse som appens private rute ikke kan bære. Bruker $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Mer om $label';
  }

  @override
  String get walletSendPaste => 'Lim inn';

  @override
  String get walletSendScanQr => 'Skann QR-kode';

  @override
  String get walletSendRecipientGetsLabel => 'Mottaker får';

  @override
  String get walletSwapDepositCopyAmount => 'Kopier beløp';

  @override
  String get walletSwapDepositAmountCopied => 'Beløp kopiert';

  @override
  String get walletScanOpenSettings => 'Åpne innstillinger';

  @override
  String get walletScanOpenSettingsFailed => 'Kunne ikke åpne innstillingene.';

  @override
  String get walletSendLeaveTitle => 'Sender fortsatt';

  @override
  String get walletSendLeaveBody =>
      'Betalingen fortsetter hvis du går. Du ser hvordan den endte i aktiviteten din.';

  @override
  String get walletSendLeaveStay => 'Bli';

  @override
  String get walletSendLeaveConfirm => 'Gå';

  @override
  String get walletSheetLeaveBody =>
      'Dette fortsetter hvis du går. Du ser hvordan det endte i aktiviteten din.';

  @override
  String get walletLoadingLabel => 'Laster inn';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'Sjekk før du skjermer igjen';

  @override
  String get walletShieldUnknownBody =>
      'Vi kunne ikke bekrefte denne skjermingen. Se Aktivitet før du prøver igjen.';

  @override
  String get walletMoveUnknownTitle => 'Sjekk før du flytter igjen';

  @override
  String get walletMoveUnknownBody =>
      'Vi kunne ikke bekrefte denne flyttingen. Se Aktivitet før du prøver igjen.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
