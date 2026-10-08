// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Finnish (`fi`).
class WalletLocalizationsFi extends WalletLocalizations {
  WalletLocalizationsFi([String locale = 'fi']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Asetukset';

  @override
  String get walletTitle => 'Lompakko';

  @override
  String get walletNotSetUpTitle => 'Lompakkoa ei ole vielä otettu käyttöön';

  @override
  String get walletNotSetUpBody =>
      'Lompakon käyttöönotto tulee myöhemmässä versiossa. Käyttöönotto opastaa sinut kirjoittamaan palautuslauseesi muistiin, ennen kuin varoja voidaan vastaanottaa — näin mitään ei koskaan vaarannu ilman varmuuskopiota.';

  @override
  String get walletStartupFailedTitle => 'Lompakko ei käynnistynyt';

  @override
  String get walletStartupFailedBody =>
      'Jokin esti lompakkoa latautumasta tällä laitteella. Jos sinulla on jo lompakko, sen varat eivät ole vaarassa — ne ovat Zcash-verkossa ja ne voidaan palauttaa palautuslauseesi avulla. Yritä uudelleen; jos tämä toistuu, sulje sovellus ja avaa se uudelleen.';

  @override
  String get walletBalanceLabel => 'Saldo';

  @override
  String get walletHideBalance => 'Piilota saldo';

  @override
  String get walletShowBalance => 'Näytä saldo';

  @override
  String get walletBalanceHiddenAmount => 'Saldo piilotettu';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Käytettävissä nyt';

  @override
  String get walletArrivingLabel => 'Saapumassa';

  @override
  String get walletNotSpendableYetLabel => 'Ei vielä käytettävissä';

  @override
  String get walletActivityTitle => 'Tapahtumat';

  @override
  String get walletActivityEmpty => 'Ei tapahtumia vielä';

  @override
  String get walletActivityError => 'Tapahtumien lataaminen epäonnistui';

  @override
  String get walletActivityReceived => 'Vastaanotettu';

  @override
  String get walletActivitySent => 'Lähetetty';

  @override
  String get walletActivityPending => 'Vireillä';

  @override
  String get walletActivityQueued => 'Jonossa';

  @override
  String get walletActivityRetrying => 'Yritetään uudelleen';

  @override
  String get walletActivitySaved => 'Tallennettu';

  @override
  String get walletActivityExpired => 'Vanhentunut';

  @override
  String get walletActivityFailed => 'Epäonnistunut';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count vahvistusta',
      one: '1 vahvistus',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count maksua vastaanotettu',
      one: 'Maksu vastaanotettu',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Näytä tapahtuman tiedot';

  @override
  String get walletTxDetailStatus => 'Tila';

  @override
  String get walletTxDetailFee => 'Verkkomaksu';

  @override
  String get walletTxDetailDate => 'Päivämäärä';

  @override
  String get walletTxDetailHeight => 'Lohkokorkeus';

  @override
  String get walletTxDetailMemo => 'Muistio';

  @override
  String get walletTxDetailMemoAttached => 'Mukana';

  @override
  String get walletTxDetailTxid => 'Tapahtumatunnus';

  @override
  String get walletTxDetailCopyTxid => 'Kopioi tapahtumatunnus';

  @override
  String get walletTxDetailCopied => 'Tapahtumatunnus kopioitu';

  @override
  String get walletTxDetailClose => 'Sulje';

  @override
  String get walletTxFundsKept => 'Varoja ei lähtenyt lompakostasi';

  @override
  String get walletTxExplainQueued =>
      'Tallennettu tälle laitteelle, kohtaan Tallennettu ja vireillä — voit lähettää tai peruuttaa sen siellä.';

  @override
  String get walletTxExplainPending =>
      'Lähetetty Zcash-verkkoon — odottaa vahvistusta lohkossa.';

  @override
  String get walletTxExplainRetrying =>
      'Lompakkosi ei vielä pystynyt lähettämään tätä Zcash-verkkoon. Se säilyttää allekirjoitetun tapahtuman ja yrittää uudelleen jokaisessa synkronoinnissa, kunnes se menee läpi tai vanhenee.';

  @override
  String get walletTxExplainSaved =>
      'Lompakkosi on säilyttänyt tämän allekirjoitetun tapahtuman, mutta ei tällä hetkellä lähetä sitä itsestään.';

  @override
  String get walletTxExplainConfirmed => 'Vahvistettu Zcash-verkossa.';

  @override
  String get walletTxExplainExpired =>
      'Tämä tapahtuma vanheni ennen kuin verkko ehti vahvistaa sen, joten se peruutettiin. Summa on edelleen käytettävissäsi.';

  @override
  String get walletTxExplainFailed =>
      'Verkko hylkäsi tämän tapahtuman, joten se ei mennyt läpi. Summa on edelleen käytettävissäsi.';

  @override
  String get walletTxExplainUnknown =>
      'Tämän tapahtuman tilaa ei voida tällä hetkellä määrittää. Tila päivittyy seuraavan synkronoinnin jälkeen.';

  @override
  String get walletMenuTooltip => 'Lisää asetuksia';

  @override
  String get walletRescanMenuItem => 'Skannaa historia uudelleen…';

  @override
  String get walletCheckOneTimeMenuItem =>
      'Tarkista kertakäyttöiset osoitteet…';

  @override
  String get walletRescanTitle => 'Skannaa historiasi uudelleen';

  @override
  String get walletRescanBody =>
      'Puuttuuko vanhempia varoja? Skannaa lohkoketju uudelleen kauempaa ajassa, jotta aiemman aloituspäivän ohittamat talletukset löytyvät. Varasi ja palautuslauseesi eivät ole koskaan vaarassa.';

  @override
  String get walletRescanRangeTitle => 'Kuinka kauas skannataan';

  @override
  String get walletRescanRangeAll =>
      'Skannaa koko historiasi — hitain vaihtoehto, mutta palauttaa kaiken.';

  @override
  String get walletRescanRangeDefault =>
      'Skannataan lompakkosi alusta alkaen. Puuttuuko silti vanhempia varoja? Valitse aiempi päivämäärä tai skannaa koko historia.';

  @override
  String get walletRescanRangeResolving =>
      'Valmistellaan suositeltua skannausväliä…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Noin $blocks lohkoa skannattavana.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Skannataan alkaen $date. Puuttuuko silti vanhempia varoja? Valitse aiempi päivämäärä tai skannaa koko historia.';
  }

  @override
  String get walletRescanPick => 'Valitse päivämäärä';

  @override
  String get walletRescanChange => 'Vaihda päivämäärää';

  @override
  String get walletRescanScanAll => 'Skannaa koko historia';

  @override
  String get walletRescanDatePick => 'Aikaisin skannattava päivämäärä';

  @override
  String get walletRescanWarning =>
      'Tämä skannaa lohkoketjun uudelleen. Lähiaikojen skannaus kestää minuutteja; kauas taaksepäin skannaaminen voi kestää tunteja. Synkronointi toimii taustalla — voit käyttää lompakkoa sillä aikaa.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Tästä lompakosta lähtevä maksu on vielä vahvistumassa. Lompakko yleensä kieltäytyy uudelleenskannauksesta, kunnes se on valmis — voit yrittää, mutta odota, että pyyntö hylätään.';

  @override
  String get walletRescanConfirm => 'Aloita uudelleenskannaus';

  @override
  String get walletRescanCancel => 'Peruuta';

  @override
  String get walletRescanRunning => 'Rakennetaan uudelleen…';

  @override
  String get walletRescanRebuildingAll =>
      'Historiaasi rakennetaan uudelleen — koko ketjua skannataan. Saldosi ja tapahtumasi täydentyvät sitä mukaa kuin skannaus etenee.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Historiaasi rakennetaan uudelleen alkaen $date — saldosi ja tapahtumasi täydentyvät sitä mukaa kuin skannaus etenee.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Historiaasi rakennetaan uudelleen lompakkosi alusta alkaen — saldosi ja tapahtumasi täydentyvät sitä mukaa kuin skannaus etenee.';

  @override
  String get walletCatchUpBanner =>
      'Kurotaan kiinni — saldosi ja tapahtumasi täydentyvät lompakon synkronoituessa. Kaikki vastaanottamasi on turvassa.';

  @override
  String get walletCatchUpRescanBanner =>
      'Historiaasi rakennetaan uudelleen uudelleenskannauksen jälkeen — saldosi ja tapahtumasi täydentyvät sitä mukaa kuin skannaus etenee. Kaikki vastaanottamasi on turvassa.';

  @override
  String get walletRescanFailedNotice =>
      'Uudelleenskannaus ei onnistunut juuri nyt — varasi ovat turvassa, vaikka saldosi ja historiasi voivat tarvita hetken aikaa kiriäkseen kiinni. Yritä hetken kuluttua uudelleen.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Maksu on vielä vahvistumassa, joten uudelleenskannaus on keskeytetty varojesi suojaamiseksi. Lompakkosi on ennallaan — yritä uudelleen parin tunnin kuluttua ja pidä sovellus auki ja verkossa sillä aikaa.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'Uudelleenskannaus rakentaa historiasi uudelleen sitä mukaa kun lompakkosi synkronoituu, eikä synkronointi ole juuri nyt käynnissä. Lompakkosi on ennallaan — yritä uudelleen, kun synkronointi on käynnissä.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Vapaata tilaa ei ole tarpeeksi lompakon historian uudelleenrakentamiseen — varasi ovat turvassa, vaikka saldosi ja historiasi voivat tarvita hetken aikaa kiriäkseen kiinni. Vapauta tilaa ja yritä uudelleen.';

  @override
  String get walletRescanFailedDismiss => 'Hylkää';

  @override
  String get walletActivityRebuilding => 'Historiaasi rakennetaan uudelleen…';

  @override
  String get walletActivityCatchingUp =>
      'Kurotaan yhä kiinni — kaikki vastaanottamasi näkyy täällä.';

  @override
  String get walletActivitySyncNotRunning =>
      'Saldosi ja historiasi latautuvat loppuun, kun synkronointi on käynnissä.';

  @override
  String get walletActivityLoadMore => 'Lataa lisää';

  @override
  String get walletPendingChangeLabel => 'Odottava vaihtoraha';

  @override
  String get walletTransparentLabel => 'Suojaamaton (julkinen)';

  @override
  String get walletTransparentNote =>
      'Ei sisälly kohtaan \"Käytettävissä nyt\" — suojaa nämä varat, jotta voit käyttää niitä. Siihen asti ne näkyvät julkisesti lohkoketjussa.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Nämä varat näkyvät julkisesti lohkoketjussa.';

  @override
  String walletPoolShielded(String amount) {
    return 'Suojattu $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Julkinen $amount';
  }

  @override
  String get walletPoolAllShielded => 'Kaikki suojattu · yksityinen';

  @override
  String get walletPoolTapHint => 'Näytä julkiset varat';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount saldostasi on kertakäyttöisellä osoitteella (palautettavissa).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount saldostasi on kertakäyttöisellä osoitteella.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Yhteensä $amount arvosta maksuja on varattu ja niitä viimeistellään yhä lompakkosi hallitsemien kertakäyttöisten osoitteiden kautta. Älä lähetä niitä uudelleen.',
      one:
          '$amount on varattu maksuun, jonka lompakkosi viimeistelee yhä sen hallitseman kertakäyttöisen osoitteen kautta. Älä lähetä sitä uudelleen.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Yhteensä $amount arvosta maksuja on varattu ja ne ovat puolimatkassa lompakkosi hallitsemien kertakäyttöisten osoitteiden kautta. Ne on keskeytetty, kunnes lompakkosi synkronoi jälleen. Älä lähetä niitä uudelleen.',
      one:
          '$amount on varattu maksuun, joka on puolimatkassa lompakkosi hallitseman kertakäyttöisen osoitteen kautta. Se on keskeytetty, kunnes lompakkosi synkronoi jälleen. Älä lähetä sitä uudelleen.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Ei voitu tarkistaa, onko maksu vielä kesken. Yritetään uudelleen – tarkista sillä välin toiminnastasi, onko siellä odottava maksu, ennen kuin lähetät uudelleen.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount saldostasi on kertakäyttöisellä osoitteella (vahvistuu vielä).';
  }

  @override
  String get walletShieldButton => 'Suojaa';

  @override
  String get walletShieldSheetTitle => 'Suojaa julkiset varat';

  @override
  String get walletShieldNote =>
      'Tämä siirtää varoja julkisesta, lohkoketjussa näkyvästä saldostasi yksityiseen suojattuun saldoosi.';

  @override
  String get walletShieldPreparing => 'Valmistellaan…';

  @override
  String get walletShieldAmountLabel => 'Suojataan';

  @override
  String get walletShieldFeeLabel => 'Verkkomaksu';

  @override
  String get walletShieldNetLabel => 'Saapuu suojattuna';

  @override
  String get walletShieldConfirmButton => 'Suojaa nyt';

  @override
  String get walletShieldSubmitting => 'Suojataan…';

  @override
  String get walletShieldNothingTitle => 'Ei vielä suojattavaa';

  @override
  String get walletShieldNothingBody =>
      'Nämä varat alittavat tällä hetkellä suojaamisen kannalta järkevän summan — verkkomaksu söisi hyödyn. Ne voi suojata, kun lisää varoja saapuu.';

  @override
  String get walletShieldDoneTitle => 'Suojaus lähetetty';

  @override
  String get walletShieldDoneBody =>
      'Varasi siirtyvät suojattuun saldoosi. Vahvistus lohkoketjussa tapahtuu pian.';

  @override
  String get walletShieldSavedTitle =>
      'Tallennettu — viimeistelemme suojauksen';

  @override
  String get walletShieldSavedBody =>
      'Emme tavoittaneet verkkoa juuri nyt. Suojauksesi on tallennettu, ja lompakkosi viimeistelee sen myöhemmässä synkronoinnissa. Mitään ei ole menetetty.';

  @override
  String get walletShieldAlreadyTitle => 'Jo lähetetty';

  @override
  String get walletShieldFailedTitle => 'Suojaus ei onnistunut juuri nyt';

  @override
  String get walletShieldStaleBody =>
      'Lompakko synkronoi vielä. Yritä suojausta uudelleen hetken kuluttua.';

  @override
  String get walletShieldTransientBody =>
      'Suojausta ei voitu valmistella juuri nyt. Yritä hetken kuluttua uudelleen.';

  @override
  String get walletShieldStorageFullBody =>
      'Vapaata tilaa ei ole tarpeeksi suojaamiseen juuri nyt. Vapauta tilaa ja yritä uudelleen. Varasi ovat turvassa.';

  @override
  String get walletShieldClose => 'Sulje';

  @override
  String get walletShieldRetry => 'Yritä uudelleen';

  @override
  String get walletMoveMenuItem => 'Siirrä julkiseksi…';

  @override
  String get walletMoveSheetTitle => 'Siirrä julkiseksi';

  @override
  String get walletMoveSheetSubtitle =>
      'Lähetä suojattua ZEC:iä omalle julkiselle osoitteellesi — hyödyllistä, jos pörssi ei hyväksy suojattua talletusta.';

  @override
  String get walletMoveDestinationLabel => 'Julkinen osoitteesi';

  @override
  String walletMoveAvailable(String amount) {
    return 'Siirrettävissä: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Siirrettävissä: $amount ZEC — saldosi kurottaa yhä kiinni';
  }

  @override
  String get walletMoveDeshieldTitle => 'Tämä siirto tekee varoistasi julkisia';

  @override
  String get walletMoveDeshieldBody =>
      'Siirto julkiseen osoitteeseen ottaa nämä varat pois suojatusta saldostasi — summa ja julkinen osoitteesi tulevat julkisesti näkyviin Zcash-lohkoketjussa.';

  @override
  String get walletMoveWalletEnded =>
      'Lompakkoistunto päättyi. Sulje ja avaa uudelleen yrittääksesi uudestaan.';

  @override
  String get walletMoveLoading => 'Valmistellaan…';

  @override
  String get walletMovePreparing => 'Tarkistetaan summaa…';

  @override
  String get walletMoveSubmitting => 'Siirretään…';

  @override
  String get walletMoveReviewButton => 'Tarkista';

  @override
  String get walletMoveCancel => 'Peruuta';

  @override
  String get walletMoveReviewTitle => 'Tarkista siirto';

  @override
  String get walletMoveOwnAddressNote =>
      'Siirrät varoja omalle julkiselle osoitteellesi. Voit suojata nämä varat uudelleen myöhemmin, mutta tämä siirto jää pysyvästi julkiseen tietoon.';

  @override
  String get walletMoveConfirmButton => 'Siirrä julkiseksi';

  @override
  String get walletMoveBackButton => 'Takaisin';

  @override
  String get walletMoveDoneTitle => 'Siirretty julkiseksi';

  @override
  String get walletMoveDoneBody =>
      'Varasi siirtyvät julkiselle osoitteellesi. Vahvistus lohkoketjussa tapahtuu pian.';

  @override
  String get walletMoveSavedTitle => 'Tallennettu — viimeistelemme siirron';

  @override
  String get walletMoveSavedBody =>
      'Tämä siirto on tallennettu, ja lompakkosi lähettää sen myöhemmässä synkronoinnissa. Mitään ei menetetty.';

  @override
  String get walletMoveAlreadyTitle => 'Jo lähetetty';

  @override
  String get walletMoveAlreadyBody =>
      'Nämä varat on jo lähetetty ja ne ovat matkalla julkiselle osoitteellesi.';

  @override
  String get walletMoveFailedTitle => 'Siirtoa ei voitu viedä loppuun';

  @override
  String get walletMoveNothingTitle => 'Ei vielä siirrettävää';

  @override
  String get walletMoveNothingBody =>
      'Sinulla ei ole tällä hetkellä suojattua saldoa siirrettäväksi. Kun varat vahvistuvat, voit siirtää ne julkiselle osoitteellesi.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Lompakkosi kurottaa yhä kiinni — kaikki vastaanottamasi tulee siirrettäväksi, kun synkronointi on valmis.';

  @override
  String get walletMoveCouldNotLoad =>
      'Julkisen osoitteesi lataaminen epäonnistui. Yritä uudelleen.';

  @override
  String get walletMoveRetry => 'Yritä uudelleen';

  @override
  String get walletMoveClose => 'Sulje';

  @override
  String get walletSnapshotUnavailable =>
      'Lompakon lukeminen ei onnistunut juuri nyt. Se päivittyy itsestään.';

  @override
  String get walletBalanceStale =>
      'Päivitys ei onnistunut — näytetään viimeisin tunnettu saldo.';

  @override
  String get walletSyncStartFailed =>
      'Synkronoinnin käynnistys ei onnistunut. Yritämme edelleen.';

  @override
  String get walletSyncRetry => 'Yritä uudelleen';

  @override
  String get walletSyncTryNow => 'Yritä nyt';

  @override
  String get walletSyncIdle => 'Ei vielä synkronoinnissa';

  @override
  String get walletSyncIdleDetail => 'Synkronointi käynnistyy automaattisesti.';

  @override
  String get walletSyncDisabled => 'Synkronointi pois päältä';

  @override
  String get walletSyncDisabledDetail =>
      'Ota synkronointi käyttöön tämän sovelluksen asetuksista päivittääksesi saldosi.';

  @override
  String get walletSyncExplainDisabled =>
      'Synkronointi on pois päältä tämän sovelluksen asetuksista. Varasi ovat turvassa. Saldosi ja tapahtumasi näyttävät viimeisimmän synkronoidun tilan, eivätkä ne päivity ennen kuin synkronointi otetaan käyttöön.';

  @override
  String get walletParkedSyncPausedNote =>
      'Lompakkosi ei synkronoi, joten nämä maksut eivät lähde itsestään. Käytä Lähetä nyt -painiketta lähettääksesi yhden niistä itse.';

  @override
  String get walletSyncPausedMoneyNote =>
      'Keskeytetty, kunnes lompakkosi synkronoi jälleen.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Yhdistetään…';

  @override
  String get walletSyncStartingDetail =>
      'Otetaan yhteyttä Zcash-verkkoon ja valmistaudutaan skannaukseen.';

  @override
  String get walletSyncConnecting => 'Yhdistetään…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Yhdistetään… $percent %';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Skannataan $percent %';
  }

  @override
  String get walletSyncScanningEarly => 'Skannataan…';

  @override
  String get walletSyncSpendableReady => 'Varat ovat käytettävissä.';

  @override
  String get walletSyncCatchingUp =>
      'Kurotaan kiinni verkkoa — perusteellinen alkusynkronointi voi kestää tovin. Voit käyttää sovellusta sillä aikaa';

  @override
  String walletSyncScanRemaining(String count) {
    return '$count lohkoa jäljellä';
  }

  @override
  String get walletSyncUpToDate => 'Ajan tasalla';

  @override
  String get walletSyncOffline => 'Offline';

  @override
  String get walletSyncOfflineDetail =>
      'Jonossa olevat lähetykset pysyvät tallennettuina kohdassa Tallennettu ja vireillä.';

  @override
  String get walletSyncUnknown => 'Synkronoidaan…';

  @override
  String get walletSyncStalled => 'Synkronointi keskeytetty';

  @override
  String get walletStallEndpoint =>
      'Zcash-verkkoa ei tavoiteta juuri nyt. Yritämme edelleen automaattisesti — tarkista internetyhteytesi, tai palvelin saattaa olla tilapäisesti pois käytöstä.';

  @override
  String get walletStallTor =>
      'Sovelluksesi yksityinen reitti ei ole käytettävissä, joten lompakko ei yhdistä. Tarkista sovelluksesi verkkoasetukset tai poista yksityinen reitti käytöstä. Synkronointi jatkuu heti kun reitti palaa.';

  @override
  String get walletStallStorage =>
      'Laitteen tallennustila on täynnä. Vapauta tilaa, niin synkronointi jatkuu.';

  @override
  String get walletStallReorg =>
      'Ketju järjestyi uudelleen; viimeisimpiä lohkoja tarkistetaan uudelleen.';

  @override
  String get walletStallInternal =>
      'Paikallinen ongelma pysäytti synkronoinnin. Jos tämä toistuu, palauta lompakko palautuslauseesta.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Tämä palvelin lähetti tietoja, jotka eivät voi pitää paikkaansa, joten synkronointi pysähtyi. Kyse ei ole yhteysongelmasta – vaihda toiseen palvelimeen. Jos jokainen palvelin hylätään, skannaa historia uudelleen: lompakko saattaa säilyttää virheellisen tietueen aiemmalta palvelimelta.';

  @override
  String get walletStallBirthdayInFuture =>
      'Tämä lompakko on asetettu alkamaan lohkosta, jota tämä palvelin ei ole vielä saavuttanut. Tarkista lompakolle asetettu aloituslohko tai kokeile toista palvelinta.';

  @override
  String get walletStallStorageUnavailable =>
      'Synkronointi keskeytetty tällä laitteella. Yritetään uudelleen.';

  @override
  String get walletStallUnknown =>
      'Synkronointi pysähtyi tuntemattomasta syystä.';

  @override
  String get walletSyncBadgeHint => 'Näytä synkronoinnin tiedot';

  @override
  String get walletSyncSheetClose => 'Sulje';

  @override
  String get walletSyncSheetProgress => 'Eteneminen';

  @override
  String get walletSyncSheetBlocksLeft => 'Lohkoja jäljellä';

  @override
  String get walletSyncSheetSyncedTo => 'Synkronoitu lohkoon';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Vähintään $blocks lohkoa jäljessä',
      one: 'Vähintään 1 lohkon jäljessä',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'Synkronointi ei ole vielä alkanut — se käynnistyy automaattisesti. Sinun ei tarvitse tehdä mitään.';

  @override
  String get walletSyncExplainStartFailed =>
      'Synkronointi ei käynnistynyt. Varasi ovat turvassa — lompakko ei vain tarkista uutta toimintaa juuri nyt. Yritä uudelleen alla, tai avaa sovellus uudelleen.';

  @override
  String get walletSyncExplainStarting =>
      'Lompakko ottaa yhteyttä Zcash-verkkoon ja valmistautuu skannaukseen. Tämä kestää yleensä muutaman sekunnin.';

  @override
  String get walletSyncExplainConnecting =>
      'Muodostetaan yhteyttä Zcash-verkkoon.';

  @override
  String get walletSyncExplainScanning =>
      'Lompakko tarkistaa lohkoketjun lohkoja varojesi varalta. Saldosi ja tapahtumasi päivittyvät sitä mukaa kuin uusia tapahtumia löytyy — voit käyttää sovellusta sillä aikaa.';

  @override
  String get walletSyncExplainUpToDate =>
      'Täysin synkronoitu Zcash-verkon kanssa. Saldosi ja tapahtumasi ovat ajan tasalla.';

  @override
  String get walletSyncExplainStalled =>
      'Synkronoinnissa ilmeni ongelma ja se on keskeytetty. Sitä yritetään automaattisesti uudelleen.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Zcash-verkkoa ei tavoiteta — se on normaalia, jos olet offline-tilassa, tai palvelin saattaa olla tilapäisesti pois käytöstä. Varasi ovat turvassa: saldo näyttää viimeisimmän synkronoidun tilan, ja jonossa olevat lähetykset pysyvät tallennettuina kohdassa Tallennettu ja vireillä. Yhteys yrittää uudelleen itsestään.';

  @override
  String get walletSyncExplainOffline =>
      'Ei verkkoyhteyttä. Varasi ovat turvassa — saldo näyttää viimeisimmän synkronoidun tilan, ja jonossa olevat lähetykset pysyvät tallennettuina kohdassa Tallennettu ja vireillä.';

  @override
  String get walletSyncExplainUnknown =>
      'Lompakko synkronoi. Saldosi ja tapahtumasi päivittyvät sitä mukaa kuin synkronointi etenee.';

  @override
  String get walletTorOff => 'Tor pois päältä';

  @override
  String get walletTorBootstrapping => 'Yksityinen reitti käynnistyy…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport käynnistyy…';
  }

  @override
  String get walletTorActive => 'Tor käytössä';

  @override
  String get walletTorActiveUnverified =>
      'Tor käytössä (vahvistamaton ajonaikainen ympäristö)';

  @override
  String get walletTorActiveUnattested =>
      'Yksityinen reitti käytössä (yksityisyyttä ei vahvistettu)';

  @override
  String get walletTorFellBack =>
      'Tor ei käytettävissä — käytetään suoraa yhteyttä';

  @override
  String get walletTorUnavailable =>
      'Yksityinen reitti ei käytettävissä — ei yhteyttä';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport ei käytettävissä — ei yhteyttä';
  }

  @override
  String get walletTorUnanswered =>
      'Yksityinen reitti yhdistetty — mitään ei tule takaisin';

  @override
  String get walletTorUnansweredUnattested =>
      'Yksityinen reitti yhdistetty — mitään ei tule takaisin (yksityisyyttä ei vahvistettu)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport yhdistetty — mitään ei tule takaisin';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Ei yksityinen (sovelluksesi suora yhteys) — mitään ei tule takaisin';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Yhdistetty reitillä $transport — mitään ei tule takaisin; välityspalvelin voi yhdistää yhteydet toisiinsa';
  }

  @override
  String get walletTorUnknown =>
      'Torin tila tuntematon — käsittele suojaamattomana';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Saldo (lohkon $height mukaan)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Saldo · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Saldo (lohkon $height ja ajan $time mukaan)';
  }

  @override
  String get walletSyncSheetConnection => 'Yhteys';

  @override
  String get walletSyncSheetServer => 'Palvelin';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Palvelin, $host, avaa palvelimen valinnan';
  }

  @override
  String get walletSyncServerSheetTitle => 'Synkronointipalvelin';

  @override
  String get walletSyncServerInUse => 'Käytössä';

  @override
  String get walletSyncServerAppDefault => 'Sovelluksen oletus';

  @override
  String get walletSyncServerCustom => 'Oma palvelin…';

  @override
  String get walletSyncServerCustomHint => 'https://isäntä:portti';

  @override
  String get walletSyncServerCheck => 'Tarkista palvelin';

  @override
  String get walletSyncServerChecking => 'Tarkistetaan…';

  @override
  String get walletSyncServerUse => 'Käytä tätä palvelinta';

  @override
  String get walletSyncServerSwitching => 'Vaihdetaan…';

  @override
  String get walletSyncServerContinue => 'Jatka';

  @override
  String get walletSyncServerCancel => 'Peruuta';

  @override
  String get walletSyncServerTrustTitle => 'Luotatko tähän palvelimeen?';

  @override
  String get walletSyncServerTrustNotice =>
      'Luotat siihen, että tämä palvelin ilmoittaa saldosi ja historiasi ja välittää maksusi. Se näkee IP-osoitteesi, ellei Tor ole käytössä, suunnilleen milloin lompakkosi luotiin, lompakkosi tarkistamat julkiset osoitteet, sen hakemat tapahtumat sekä lähettämäsi tapahtumat.';

  @override
  String get walletSyncServerKeyLabel => 'Käyttöavain (valinnainen)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Avaimen otsake';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Anna otsake, jota palvelimesi odottaa';

  @override
  String get walletSyncServerKeyInvalid =>
      'Tätä avainta tai otsaketta ei voi käyttää';

  @override
  String get walletSyncServerKeySaved => 'Avain tallennettu';

  @override
  String get walletSyncServerKeyShow => 'Näytä';

  @override
  String get walletSyncServerKeyHide => 'Piilota';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Avaimesi tunnistaa sinut tälle palvelimelle. Se voi yhdistää maksusi lompakkoosi, myös Torin kautta.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Vaihto käynnistää meneillään olevan synkronoinnin uudelleen. Saldo ja historia säilyvät. Varat voivat näkyä saapuvina, kunnes uuden palvelimen skannaus on ajan tasalla.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Vaihto yhdistää uudelleen uuteen palvelimeen. Saldo ja historia säilyvät.';

  @override
  String get walletSyncServerUnreachable =>
      'Palvelimeen ei saatu yhteyttä. Tarkista osoite — ja jos se on oikein, joko tämä palvelin ei vastaa tai sovelluksesi ei juuri nyt tavoita sitä. Yritä uudelleen tai valitse toinen palvelin.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Palvelimeen ei saatu yhteyttä. Lompakko ei pysty erottamaan, jättääkö tämä palvelin vastaamatta vai eikö sovelluksesi tavoita sitä juuri nyt. Valitse toinen palvelin tai yritä myöhemmin uudelleen.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Tämä palvelin on eri Zcash-verkossa.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Tämä ei näytä palvelimen osoitteelta. Käytä muotoa https://isäntä:portti.';

  @override
  String get walletSyncServerNotOffered =>
      'Tämä sovellus ei tarjoa tätä palvelinta.';

  @override
  String get walletSyncServerBusy =>
      'Lompakko on juuri nyt varattu. Yritä hetken kuluttua uudelleen.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'Valitsemaasi palvelinta ei enää tarjota tässä sovelluksessa. Käytetään palvelinta $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'Tallennettua palvelinvalintaa ei voitu lukea. Käytetään palvelinta $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Vaihto ei onnistunut – käytössä on edelleen $host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Lompakon liikenne yhdistää suoraan palvelimeen. Palvelin näkee IP-osoitteesi.';

  @override
  String get walletTransportExplainTor =>
      'Lompakon liikenne reititetään Tor-verkon kautta, mikä piilottaa IP-osoitteesi palvelimelta.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Sovelluksesi yksityinen reitti käynnistyy. Lompakon liikenne odottaa sitä ennen yhdistämistä.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport käynnistyy. Lompakon liikenne odottaa sitä ennen yhdistämistä.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Toria ei tavoitettu, joten liikenne siirtyi suoraan yhteyteen. Palvelin näkee IP-osoitteesi.';

  @override
  String get walletTransportExplainUnavailable =>
      'Sovelluksesi yksityinen reitti ei ole käytettävissä, joten lompakko ei yhdistä. Poista yksityinen reitti käytöstä tai tarkista sovelluksesi verkkoasetukset.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport ei ole käytettävissä, joten lompakko ei yhdistä. Poista se käytöstä tai tarkista sovelluksesi verkkoasetukset.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'Yksityinen reitti otti yhteyden vastaan, mutta minuuttiin ei ole tullut mitään takaisin. Syynä voi olla reitti tai lompakkopalvelin — lompakko ei pysty erottamaan näitä. Se jatkaa yrittämistä; jos tilanne ei korjaannu, kokeile toista palvelinta tai tarkista sovelluksesi verkkoasetukset.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport otti yhteyden vastaan, mutta minuuttiin ei ole tullut mitään takaisin. Syynä voi olla reitti tai lompakkopalvelin — lompakko ei pysty erottamaan näitä. Se jatkaa yrittämistä; jos tilanne ei korjaannu, kokeile toista palvelinta tai tarkista sovelluksesi verkkoasetukset.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Lompakon liikenne yhdistää suoraan palvelimeen. Palvelin näkee IP-osoitteesi. Yhteys otettiin vastaan, mutta minuuttiin ei ole tullut mitään takaisin. Syynä voi olla reitti tai lompakkopalvelin — lompakko ei pysty erottamaan näitä. Se jatkaa yrittämistä; jos tilanne ei korjaannu, kokeile toista palvelinta tai tarkista sovelluksesi verkkoasetukset.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'Tämän yhteyden yksityisyyttä ei voida vahvistaa — käsittele sitä ei-yksityisenä. Yhteys otettiin vastaan, mutta minuuttiin ei ole tullut mitään takaisin. Syynä voi olla reitti tai lompakkopalvelin — lompakko ei pysty erottamaan näitä. Se jatkaa yrittämistä; jos tilanne ei korjaannu, kokeile toista palvelinta tai tarkista sovelluksesi verkkoasetukset.';

  @override
  String get walletTransportExplainUnverified =>
      'Tämän yhteyden yksityisyyttä ei voida vahvistaa — käsittele sitä ei-yksityisenä.';

  @override
  String get walletTransportExplainHostProxy =>
      'Lompakon liikenne reititetään tämän sovelluksen yksityisyyssuojatun yhteyden kautta, mikä piilottaa IP-osoitteesi palvelimelta.';

  @override
  String get walletOnboardingWelcomeTitle => 'Ota lompakko käyttöön';

  @override
  String get walletOnboardingWelcomeBody =>
      'Luo uusi lompakko ZEC:in vastaanottamista ja säilyttämistä varten. Luomme palautuslauseen ja opastamme sinut sen varmuuskopiointiin, ennen kuin varoja voi saapua — näin mitään ei koskaan vaarannu ilman varmuuskopiota.';

  @override
  String get walletCreateButton => 'Luo uusi lompakko';

  @override
  String get walletRestoreButton => 'Palauta palautuslauseesta';

  @override
  String get walletWatchOnlyButton => 'Katsele lompakkoa (vain katselu)';

  @override
  String get walletWatchOnlyTitle => 'Katsele lompakkoa';

  @override
  String get walletWatchOnlyBody =>
      'Liitä katseluavain katsellaksesi lompakkoa ilman sen kulutusavaimia. Näet sen saldon ja historian, mutta et voi lähettää varoja. Valitse lompakon likimääräinen aloituspäivä, jotta tiedämme, kuinka kauas taaksepäin skannataan.';

  @override
  String get walletWatchOnlyKeyLabel => 'Katseluavain';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => 'Skannaa katseluavaimen QR-koodi';

  @override
  String get walletWatchOnlyScanTitle => 'Skannaa katseluavain';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Kohdista kamera katseluavaimen QR-koodiin.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Kamera ei ole käytettävissä. Liitä avain manuaalisesti sen sijaan.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Liitä sen sijaan';

  @override
  String get walletWatchOnlyScanHint =>
      'Tai napauta skannauspainiketta lukeaksesi katseluavaimen QR-koodin.';

  @override
  String get walletWatchOnlyScanFilled => 'Katseluavain skannattu.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Lompakon aloituspäivä';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Skannataan alkaen $date — sitä ennen vastaanotetut varat eivät näy. Onko lompakkosi vanhempi? Valitse aiempi päivämäärä.';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'Valitse lompakon aloituspäivä';

  @override
  String get walletWatchOnlyBirthdayChange => 'Vaihda päivämäärä';

  @override
  String get walletWatchOnlySubmit => 'Katsele tätä lompakkoa';

  @override
  String get walletWatchOnlyBack => 'Takaisin';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Tämä ei näytä kelvolliselta katseluavaimelta. Tarkista se ja yritä uudelleen.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Tämä katseluavain on eri verkkoa varten. Sitä ei voi käyttää täällä.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'Tällä laitteella on jo lompakko. Palaa takaisin ja avaa se sen sijaan.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Tämä aloituspäivä on liian tuore. Valitse aikaisempi päivä.';

  @override
  String get walletRestoreTitle => 'Palauta lompakkosi';

  @override
  String get walletRestoreBody =>
      'Syötä palautuslauseesi palauttaaksesi lompakkosi — kirjoita tai liitä sanat järjestyksessä välilyönnein erotettuna. Vain tavalliset lauseet: jos lompakossasi käytettiin ylimääräistä salasanaa (\"25. sana\"), tämä sovellus ei voi vielä palauttaa sitä — näkyviin tulisi tyhjä lompakko, ei virheilmoitus.';

  @override
  String get walletRestorePhraseHint => 'sana yksi  sana kaksi  sana kolme  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count sanaa',
      one: '1 sana',
      zero: 'Ei sanoja vielä',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'palautuslauseissa on 12, 15, 18, 21 tai 24 sanaa';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count sanaa ei ole palautussanoja — korjaa korostetut sanat',
      one: '1 sana ei ole palautussana — korjaa korostettu sana',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'sana $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'sana $index: ei ole palautussana';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Poista sana $index';
  }

  @override
  String get walletRestoreSubmit => 'Palauta lompakko';

  @override
  String get walletRestoreBack => 'Takaisin';

  @override
  String get walletRestoreBirthdayTitle => 'Kuinka kauas skannataan';

  @override
  String get walletRestoreBirthdayNone =>
      'Skannaamme koko historiasi — hitaampaa, mutta mikään ei jää huomaamatta.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Skannataan alkaen $date — sitä ennen vastaanotetut varat eivät näy. Onko lompakkosi vanhempi? Valitse aiempi päivämäärä tai skannaa koko historia.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Valitse päivämäärä';

  @override
  String get walletRestoreBirthdayChange => 'Vaihda päivämäärää';

  @override
  String get walletRestoreBirthdayClear => 'Skannaa koko historia';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'Sana $index ei ole palautussana. Tarkista lauseesi kirjoitusvirheiden varalta ja yritä uudelleen.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Tämä palautuslause ei ole kelvollinen. Tarkista sanat ja niiden järjestys ja yritä uudelleen.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Tämä lause ei vastaa tämän laitteen lompakkoa. Tarkista se huolellisesti ja yritä uudelleen.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'Tällä laitteella on jo lompakko. Palaa takaisin avataksesi sen.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Tämä päivämäärä on liian tuore. Valitse aiempi päivämäärä tai skannaa kaikki.';

  @override
  String get walletGeneratingLabel => 'Luodaan lompakkoasi…';

  @override
  String get walletOpeningLabel => 'Avataan lompakkoasi…';

  @override
  String get walletBackupTitle => 'Varmuuskopioi palautuslauseesi';

  @override
  String get walletBackupBody =>
      'Nämä sanat ovat AINOA tapa palauttaa lompakkosi ja varasi. Kirjoita ne muistiin järjestyksessä ja säilytä niitä turvallisessa ja yksityisessä paikassa. Älä koskaan jaa niitä tai tallenna verkkoon — kuka tahansa, jolla on nämä sanat, voi viedä varasi.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Kuvakaappaukset on estetty tällä näytöllä.';

  @override
  String get walletBackupSecureNoteOther =>
      'Varmista, ettei kukaan muu näe näyttöäsi.';

  @override
  String get walletBackupReveal => 'Näytä palautuslause';

  @override
  String get walletBackupRevealing => 'Valmistellaan palautuslausettasi…';

  @override
  String get walletBackupRevealFailed =>
      'Palautuslauseen näyttäminen ei onnistunut juuri nyt. Varmista, että laitteesi lukitus on avattu, ja yritä uudelleen.';

  @override
  String get walletBackupRetryReveal => 'Yritä uudelleen';

  @override
  String get walletBackupReauthFailed =>
      'Henkilöllisyytesi vahvistaminen ei onnistunut. Yritä uudelleen.';

  @override
  String get walletBackupConfirmCheckbox =>
      'Olen kirjoittanut palautuslauseeni muistiin ja säilyttänyt sen turvallisesti.';

  @override
  String get walletBackupContinue => 'Jatka';

  @override
  String get walletBackupSaveFailed =>
      'Vahvistuksesi tallentaminen ei onnistunut. Yritä uudelleen.';

  @override
  String get walletBackupStartOver => 'Aloita alusta';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Aloitetaanko alusta ilman tätä lompakkoa?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Tämä poistaa tämän lompakon laitteelta ja palauttaa sinut alkuun. Mitään ei voi tallettaa tämän sovelluksen kautta ennen kuin käyttöönotto on valmis.\n\nJos tässä lompakossa on joskus ollut varoja — tai se palautettiin palautuslauseesta — vain kyseinen lause voi palauttaa sen.';

  @override
  String get walletBackupStartOverConfirm => 'Poista ja aloita alusta';

  @override
  String get walletBackupStartOverKeep => 'Säilytä tämä lompakko';

  @override
  String get walletBackupSectionTitle => 'Palautuslause';

  @override
  String get walletBackupTileTitle => 'Varmuuskopioi palautuslauseesi';

  @override
  String get walletBackupTileSubtitle =>
      'Näytä sanat, jotka voivat palauttaa lompakkosi ja varasi.';

  @override
  String get walletBackupScreenTitle => 'Palautuslause';

  @override
  String get walletBackupDone => 'Valmis';

  @override
  String get walletBackupManagedTitle => 'Ei erillistä palautuslausetta';

  @override
  String get walletBackupManagedBody =>
      'Tämä lompakko otettiin käyttöön sen sovelluksen tilillä, joka asensi sen, joten sillä ei ole omaa palautuslausetta. Varasi palautuvat yhdessä kyseisen tilin kanssa — käytä sen varmuuskopiota pitääksesi ne turvassa.';

  @override
  String get walletExportViewingKeyTitle => 'Vie katseluavain';

  @override
  String get walletExportViewingKeyTileTitle => 'Vie katseluavain';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Jaa lompakostasi katselukopio — se näkee historiasi, mutta ei voi käyttää varoja.';

  @override
  String get walletExportViewingKeyWarning =>
      'Tämä avain antaa kenelle tahansa, jolla se on, mahdollisuuden nähdä kaiken, mitä tämä lompakko on koskaan vastaanottanut ja lähettänyt — sekä kaiken, mitä se vastaanottaa ja lähettää tulevaisuudessa. Se ei voi käyttää varojasi eikä palauttaa lompakkoasi. Jaa se vain jollekulle, johon luotat näkemään koko historiasi, esimerkiksi kirjanpitäjällesi tai omalle toiselle laitteellesi. Ainoa tapa perua jakaminen myöhemmin on siirtää varasi uuteen lompakkoon.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Tämä avain antaa kenelle tahansa, jolla se on, mahdollisuuden nähdä kaiken, mitä tämä lompakko on koskaan vastaanottanut ja lähettänyt — sekä kaiken, mitä se vastaanottaa ja lähettää tulevaisuudessa. Se ei voi käyttää varojasi eikä palauttaa lompakkoasi. Jaa se vain jollekulle, johon luotat näkemään koko historiasi, esimerkiksi kirjanpitäjällesi tai omalle toiselle laitteellesi. Kun se on jaettu, jakamista ei voi enää perua.';

  @override
  String get walletExportViewingKeyReveal => 'Näytä katseluavain';

  @override
  String get walletExportViewingKeyRetry => 'Yritä uudelleen';

  @override
  String get walletExportViewingKeyRevealing =>
      'Valmistellaan katseluavaintasi…';

  @override
  String get walletExportViewingKeyFailed =>
      'Katseluavaimesi näyttäminen ei onnistunut juuri nyt. Yritä uudelleen hetken kuluttua.';

  @override
  String get walletExportViewingKeyQrLabel => 'Katseluavaimen QR-koodi';

  @override
  String get walletExportViewingKeyCopy => 'Kopioi katseluavain';

  @override
  String get walletExportViewingKeyCopied => 'Katseluavain kopioitu';

  @override
  String get walletExportViewingKeyDone => 'Valmis';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Kuvakaappaukset on estetty tällä näytöllä.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Varmista, ettei kukaan muu näe näyttöäsi.';

  @override
  String get walletWatchOnlySectionTitle => 'Tietoa tästä katselulompakosta';

  @override
  String get walletWatchOnlyAboutBody =>
      'Tämä on katselulompakko. Se on otettu käyttöön katseluavaimella, joten se näkee saldosi ja historiasi, mutta sillä ei ole kulutusavaimia — täällä ei ole mitään varmuuskopioitavaa, eikä se voi lähettää varoja.';

  @override
  String get walletWatchOnlyBadge => 'Vain katselu';

  @override
  String get walletOnboardingFailedTitle =>
      'Lompakon käyttöönotto ei onnistunut';

  @override
  String get walletOnboardingRetry => 'Yritä uudelleen';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'Puhelimesi suojattu tallennustila ei vastaa. Poista laitteesi lukitus ja yritä uudelleen. Jos tämä toistuu, käynnistä puhelin uudelleen.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Tämä lompakko on avoinna toisessa ikkunassa tai sovelluksessa, tai se viimeistelee vielä edellistä toimintoa. Sulje muut sitä käyttävät ikkunat — tai odota hetki — ja yritä sitten uudelleen.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'Tämän lompakon suojattu avain ei ole enää käytettävissä, joten sitä ei voida avata tällä laitteella. Varasi ovat turvassa — palauta lompakko palautuslauseestasi saadaksesi ne takaisin.';

  @override
  String get walletOnboardingFailedRestoreAction => 'Palauta palautuslauseesta';

  @override
  String get walletOnboardingRecoverConfirmTitle =>
      'Palautetaanko tämä lompakko?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Varmista, että sinulla on palautuslauseesi käsillä ennen jatkamista — tarvitset sitä seuraavalla näytöllä varojesi palauttamiseen. Varasi ovat turvassa lohkoketjussa ja kyseisen lauseen hallinnassa. Tämä poistaa lukukelvottoman lompakkodatan tältä laitteelta, jotta se voidaan rakentaa uudelleen.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Peruuta';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Vapaata tilaa ei ole tarpeeksi lompakon käyttöönottoon. Vapauta tilaa ja yritä uudelleen.';

  @override
  String get walletOnboardingFailedNoVault =>
      'Tässä laitteessa ei ole suojattua avainsäilöä, joten lompakko ei voi suojata palautuslausettasi tällä laitteella.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Verkkoa ei tavoitettu käyttöönoton aikana. Tarkista yhteytesi ja yritä uudelleen.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'Lompakon käyttöönotto ei valmistunut. Yritä uudelleen saattaaksesi sen loppuun — mitään ei menetetty.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Lompakon käyttöönotossa tapahtui virhe. Yritä uudelleen.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'Tämän sovelluksen lompakon määritykset ovat virheelliset, joten lompakko ei voi käynnistyä. Uudelleenyrittäminen ei auta — ilmoita tästä sovelluksen kehittäjälle. Varasi eivät ole vaarassa.';

  @override
  String get walletSendButton => 'Lähetä';

  @override
  String get walletSendSyncNotRunning =>
      'Synkronointi ei ole käynnissä — käytettävissä oleva saldosi ei voi päivittyä';

  @override
  String get walletSendWaitingForFunds =>
      'Synkronointi on vielä kesken — voit lähettää, kun sinulla on käytettävissä olevaa saldoa';

  @override
  String get walletSendNoSpendableYet => 'Ei vielä käytettävissä olevaa saldoa';

  @override
  String get walletSendSyncUnavailable =>
      'Voit lähettää, kun synkronointi jatkuu';

  @override
  String get walletSendTitle => 'Lähetä';

  @override
  String get walletSendUnavailable =>
      'Lompakkosi ei ole valmis juuri nyt. Palaa takaisin ja yritä uudelleen.';

  @override
  String get walletSendWatchOnly =>
      'Tämä lompakko on vain katselua varten. Se voi näyttää saldon ja vastaanottaa maksuja, mutta sillä ei ole kulutusavaimia — joten se ei voi lähettää.';

  @override
  String get walletSendExpiredTitle => 'Tämä maksupyyntö vanheni';

  @override
  String get walletSendExpiredBody =>
      'Lähetysnäkymän avautuminen kesti yli viisi sekuntia, joten sovellukselle ilmoitettiin, ettei mitään lähetetty. Vastaus on lopullinen: tätä pyyntöä ei voi maksaa tästä. Maksaaksesi aloita uudelleen sovelluksesta.';

  @override
  String get walletSendFaultWatchOnly =>
      'Tämä lompakko on vain katselua varten — sillä ei ole kulutusavaimia, joten se ei voi lähettää.';

  @override
  String walletSendAvailable(String amount) {
    return 'Lähetettävissä: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Lähetettävissä: $amount ZEC — saldosi kurottaa yhä kiinni';
  }

  @override
  String get walletSendRecipientLabel => 'Vastaanottajan osoite';

  @override
  String get walletSendRecipientHint =>
      'Zcash-osoite (alkaa kirjaimella u, z tai t)';

  @override
  String get walletSendRecipientLocked =>
      'Vastaanottajaa ei voi muuttaa täällä';

  @override
  String get walletSendAmountLabel => 'Summa (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Muistio (valinnainen)';

  @override
  String get walletSendMemoHint =>
      'Toimitetaan vain suojatuille (yksityisille) vastaanottajille';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Muistiot vaativat suojatun vastaanottajan. Tämä julkinen osoite ei voi vastaanottaa niitä.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Tässä maksussa on jo sovelluksen viite, joten siihen ei voi lisätä myös kirjoitettua viestiä.';

  @override
  String get walletSendMachineMemoTitle => 'Sovellus liittää viitteen';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Sen mukaan tämä on tarkoitukseen: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Se jää maksutapahtumaan eikä sitä voi poistaa jälkikäteen. Lompakko ei voi tarkistaa sen sisältöä.';

  @override
  String get walletSendRecipientShielded => 'Suojattu · yksityinen';

  @override
  String get walletSendRecipientTransparent => 'Julkinen';

  @override
  String get walletSendRecipientInvalid =>
      'Tämä ei näytä kelvolliselta Zcash-osoitteelta.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Tämä osoite on eri Zcash-verkkoa varten.';

  @override
  String get walletSendReviewButton => 'Tarkista maksu';

  @override
  String get walletSendQueueButton => 'Aseta lähetettäväksi myöhemmin';

  @override
  String get walletSendQueueHint =>
      'Jonossa oleva maksu odottaa kohdassa Tallennettu ja vireillä, jossa voit lähettää sen tai peruuttaa sen. Verkkomaksu lasketaan lähetyshetkellä.';

  @override
  String get walletSendPreparing => 'Valmistellaan maksuasi…';

  @override
  String get walletSendSubmitting => 'Lähetetään…';

  @override
  String get walletSendQueuing => 'Asetetaan jonoon…';

  @override
  String get walletSendReviewTitle => 'Vahvista maksu';

  @override
  String get walletSendTotalLabel => 'Yhteensä';

  @override
  String get walletSendFeeLabel => 'Verkkomaksu';

  @override
  String get walletSendChangeLabel => 'Palautettu vaihtoraha';

  @override
  String get walletSendDeshieldTitle => 'Tämä maksu ei ole yksityinen';

  @override
  String get walletSendDeshieldBody =>
      'Se lähetetään julkiseen osoitteeseen, joten summa ja vastaanottaja näkyvät julkisesti Zcash-lohkoketjussa.';

  @override
  String get walletSendPublicAckLabel =>
      'Ymmärrän, että tämä maksu on julkinen.';

  @override
  String get walletSendConfirmButton => 'Lähetä nyt';

  @override
  String get walletSendBackButton => 'Takaisin';

  @override
  String get walletSendSelfSendNote =>
      'Lähetät omalle lompakollesi. Verkkomaksu peritään silti.';

  @override
  String get walletSendLargeConfirmTitle => 'Lähetetäänkö suuri summa?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Tämä on lähes koko saldosi. Lähetettyä maksua ei voi peruuttaa.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Tämä on suuri maksu. Lähetettyä maksua ei voi peruuttaa.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Tämä on suuri maksu — lähes koko saldosi. Lähetettyä maksua ei voi peruuttaa.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Lähetä $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Palaa takaisin';

  @override
  String get walletSendSentTitle => 'Maksu lähetetty';

  @override
  String get walletSendSentBody => 'Maksusi on lähetetty verkkoon.';

  @override
  String get walletSendSavedTitle => 'Tallennettu — viimeistelemme lähetyksen';

  @override
  String get walletSendSavedBody =>
      'Maksusi ei lähtenyt juuri nyt, joten se on tallennettu — lompakkosi lähettää sen myöhemmässä synkronoinnissa. Mitään ei ole menetetty.';

  @override
  String get walletSendKeptTitle => 'Tallennettu';

  @override
  String get walletSendKeptBody =>
      'Lompakkosi on säilyttänyt tämän tapahtuman, mutta ei ole luvannut lähettää sitä itsestään. Katso sen tila Tapahtumat-näkymästä.';

  @override
  String get walletSendPartialBody =>
      'Osa maksustasi lähti; lompakkosi viimeistelee lopun myöhemmässä synkronoinnissa. Mitään ei ole menetetty.';

  @override
  String get walletSendInMotionTitle => 'Maksu käynnissä';

  @override
  String get walletSendInMotionBody =>
      'Maksusi on käynnistynyt ja liikkuu lompakkosi hallitseman kertakäyttöisen osoitteen kautta. Älä lähetä sitä uudelleen. Jos se ei valmistu, voit palauttaa varat lompakkonäytöltä.';

  @override
  String get walletSendAlreadyTitle => 'Jo lähetetty';

  @override
  String get walletSendAlreadyBody =>
      'Tämä maksu on jo lähetetty — sitä ei lähetetä kahdesti.';

  @override
  String get walletSendFailedTitle => 'Maksua ei voitu viedä loppuun';

  @override
  String get walletSendFailedBody =>
      'Maksun viimeistelyssä tapahtui virhe, eikä mitään lähetetty. Voit yrittää uudelleen.';

  @override
  String get walletSendTryAgain => 'Yritä uudelleen';

  @override
  String get walletSendDone => 'Valmis';

  @override
  String get walletSendAnother => 'Lähetä uusi maksu';

  @override
  String get walletSendQueuedTitle => 'Asetettu lähetysjonoon';

  @override
  String get walletSendQueuedBody =>
      'Tämä maksu on tallennettu. Löydät sen kohdasta Tallennettu ja vireillä, jossa voit lähettää sen nyt tai peruuttaa sen.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Käytettävissä oleva saldo ei riitä — sinulla on $available ZEC ja tämä vaatii $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Zcash-verkko päivitettiin, ja tämä sovellus tarvitsee päivityksen ennen kuin se voi lähettää. Varasi ovat turvassa.';

  @override
  String get walletSyncUpToDateLimited =>
      'Ajan tasalla niin pitkälle kuin tämä versio osaa lukea';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Zcash-verkko päivitettiin. Tämä versio on skannannut kaiken, minkä se osaa lukea, mutta uudemmissa lohkoissa voi olla varoja, joita se ei vielä voi näyttää, eivätkä viimeaikaisten maksujen viestit ole saatavilla. Päivitä sovellus nähdäksesi kaiken.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Ajan tasalla, mutta tämä palvelin ei palvele kaikkia pooleja';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Tämä palvelin kieltäytyy palvelemasta, pidättää tai raportoi väärin yhden Zcashin suojatuista pooleista. Siihen pooliin vastaanotettuja varoja ei voi käyttää tämän palvelimen kautta, ja näytetty saldo on alaraja. Vaihda toiseen palvelimeen käyttääksesi niitä – kyse ei ole yhteysongelmasta.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: tämä palvelin kieltäytyy palvelemasta sitä';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: tämä palvelin pidättää osan siitä';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: tämä palvelin ilmoittaa sen väärin';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: ei tiedetä, palveleeko tämä palvelin sitä';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Ajan tasalla tämän palvelimen kanssa, mutta palvelin on verkkoa jäljessä';

  @override
  String get walletSyncExplainEndpointBehind =>
      'Tämän palvelimen ketju päättyy lohkoon, jonka verkko oli ohittanut jo ennen tämän sovellusversion kääntämistä, joten saldosi on ajan tasalla vain siihen lohkoon asti. Uudet sinulle tulleet maksut eivät ehkä vielä näy, eikä täältä lähetetty maksu välttämättä mene perille. Vaihda toiseen palvelimeen päästäksesi ajan tasalle – kyse ei ole yhteysongelmasta.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Odotetaan sovelluspäivitystä — varasi ovat turvassa eikä mitään ole lähetetty.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Odotetaan palvelinta, joka ilmoittaa verkkoversion — vaihda palvelinta. Varasi ovat turvassa eikä mitään ole lähetetty.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Odotetaan palvelinta, joka ilmoittaa verkkoversion. Jos tämän laitteen päivämäärä ja kellonaika ovat väärin, korjaa ne ensin – ja vaihda sitten palvelinta. Varasi ovat turvassa eikä mitään ole lähetetty.';

  @override
  String get walletSyncUnverified =>
      'Ajan tasalla, mutta tämä palvelin ei ilmoita verkkoversiota';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'Lähettäminen toimii vielä noin $hours tuntia — vaihda sen jälkeen palvelinta.',
      one:
          'Lähettäminen toimii vielä noin 1 tunnin — vaihda sen jälkeen palvelinta.',
      zero:
          'Lähettäminen toimii vielä alle tunnin — vaihda sen jälkeen palvelinta.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'Lähettäminen toimii vielä noin $blocks lohkon ajan — vaihda sen jälkeen palvelinta.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Tämä palvelin ei ole ilmoittanut verkkoversiota $blocks lohkoon, joten sovellus ei voi varmistaa, että lähettäminen on turvallista. Vaihda toiseen palvelimeen.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Tämä palvelin ei ole ilmoittanut verkkoversiota päivään, joten sovellus ei voi varmistaa, että lähettäminen on turvallista. Jos tämän laitteen päivämäärä ja kellonaika ovat väärin, korjaa ne ensin – ja vaihda sitten palvelimeen, joka ilmoittaa verkkoversion.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Tämä palvelin ei ole koskaan ilmoittanut verkkoversiota, joten sovellus ei voi varmistaa, että lähettäminen on turvallista. Vaihda toiseen palvelimeen.';

  @override
  String get walletSyncExplainUnverified =>
      'Tämä palvelin ei kerro, missä Zcash-verkon versiossa se on, joten sovellus ei voi varmistaa, että sen allekirjoittama maksu hyväksytään. Saldosi on ajan tasalla. Vaihda toiseen palvelimeen – tämä ei ole yhteysongelma.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Tämä palvelin ei kerro, missä Zcash-verkon versiossa se on, joten sovellus ei voi varmistaa, että sen allekirjoittama maksu hyväksytään. Se on myös toistuvasti tarjonnut lohkoja, jotka tämä lompakko joutui sitten perumaan, joten saldosi ei ehkä ole ajan tasalla. Vaihda toiseen palvelimeen – tämä ei ole yhteysongelma.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Tämä palvelin tarjoaa myös toistuvasti lohkoja, jotka lompakko joutuu perumaan – vaihda palvelinta.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Saldosi kurottaa yhä kiinni — lisää saattaa tulla käytettäväksi lompakon synkronoituessa.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC on vielä saapumassa ja tulee käytettäväksi, kun lompakko on ajan tasalla.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Anna lähetettävä summa.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Anna summa numerona, esimerkiksi 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'ZEC:llä on enintään 8 desimaalia.';

  @override
  String get walletSendFaultAmountNotPositive => 'Anna nollaa suurempi summa.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Tämä summa on suurempi kuin ZEC:n kokonaistarjonta.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Tämä sovellus rajoittaa lähetykset tällä hetkellä enimmäismäärään $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Tämä ei näytä kelvolliselta Zcash-osoitteelta tälle verkolle. Tarkista se ja yritä uudelleen.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Tämä vastaanottaja ei voi vastaanottaa muistiota. Poista muistio tai lähetä suojattuun (yksityiseen) osoitteeseen.';

  @override
  String get walletSendFaultMemoTooLong =>
      'Muistiosi on liian pitkä. Lyhennä sitä ja yritä uudelleen.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Tätä muistiota ei voida lähettää. Poista se ja yritä uudelleen.';

  @override
  String get walletSendFaultMemoConflict =>
      'Tätä maksua ei voitu lähettää – sovellus liitti siihen kaksi viestiä. Mitään ei lähetetty.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Tämä osoite on eri verkkoa varten.';

  @override
  String get walletSendFaultUriInvalid =>
      'Tätä maksua ei voitu muodostaa. Tarkista osoite ja summa.';

  @override
  String get walletSendFaultNotSynced =>
      'Lompakkosi ei ole vielä synkronoitu tarpeeksi pitkälle. Odota synkronoinnin etenemistä tai aseta tämä lähetettäväksi myöhemmin.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Lompakkosi ei ole vielä synkronoitu tarpeeksi pitkälle. Odota synkronoinnin etenemistä.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Lompakkosi ei ole vielä synkronoitu tarpeeksi pitkälle, ja synkronointi ei ole juuri nyt käynnissä. Tarkista synkronoinnin tila lompakkonäytöltä.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Summat vanhenivat tarkistuksen aikana. Tarkista maksu uudelleen.';

  @override
  String get walletSendFaultQueueFull =>
      'Liian monta lähetystä odottaa lähtöä. Anna niiden lähteä ensin ja yritä sitten uudelleen.';

  @override
  String get walletSendFaultWalletBusy =>
      'Lompakko on varattu juuri nyt. Yritä hetken kuluttua uudelleen.';

  @override
  String get walletSendFaultStorageFull =>
      'Vapaata tilaa ei ole tarpeeksi tämän lähetyksen suorittamiseen. Vapauta tilaa ja yritä uudelleen.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Liian monta kertakäyttöistä osoitetta on käytössä juuri nyt. Osa saattaa vapautua, kun siirrot vahvistuvat, mutta tilanne ei välttämättä selviä itsestään. Varasi ovat turvassa.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Tätä maksua ei voitu valmistella. Tarkista tiedot ja yritä uudelleen.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Tätä maksua ei voitu valmistella juuri nyt. Yritä hetken kuluttua uudelleen.';

  @override
  String get walletSwapButton => 'Vaihda';

  @override
  String get walletSwapTitle => 'Vaihda ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'Lompakkosi ei ole valmis juuri nyt. Palaa takaisin ja yritä uudelleen.';

  @override
  String get walletSwapUnavailableOff =>
      'Vaihto ei ole käytettävissä juuri nyt.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Tämä lompakko on vain katselua varten — se ei voi vaihtaa.';

  @override
  String get walletSwapDone => 'Valmis';

  @override
  String get walletSwapBackToWallet => 'Takaisin lompakkoon';

  @override
  String walletSwapAvailable(String amount) {
    return 'Vaihdettavissa: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Vaihdettavissa: $amount ZEC — saldosi kurottaa yhä kiinni';
  }

  @override
  String get walletSwapAssetLabel => 'Vastaanotettava omaisuuserä';

  @override
  String get walletSwapAmountLabel => 'Vaihdettava summa (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Kohdeosoite';

  @override
  String get walletSwapDestinationHint =>
      'Vastaanotto-osoitteesi kohdeketjussa';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Vastaanotto-osoitteesi verkossa $chain';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Osoite verkossa $chain — tähän vaihdettu omaisuuserä lähetetään. Tarkista, että verkko on oikea.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Skannaa kohdeosoitteen QR-koodi';

  @override
  String get walletSwapTargetAssetHint => 'Valitse vastaanotettava omaisuuserä';

  @override
  String get walletSwapQuoteButton => 'Hae tarjous';

  @override
  String get walletSwapQuoting => 'Haetaan tarjousta…';

  @override
  String get walletSwapExecuting => 'Käynnistetään vaihtoasi…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Vielä kesken — vaihto on käynnistymässä. Tämä voi kestää jopa minuutin.';

  @override
  String get walletSwapReviewTitle => 'Vahvista vaihto';

  @override
  String get walletSwapYouSendLabel => 'Lähetät';

  @override
  String get walletSwapYouReceiveLabel => 'Saat vähintään';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Verkkomaksu';

  @override
  String get walletSwapNetworkFeeValue => 'Lisätään talletusta lähetettäessä';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Tarjous on voimassa vielä noin $time — vahvista ennen sen vanhenemista.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Tarjous on voimassa vielä alle minuutin — vahvista ennen sen vanhenemista.';

  @override
  String get walletSwapQuoteExpired =>
      'Tämä tarjous on vanhentunut. Palaa takaisin ja hae uusi — sen kurssi ei ole enää voimassa, ja lähettäminen nyt voi johtaa palautukseen.';

  @override
  String get walletCountdownUnderMinute => 'alle minuutin';

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
    return '$hours h $minutes min';
  }

  @override
  String get walletSwapDeshieldTitle => 'Tämä vaihto ei ole yksityinen';

  @override
  String get walletSwapDeshieldBody =>
      'ZEC:in vaihtaminen ulos poistaa sen suojauksen — talletus on julkinen tapahtuma, ja palveluntarjoajan puoli on julkinen sen omassa verkossa.';

  @override
  String get walletSwapDiscloseTitle => 'Mitä vaihdon palveluntarjoaja näkee';

  @override
  String get walletSwapDiscloseAmounts => 'Summat molemmilla puolilla';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Että tämä ZEC ja vastaanottamasi omaisuuserä ovat sama vaihto';

  @override
  String get walletSwapDiscloseDestination => 'Kohdeosoitteesi';

  @override
  String get walletSwapDiscloseSource => 'Lähdeosoitteesi';

  @override
  String get walletSwapDiscloseIp =>
      'IP-osoitteesi (ellet reititä Torin kautta)';

  @override
  String get walletSwapDiscloseGeneric => 'Muut tämän vaihdon tiedot';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'Palveluntarjoajan omat tapahtumat ovat julkisia sen verkossa';

  @override
  String get walletSwapAckLabel =>
      'Ymmärrän, että palveluntarjoaja näkee yllä olevat tiedot.';

  @override
  String get walletSwapConfirmButton => 'Aloita vaihto';

  @override
  String get walletSwapBackButton => 'Takaisin';

  @override
  String get walletSwapStatusPendingTitle => 'Vaihto aloitettu';

  @override
  String get walletSwapStatusCheckingTitle => 'Tarkistetaan vaihdon tilaa…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Lompakkosi lähettää ZEC-talletusta palveluntarjoajalle. Jos olet hetken offline-tilassa, se lähetetään automaattisesti, kun olet taas verkossa — mutta lähetysikkuna on lyhyt, ja jos se sulkeutuu ensin, vaihto yksinkertaisesti päättyy eikä mitään vaihdeta. ZECisi pysyy sinun omanasi, ja sen näkyminen taas käytettävissä olevana voi kestää jopa tunnin.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'Odotetaan talletuksesi saapumista. Jos et ole vielä lähettänyt varoja toisesta lompakostasi, lähetä ne ennen tarjouksen vanhenemista.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Tämä vaihto odottaa yhä talletustaan. Talletusohjeet eivät ole enää saatavilla tällä laitteella — jos olet jo lähettänyt varat, ne havaitaan; jos et ole, anna tämän vaihdon vanheta ja aloita uusi.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'Talletusikkuna päättyy $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'Talletusikkuna on päättynyt. Jos talletusta ei lähetetty ajoissa, vaihto päättyy ja ZECisi pysyy lompakossasi.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'Talletusikkuna on päättynyt. Jos et ole vielä lähettänyt talletustasi, vaihto päättyy yksinkertaisesti — hae uusi tarjous, kun olet valmis.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Vaihdot käynnissä',
      one: 'Vaihto käynnissä',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'ZECisi on matkalla palveluntarjoajalle.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'Odotetaan talletuksesi saapumista palveluntarjoajalle.';

  @override
  String get walletSwapInFlightRowGeneric => 'Vaihto on käynnissä.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'Talletusikkuna on päättynyt — tarkista tämän vaihdon tila.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Tämä vaihto ei ole vielä saavuttanut vahvistettua lopputulosta täällä — avaa se ja tarkista. Tälle lompakolle palautuva ZEC näkyy saldossasi synkronoinnin jälkeen.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Tämä vaihto ei ole vielä saavuttanut vahvistettua lopputulosta täällä — avaa se ja tarkista. Tämän vaihdon tälle lompakolle toimittama ZEC näkyy saldossasi synkronoinnin jälkeen.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Vaihto valmistui.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Vaihto palautettu.';

  @override
  String get walletSwapRowOutcomeFailed => 'Vaihto ei valmistunut.';

  @override
  String get walletSwapRemove => 'Poista';

  @override
  String get walletSwapRemoveTitle => 'Poistetaanko tämä vaihto listalta?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Tämä vain poistaa vaihdon listalta — se ei peruuta vaihtoa, ja tämä lompakko lopettaa sen palautuksen seuraamisen. Myöhemmin palautettu ZEC kuuluu silti tälle lompakolle; täydellinen uudelleenskannaus voi löytää sen.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Tämä vain poistaa vaihdon listalta — se ei peruuta vaihtoa, ja tämä lompakko lopettaa saapuvan ZECin seuraamisen. Myöhemmin saapuva ZEC kuuluu silti tälle lompakolle; täydellinen uudelleenskannaus voi löytää sen. Jos vaihto sen sijaan palautetaan, palautus tulee takaisin lähettämässäsi omaisuuserässä, tämän lompakon ulkopuolelle.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Tämä vain poistaa vaihdon listalta — se ei peruuta vaihtoa, ja tämä lompakko lopettaa siitä yhä saapuvan ZECin seuraamisen. Myöhemmin saapuva ZEC kuuluu silti tälle lompakolle; täydellinen uudelleenskannaus voi löytää sen.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Tämä poistaa valmistuneen vaihdon listalta.';

  @override
  String get walletSwapRemoveCancel => 'Peruuta';

  @override
  String get walletSwapRemoveConfirm => 'Poista';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Aloitettu $time';
  }

  @override
  String get walletSwapViewSwap => 'Näytä vaihto';

  @override
  String get walletSwapsInFlightError =>
      'Käynnissä olevien vaihtojesi lataaminen epäonnistui juuri nyt.';

  @override
  String get walletSwapsInFlightRetry => 'Yritä uudelleen';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Yritetään…';

  @override
  String get walletSwapStartAnother => 'Aloita toinen vaihto';

  @override
  String get walletSwapStatusUnderTitle => 'Odotetaan koko talletusta';

  @override
  String get walletSwapStatusUnderBody =>
      'Osa talletuksesta on saapunut. Loput valmistuu, tai palveluntarjoaja palauttaa varat.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Osa talletuksestasi on saapunut. Lähetä puuttuva summa ennen määräaikaa, tai palveluntarjoaja palauttaa saapuneen osan.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Saapunut: $received; puuttuu vielä $missing. Talletusikkuna päättyy: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Talletus vastaanotettu';

  @override
  String get walletSwapStatusDetectedBody =>
      'Palveluntarjoaja vastaanotti talletuksesi ja käsittelee vaihdon.';

  @override
  String get walletSwapStatusProcessingTitle => 'Käsitellään vaihtoasi';

  @override
  String get walletSwapStatusProcessingBody =>
      'Palveluntarjoaja viimeistelee vaihtoasi.';

  @override
  String get walletSwapStatusSuccessTitle => 'Vaihto valmis';

  @override
  String get walletSwapStatusSuccessBody => 'Vaihtosi valmistui onnistuneesti.';

  @override
  String get walletSwapStatusRefundedTitle => 'Vaihto palautettu';

  @override
  String get walletSwapStatusRefundedBody =>
      'Vaihto ei valmistunut, joten palveluntarjoaja palautti varat palautusosoitteeseesi.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'Vaihto ei valmistunut, joten palveluntarjoaja lähetti ZECisi takaisin tähän lompakkoon. Se saapuu suojaamattomina varoina ja näkyy saldossasi, kun lompakko synkronoituu seuraavan kerran — tämä voi kestää hetken.';

  @override
  String get walletSwapStatusFailedTitle => 'Vaihto epäonnistui';

  @override
  String get walletSwapStatusFailedBody =>
      'Vaihtoa ei voitu viedä loppuun. Talletetut varat selvitetään tai palautetaan palveluntarjoajan puolella.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Vaihtoa ei löytynyt';

  @override
  String get walletSwapStatusNotFoundBody =>
      'Palveluntarjoajalla ei ole enää tietoa tästä vaihdosta — se on todennäköisesti vanhentunut. Jos talletus tehtiin, palveluntarjoajan pitäisi palauttaa se palautusosoitteeseen. Vaihto pysyy listallasi, ja tämä lompakko jatkaa sen ZECin seuraamista sen varalta, että se vielä saapuu; voit poistaa sen listalta milloin tahansa.';

  @override
  String get walletSwapStatusUnknownTitle => 'Tila ei ole saatavilla';

  @override
  String get walletSwapStatusUnknownBody =>
      'Tämän vaihdon tilaa ei voida lukea juuri nyt.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'Seuranta ei käytettävissä';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'Vaihto on pois päältä, joten emme voi seurata tätä täällä. Varat selvitetään tai palautetaan palveluntarjoajan puolella.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'Vaihto on täällä pois päältä, joten tätä vaihtoa ei voida seurata juuri nyt. Jos se palautettiin, ZEC palautuu tälle lompakolle — se näkyy saldossasi, kun vaihto kytketään takaisin päälle ja lompakko synkronoituu.';

  @override
  String get walletSwapTrackingError => 'Tätä vaihtoa ei voitu seurata.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Tämän vaihdon seurantaa ei voitu avata. Vaihto itse saattaa silti olla yhä käynnissä — talletetut varat selvitetään tai palautetaan palveluntarjoajan puolella.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Anna osoite, johon haluat vastaanottaa vaihdetun omaisuuserän.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Tämä kohdeosoite ei kelpaa tälle omaisuuserälle. Tarkista se ja yritä uudelleen.';

  @override
  String get walletSwapFaultExpired =>
      'Tämä tarjous vanheni. Hae uusi tarjous jatkaaksesi.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'Palveluntarjoajan hinta liikkui rajasi ulkopuolelle, joten vaihto pysäytettiin ennen kuin mitään siirtyi. Yritä uudelleen.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'Liukumaraja on liian suuri turvalliselle vaihdolle. Yritä uudelleen.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'Vaihdon palveluntarjoaja ei ole käytettävissä juuri nyt. Yritä hetken kuluttua uudelleen.';

  @override
  String get walletSwapFaultConnection =>
      'Vaihtopalvelua ei tavoitettu. Tarkista internetyhteytesi ja yritä uudelleen.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'Vaihdon palveluntarjoaja palautti odottamattoman vastauksen, joten vaihto pysäytettiin. Yritä uudelleen.';

  @override
  String get walletSwapFaultSwapOff => 'Vaihto on pois päältä juuri nyt.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Talletustasi ei voitu lähettää, joten mitään ei lähtenyt lompakostasi. Hae uusi tarjous yrittääksesi uudelleen.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Vaihto on jo käynnissä. Voit aloittaa uuden, kun se on kokonaan selvitetty tai sen tarjous vanhenee — tämä voi kestää jonkin aikaa.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Tämä lompakko ei voi vielä määrittää palautusosoitetta — tämä tarkoittaa yleensä vain, että ensimmäinen synkronointi ei ole valmis. Odota, kunnes synkronointi on valmis, ja yritä sitten uudelleen.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Tämä lompakko ei voi vielä määrittää vastaanotto-osoitetta tälle vaihdolle — tämä tarkoittaa yleensä vain, että ensimmäinen synkronointi ei ole valmis. Odota, kunnes synkronointi on valmis, ja yritä sitten uudelleen.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'Vaihto ei käynnistynyt ajoissa — yhteys saattoi olla hidas, tai lompakko oli varattu. Hae uusi tarjous ja yritä uudelleen.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'Lompakko on hetken varattu. Yritä uudelleen.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Tämä tarjous ei vastaa lompakkosi antamaa tarjousta, joten mitään ei lähetetty. Hae uusi tarjous ja yritä uudelleen.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Tämä vaihto vaatii noin $needed ZEC verkkomaksuineen, mutta vain $spendable ZEC on juuri nyt käytettävissä.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Tämä sovellus rajoittaa vaihdot tällä hetkellä enimmäismäärään $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Tämä vaihto vaatii noin $needed ZEC verkkomaksuineen, mutta vain $spendable ZEC on juuri nyt käytettävissä. Saldosi kurottaa yhä kiinni — lisää saattaa tulla käytettäväksi pian.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'Lompakko ei voinut tallentaa tätä vaihtoa turvallisesti, joten mitään ei siirtynyt. Yritä uudelleen.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Tätä vaihtopyyntöä ei voitu käsitellä. Hae uusi tarjous ja yritä uudelleen.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Vaihtotarjouksen hakeminen ei onnistunut. Tarkista tiedot ja yritä uudelleen.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Lompakkosi ei ole valmis juuri nyt. Palaa takaisin ja yritä uudelleen.';

  @override
  String get walletSwapDirectionBuy => 'Osta ZEC';

  @override
  String get walletSwapDirectionSell => 'Myy ZEC';

  @override
  String get walletSwapRefundLabel => 'Palautusosoitteesi';

  @override
  String get walletSwapRefundHint =>
      'Tähän kolikkosi palautuvat, jos vaihto epäonnistuu';

  @override
  String get walletSwapRefundHelper =>
      'Verkossa, josta lähetät — ei Zcash-osoite.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Palautusosoitteesi verkossa $chain';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Osoite verkossa $chain — tähän kolikkosi palautuvat, jos vaihto epäonnistuu. Ei Zcash-osoite.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'Tietoa palautusosoitteestasi';

  @override
  String get walletSwapRefundInfoBody =>
      'Jos vaihtoa ei voida viedä loppuun, palveluntarjoaja lähettää kolikkosi takaisin tähän osoitteeseen sillä ketjulla, josta maksoit. Anna osoite, jota hallitset itse — lompakko ei voi tarkistaa vierasta osoitetta puolestasi, joten tarkista se huolellisesti.';

  @override
  String get walletSwapRefundScanTooltip =>
      'Skannaa palautusosoitteen QR-koodi';

  @override
  String get walletSwapScanTitle => 'Skannaa osoite';

  @override
  String get walletSwapScanInstruction =>
      'Kohdista kamera osoitteen QR-koodiin.';

  @override
  String get walletSwapScanManualEntry => 'Syötä manuaalisesti';

  @override
  String get walletSwapScanCancel => 'Peruuta';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Kamera ei ole käytettävissä. Syötä osoite manuaalisesti alla.';

  @override
  String get walletSwapSourceAssetLabel => 'Vaihdettava omaisuuserä';

  @override
  String get walletSwapSourceAssetHint => 'Valitse omaisuuserä';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Lähetettävä summa ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Lähetettävä summa';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol verkossa $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Valitse omaisuuserä, josta vaihdat';

  @override
  String get walletSwapPickerTitleReceive =>
      'Valitse vastaanotettava omaisuuserä';

  @override
  String get walletSwapPickerStale =>
      'Omaisuuserälistan päivitys epäonnistui — näytetään viimeisin tunnettu lista.';

  @override
  String get walletSwapPickerEmpty =>
      'Vaihdettavia omaisuuseriä ei ole juuri nyt saatavilla. Yritä myöhemmin uudelleen.';

  @override
  String get walletSwapPickerSearchHint => 'Hae nimellä tai verkolla';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Haku \"$query\" ei löytänyt omaisuuseriä.';
  }

  @override
  String get walletSwapPickerError =>
      'Omaisuuserälistan lataaminen epäonnistui. Tarkista yhteytesi ja yritä uudelleen.';

  @override
  String get walletSwapPickerRetry => 'Yritä uudelleen';

  @override
  String get walletSwapSlippageLabel => 'Liukumatoleranssi';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value %';
  }

  @override
  String get walletSwapSlippageCustom => 'Mukautettu';

  @override
  String get walletSwapSlippageCustomLabel => 'Mukautettu liukuma';

  @override
  String get walletSwapSlippageMayFail =>
      'Hyvin matala — vaihto voi epäonnistua, jos hinta muuttuu.';

  @override
  String get walletSwapSlippageNormal => 'Turvallinen toleranssi.';

  @override
  String get walletSwapSlippageRisky =>
      'Korkea — saatat saada huomattavasti vähemmän kuin tarjottu määrä.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Liian korkea — vaihto hylätään. Laske se 10 %:iin tai alle.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Saat vähintään $zec ZEC — $slippage %:n liukumarajasi. Lopullinen summa ei laske tämän alle.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'Vastaanotat ZEC:iä omaan osoitteeseesi';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Kunnes suojaat sen — yksi napautus, josta muistutetaan saapumisen yhteydessä — vastaanotettu summa on hetken julkinen ja näkyvissä lohkoketjussa. Pieni erä voi pysyä julkisena, kunnes sitä kertyy lisää.';

  @override
  String get walletSwapRefundVerifyTitle => 'Vahvista palautusosoitteesi';

  @override
  String get walletSwapRefundVerifyBody =>
      'Tarkista se merkki merkiltä — tähän kolikkosi palautuvat, jos vaihto epäonnistuu. Lompakko ei voi vahvistaa vierasta osoitetta puolestasi.';

  @override
  String get walletSwapRefundVerifyAck =>
      'Olen tarkistanut, että palautusosoitteeni on oikein.';

  @override
  String get walletSwapPayoutVerifyTitle => 'Vahvista vastaanotto-osoitteesi';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Tarkista se merkki merkiltä — tähän saat $asset. Lompakko ei voi vahvistaa vierasta osoitetta puolestasi.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'Olen tarkistanut, että vastaanotto-osoitteeni on oikein.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'Vaihto on täällä pois päältä. Jo matkalla oleva ZEC ilmestyy lompakkoosi seuraavan synkronoinnin jälkeen.';

  @override
  String get walletSwapFaultForeignAmountRequired => 'Anna vaihdettava summa.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Anna palautusosoitteesi lähdeketjussa.';

  @override
  String get walletSwapDepositTitle => 'Lähetä maksusi';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Lähetä täsmälleen $amount $asset verkossa $chain alla olevaan osoitteeseen.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Lähetä täsmälleen oikea summa. Jos lähetät liian vähän tai ikkunan sulkeutumisen jälkeen, palveluntarjoaja palauttaa varat palautusosoitteeseesi.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Talletusikkuna: $time jäljellä';
  }

  @override
  String get walletSwapDepositExpired =>
      'Tämä talletusikkuna on sulkeutunut. Älä lähetä varoja nyt — aloita uusi vaihto. Jos lähetit jo, palveluntarjoajan pitäisi palauttaa varat palautusosoitteeseesi.';

  @override
  String get walletSwapDepositQrLabel => 'Talletusosoitteen QR-koodi';

  @override
  String get walletSwapDepositAddressLabel => 'Talletusosoite';

  @override
  String get walletSwapDepositCopy => 'Kopioi talletusosoite';

  @override
  String get walletSwapDepositCopied => 'Talletusosoite kopioitu';

  @override
  String get walletSwapDepositMemoRequired =>
      'Tämä talletus vaatii muistion / tunnisteen';

  @override
  String get walletSwapDepositMemoWarning =>
      'Sinun TÄYTYY liittää tämä täsmällinen muistio talletukseesi. Lähettäminen ilman sitä — tai väärällä muistiolla — voi johtaa varojesi pysyvään menetykseen.';

  @override
  String get walletSwapDepositMemoLabel => 'Talletuksen muistio / tunniste';

  @override
  String get walletSwapDepositMemoCopy => 'Kopioi muistio';

  @override
  String get walletSwapDepositMemoCopied => 'Muistio kopioitu';

  @override
  String get walletSwapDepositSent => 'Olen lähettänyt varat';

  @override
  String get walletSwapDepositBackTitle => 'Poistutaanko tältä näytöltä?';

  @override
  String get walletSwapDepositBackBody =>
      'Tämä ei peruuta vaihtoasi — se jatkuu taustalla. Tarvitset kuitenkin talletusosoitteen maksamiseen, joten kopioi se ensin, jos et ole vielä tehnyt niin.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Tämä ei peruuta vaihtoasi — se jatkuu taustalla. Talletusikkuna on sulkeutunut. Älä lähetä varoja talletusosoitteeseen nyt. Jos lähetit jo, palveluntarjoajan pitäisi palauttaa varat palautusosoitteeseesi.';

  @override
  String get walletSwapDepositBackStay => 'Jää';

  @override
  String get walletSwapDepositBackLeave => 'Poistu';

  @override
  String get walletReceive => 'Vastaanota';

  @override
  String get walletReceiveSubtitle =>
      'Jaa tämä osoite vastaanottaaksesi ZEC:iä. Sen voi turvallisesti jakaa julkisesti.';

  @override
  String get walletReceiveCopy => 'Kopioi osoite';

  @override
  String get walletReceiveCopied => 'Osoite kopioitu';

  @override
  String get walletReceiveUnavailable => 'Lompakkosi ei ole vielä valmis.';

  @override
  String get walletReceiveError =>
      'Osoitteesi lataaminen epäonnistui. Yritä uudelleen.';

  @override
  String get walletReceivePreparing => 'Valmistellaan osoitettasi…';

  @override
  String get walletReceivePreparingHint =>
      'Lompakkosi valmistelee tämän osoitteen laitteellasi — tässä voi kestää hetki, jos lompakko on varattu muuhun työhön.';

  @override
  String get walletReceiveRetry => 'Yritä uudelleen';

  @override
  String get walletReceiveQrLabel => 'Vastaanotto-osoitteesi QR-koodi';

  @override
  String get walletReceiveTypeShielded => 'Suojattu';

  @override
  String get walletReceiveTypeTransparent => 'Julkinen';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Jaa tämä julkinen osoite vastaanottaaksesi ZEC:iä lähettäjältä, joka ei voi maksaa suojattuun osoitteeseen.';

  @override
  String get walletReceiveTransparentWarning =>
      'Tämä on julkinen osoite: se näkyy lohkoketjussa ja yhdistää maksusi toisiinsa, jos sitä käytetään uudelleen. Suosi suojattua osoitettasi; suojaa nämä varat vastaanottamisen jälkeen.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'Julkisen vastaanotto-osoitteesi QR-koodi';

  @override
  String get walletReceiveFreshAddress => 'Käytä uutta osoitetta';

  @override
  String get walletReceiveFreshCaption =>
      'Uusi osoite — sitä ei voi yhdistää muihin osoitteisiisi. Maksut siihen saapuvat silti tähän lompakkoon, ja aiemmat osoitteesi toimivat edelleen. Sitä ei näytetä täällä enää uudelleen — kopioi se nyt.';

  @override
  String get walletReceiveFreshError =>
      'Uutta osoitetta ei voitu luoda. Yritä uudelleen.';

  @override
  String get walletReceiveFreshBusy =>
      'Lompakko on juuri nyt varattu. Yritä uutta osoitetta uudelleen hetken kuluttua.';

  @override
  String get walletReceiveShare => 'Jaa';

  @override
  String get walletReceiveRequestAmount => 'Pyydä summaa';

  @override
  String get walletReceiveRequestAmountLabel => 'Summa (valinnainen)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Sitä ei näytetä täällä enää uudelleen — kopioi se nyt.';

  @override
  String get walletSecurityMenuItem => 'Tietoturva…';

  @override
  String get securityTitle => 'Tietoturva';

  @override
  String get securityUnavailableBody =>
      'Lompakon tietoturva-asetuksia hallinnoi tämä sovellus, ei lompakko itse.';

  @override
  String get securityCustodySectionTitle => 'Avainten säilytys';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (laitteisto)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (laitteisto)';

  @override
  String get securityCustodyTierTee => 'Laitteistopohjainen avainsäilö (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Ohjelmistopohjainen avainsäilö';

  @override
  String get securityCustodyTierKeychain => 'Keychain (ohjelmistosalattu)';

  @override
  String get securityCustodyTierNone => 'Ei laitteistopohjaista avainsäilöä';

  @override
  String get securityCustodyTierUnknown => 'Tuntematon';

  @override
  String get securityCustodyHardwareKey =>
      'Tämän lompakon lukitseva avain säilytetään laitteen suojatussa laitteistossa, ja se poistetaan lompakon mukana.';

  @override
  String get securityCustodyBestEffort =>
      'Poistaminen poistaa avaimesi parhaan kykynsä mukaan; lyhyt forensinen palautusikkuna voi jäädä jäljelle, kunnes laite ottaa tallennustilan uudelleen käyttöön. Täyttä varmuutta varten käytä myös laitteesi kaikkien sisältöjen tyhjennystoimintoa.';

  @override
  String get securityCustodyProbeError =>
      'Säilytyksen tilan lukeminen epäonnistui. Palaa takaisin ja yritä uudelleen.';

  @override
  String get securityDeleteWalletButton => 'Poista lompakko';

  @override
  String get securityDeleteWalletSubtitle =>
      'Poista tämä lompakko ja sen avain tältä laitteelta. Varasi säilyvät lohkoketjussa ja ovat palautettavissa palautuslauseestasi.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Poista tämä lompakko ja sen avain tältä laitteelta. Sillä ei ole kulutusavaimia, joten siinä ei ole mitään varmuuskopioitavaa — lisää se takaisin milloin tahansa sen katseluavaimella.';

  @override
  String get securityDeleteDialogTitle => 'Poistetaanko tämä lompakko?';

  @override
  String get securityDeleteDialogBody =>
      'Tämä poistaa lompakon ja sen avaimen tältä laitteelta. Varmista, että olet varmuuskopioinut palautuslauseesi — se on AINOA tapa palauttaa varasi.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Tämä poistaa lompakon ja sen avaimen tältä laitteelta. Sillä ei ole kulutusavaimia, joten mitään ei tarvitse varmuuskopioida — voit lisätä sen takaisin myöhemmin sen katseluavaimella.';

  @override
  String get securityDeleteDialogConfirm => 'Poista';

  @override
  String get securityDeleteDialogCancel => 'Peruuta';

  @override
  String get securityDeleteFailedSnack =>
      'Lompakon poistaminen epäonnistui — lompakkosi on ennallaan. Yritä uudelleen.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Viimeistele ensin palvelimen vaihto — se valmistuu tai pysähtyy $seconds sekunnin kuluessa. Yritä sitten poistaa lompakko uudelleen.';
  }

  @override
  String get walletParkedTitle => 'Tallennettu ja vireillä';

  @override
  String get walletParkedSubtitle =>
      'Näitä maksuja ei ole vielä lähetetty. Niiden summat ovat yhä osa saldoasi.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Näitä maksuja ei ole vielä lähetetty. Niiden summat ovat yhä osa saldoasi — paitsi niiden, joita lompakkosi parhaillaan lähettää: niiden summa saattaa jo olla varattuna.';

  @override
  String get walletParkedCancel => 'Peruuta';

  @override
  String get walletParkedPausedHint =>
      'Keskeytetty — tämä maksu ei lähde itsestään. Varasi ovat turvassa. Lähetä se nyt, tai peruuta se.';

  @override
  String get walletParkedRetryStale =>
      'Tämä maksu ei ole enää vireillä. Tarkista vireillä olevat maksusi ja tapahtumasi.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Tämä maksu ei ole enää vireillä — lompakkosi saattaa jo lähettää sitä. Tarkista Tallennettu ja vireillä sekä tapahtumasi.';

  @override
  String get walletReclaimExplainer =>
      'Kertakäyttöisten osoitteiden lähetykset ovat jumissa. Voit avata ne uudelleen — se siirtää pienen summan omien osoitteidesi välillä ja palauttaa sen.';

  @override
  String get walletReclaimButton => 'Avaa lähetys uudelleen';

  @override
  String get walletReclaimInProgress => 'Avataan uudelleen…';

  @override
  String get walletReclaimConfirmTitle =>
      'Avataanko kertakäyttöisten osoitteiden lähettäminen uudelleen?';

  @override
  String get walletReclaimConfirmBody =>
      'Tämä siirtää pienen summan omien osoitteidesi välillä, jotta kertakäyttöisten osoitteiden lähettäminen vapautuu, ja palauttaa sen sitten. Se maksaa muutaman verkkomaksun. Kun siirto vahvistuu, palauta siirretty summa Palauta nyt -painikkeella.';

  @override
  String get walletReclaimConfirmCancel => 'Ei nyt';

  @override
  String get walletReclaimConfirmAction => 'Avaa uudelleen';

  @override
  String get walletReclaimStarted =>
      'Uudelleenavaus käynnistyi. Kun se vahvistuu, lähetä keskeytetty maksu ja palauta sitten siirretty summa Palauta nyt -painikkeella.';

  @override
  String get walletReclaimNothing => 'Ei mitään avattavaa juuri nyt.';

  @override
  String get walletReclaimNotBroadcast =>
      'Emme voineet varmistaa, että se saavutti verkon. Se on voinut silti mennä läpi. Yritä hetken kuluttua uudelleen.';

  @override
  String get walletReclaimNeedsFunds =>
      'Tarvitset suojattua ZEC:iä avataksesi lähettämisen uudelleen.';

  @override
  String get walletReclaimFailed =>
      'Uudelleenavaus ei onnistunut juuri nyt. Varasi ovat ennallaan. Yritä uudelleen.';

  @override
  String get walletReclaimUnknown =>
      'Uudelleenavaus päättyi. Tarkista kertakäyttöisten osoitteiden lähetyksesi ja palauta mahdollinen siirretty summa Palauta nyt -painikkeella.';

  @override
  String get walletParkedError =>
      'Vireillä olevien maksujesi lataaminen epäonnistui juuri nyt.';

  @override
  String get walletParkedErrorRetry => 'Yritä uudelleen';

  @override
  String get walletParkedErrorRetryInProgress => 'Yritetään…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Peruutetaanko tämä vireillä oleva maksu?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Tämä hylkää tallennetun maksun. Sitä ei ole lähetetty, joten mitään ei lähde lompakostasi — mutta tätä ei voi peruuttaa.';

  @override
  String get walletParkedCancelConfirmKeep => 'Säilytä';

  @override
  String get walletParkedCancelConfirmDiscard => 'Hylkää maksu';

  @override
  String get walletParkedCancelDone => 'Vireillä ollut maksu peruutettu.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Tämä maksu saattaa jo olla matkalla — tarkista tapahtumasi.';

  @override
  String get walletParkedCancelFailed =>
      'Peruuttaminen epäonnistui juuri nyt. Maksusi on ennallaan. Yritä uudelleen.';

  @override
  String get walletRecoverNow => 'Palauta nyt';

  @override
  String get walletRecoverConfirmTitle => 'Palautetaanko suojattuun saldoosi?';

  @override
  String get walletRecoverConfirmBody =>
      'Tämä tarkistaa kertakäyttöiset osoitteesi ja siirtää kaiken löydetyn yksityiseen suojattuun saldoosi. Sen voi suorittaa turvallisesti uudelleen milloin tahansa.';

  @override
  String get walletRecoverConfirmCancel => 'Ei nyt';

  @override
  String get walletRecoverConfirmAction => 'Palauta';

  @override
  String get walletRecoverInProgress => 'Palautetaan…';

  @override
  String walletRecoverDone(String amount) {
    return 'Palautetaan $amount suojattuun saldoosi.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'Palautetaan $amount — osa varoista tarvitsee vielä uuden yrityksen.';
  }

  @override
  String get walletRecoverRetry =>
      'Osa varoista tarvitsee vielä uuden yrityksen — suorita palautus uudelleen.';

  @override
  String get walletRecoverTruncated =>
      'Kaikkia kertakäyttöisiä osoitteita ei vielä tarkistettu — suorita se uudelleen tarkistaaksesi loput.';

  @override
  String get walletRecoverNothing => 'Ei mitään palautettavaa juuri nyt.';

  @override
  String get walletRecoverFailed =>
      'Palautus ei onnistunut juuri nyt. Varasi ovat ennallaan. Yritä uudelleen.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount tallennettu ja vireillä · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Peruuta $amount maksu, tallennettu $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount keskeytetty · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount valmistellaan lähetystä varten · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Lompakkosi valmistelee tätä maksua — sen summa saattaa jo olla varattuna. Varasi ovat turvassa. Jos se ei valmistu, se palaa listalle itsestään.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Lompakkosi valmistelee tätä maksua — sen summa saattaa jo olla varattuna. Varasi ovat turvassa, mutta maksu voi valmistua vasta, kun lompakkosi synkronoi taas.';

  @override
  String get walletParkedSendNow => 'Lähetä nyt';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Lähetetään $amount maksu, tallennettu $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Lähetä nyt $amount maksu, tallennettu $time';
  }

  @override
  String get walletParkedSendNowInProgress => 'Lähetetään…';

  @override
  String get walletParkedAuthorizeSent => 'Maksuasi lähetetään nyt.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Maksuasi lähetetään nyt. Jos se ei mene läpi, lompakkosi voi viimeistellä sen vasta, kun se synkronoi taas.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Ei vielä valmis lähetettäväksi. Maksusi on tallennettu ja ennallaan.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Ei vielä valmis lähetettäväksi. Maksusi on tallennettu eikä se ole enää keskeytetty — yritä Lähetä nyt -painiketta myöhemmin uudelleen tai peruuta se.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Lähetys epäonnistui juuri nyt. Maksusi on ennallaan. Yritä uudelleen.';

  @override
  String get walletTransparentFundsMenuItem => 'Julkiset varat…';

  @override
  String get walletTransparentFundsTitle => 'Julkiset varat';

  @override
  String get walletTransparentFundsIntro =>
      'Julkiset varat näkyvät julkisesti lohkoketjussa — summa, osoitteet ja kolikoiden historia.';

  @override
  String get walletExpertToggleLabel => 'Lisäasetukset: julkiset varat';

  @override
  String get walletExpertToggleDescription =>
      'Näytä lisäasetukset, joilla voit pitää julkisia varoja ja poistaa automaattisen suojauksen käytöstä.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Näytä lisäasetukset, joilla voit pitää julkisia varoja.';

  @override
  String get walletAutoShieldToggleLabel => 'Suojaa automaattisesti';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Kun julkinen saldosi saavuttaa $minZec ZEC, se siirretään automaattisesti suojattuun saldoosi. Jos tämä on pois päältä, julkiset varat pysyvät julkisesti näkyvinä, kunnes suojaat ne itse.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Asetuksen tallentaminen ei onnistunut. Yritä uudelleen.';

  @override
  String get walletAutoShieldIncomplete =>
      'Automaattinen suojaus ei valmistunut — nämä varat näkyvät yhä julkisesti. Voit suojata ne nyt.';

  @override
  String get walletSendPrivacyShielded =>
      'Suojattu maksu — summa ja vastaanottaja pysyvät yksityisinä lohkoketjussa.';

  @override
  String get walletSendPrivacyTransparent =>
      'Julkinen maksu — summa ja osoitteet näkyvät lohkoketjussa.';

  @override
  String get walletActivityPublicBadge => 'Julkisesti näkyvä lohkoketjussa';

  @override
  String get walletShieldWalletEnded =>
      'Lompakkoistunto päättyi. Sulje ja avaa uudelleen yrittääksesi uudestaan.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Uudet julkiset varat suojataan automaattisesti yksityiseen saldoosi, kun ne saavuttavat $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'Automaattinen suojaus on pois päältä — julkiset varat pysyvät julkisesti näkyvinä, kunnes suojaat ne.';

  @override
  String get walletMoveAutoShieldNote =>
      'Automaattinen suojaus on päällä: kun nämä varat saapuvat, ne suojataan takaisin automaattisesti (lisämaksua vastaan). Jos haluat pitää ne julkisina, poista ensin automaattinen suojaus käytöstä kohdassa Julkiset varat.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Tämän siirron jälkeen julkinen saldosi on $amount ZEC — alle $floor ZEC, joka tarvitaan sen suojaamiseen uudelleen. Se pysyy julkisena, kunnes varoja tulee lisää.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Siirrät varoja omalle julkiselle osoitteellesi. Tämä siirto jää pysyvästi julkiseen tietoon.';

  @override
  String get walletTxDetailVisibility => 'Näkyvyys';

  @override
  String get walletTransparentFundsAutoDenied =>
      'Automaattinen suojaus on keskeytetty tätä istuntoa varten — sitä ei hyväksytty. Voit silti suojata manuaalisesti.';

  @override
  String get walletDeepScanMenuItem => 'Tarkista vanhemmat vaihto-osoitteet…';

  @override
  String get walletMenuSyncNotRunningHint =>
      'Synkronointi ei ole juuri nyt käynnissä.';

  @override
  String get walletDeepScanTitle => 'Tarkista vanhemmat vaihto-osoitteet';

  @override
  String get walletDeepScanBody =>
      'Jos palautit tämän lompakon ja se käytti aiemmin paljon vaihtoja, sen vanhimpien vaihtojen varat voivat vaatia ylimääräisen vaiheen löytyäkseen. Tämä tarkistaa asian — kaikki löytynyt näkyy saldossasi, kun lompakkosi synkronoituu.';

  @override
  String get walletDeepScanCoverage =>
      'Vanhemmat vaihto-osoitteesi on tarkistettu tähän mennessä. Jos vanhan vaihdon varoja puuttuu yhä, tarkista vielä syvemmältä.';

  @override
  String get walletDeepScanCoveragePending =>
      'Nykyistä väliä tarkistetaan vielä — kaikki löytynyt näkyy saldossasi. Tämä voi kestää hetken.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Tarkistaa lompakkosi vanhimpien vaihtojen varoja.';

  @override
  String get walletDeepScanCheckButton => 'Tarkista vanhemmat osoitteet';

  @override
  String get walletDeepScanCheckDeeperButton =>
      'Tarkista vieläkin vanhemmat osoitteet';

  @override
  String get walletDeepScanChecking => 'Tarkistetaan…';

  @override
  String get walletDeepScanClose => 'Sulje';

  @override
  String get walletDeepScanTorHint =>
      'Et ole tällä hetkellä yhteydessä Torin kautta. Yksityisyyden lisäämiseksi harkitse odottamista, kunnes Tor on käytössä, ennen kuin tarkistat.';

  @override
  String get walletDeepScanRescanBusy =>
      'Voit tarkistaa vanhempia vaihto-osoitteita, kun uudelleenskannaus on valmis.';

  @override
  String get walletDeepScanRan =>
      'Vanhempia vaihto-osoitteita tarkistetaan — kaikki löytynyt näkyy saldossasi.';

  @override
  String get walletDeepScanFailed =>
      'Tarkistusta ei voitu käynnistää. Mikään ei muuttunut — yritä uudelleen.';

  @override
  String get walletDeepScanSlow =>
      'Tämä kestää tavallista kauemmin. Jos vanhemmat vaihto-osoitteesi tarkistettiin, kaikki löytynyt näkyy saldossasi — tarkista pian uudelleen.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'Vaihto on tällä hetkellä pois käytöstä, joten tätä ei voi suorittaa. Yritä uudelleen, kun vaihto on käytettävissä.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Viimeistä väliä tarkistetaan vielä — tämä voi kestää jopa pari päivää, mutta yleensä paljon vähemmän. Se valmistuu itsestään. Tarkista uudelleen myöhemmin.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'Emme voi vielä vahvistaa yhteytesi yksityisyyttä. Yksityisyyden lisäämiseksi tarkista vasta, kun Tor on käytössä.';

  @override
  String get walletDeepScanBannerChecking =>
      'Vanhempia vaihto-osoitteita tarkistetaan vielä — kaikki löytynyt näkyy saldossasi.';

  @override
  String get walletRescanSwapPointer =>
      'Etsitkö varoja vanhasta vaihdosta? Uudelleenskannaus ei löydä sitä — käytä sen sijaan “Tarkista vanhemmat vaihto-osoitteet”.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Palautitko lompakon, joka käytti vaihtoja?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Jos tällä lompakolla oli hyvin pitkä vaihtohistoria, sen vanhimpien vaihtojen varat voivat vaatia ylimääräisen vaiheen löytyäkseen. Useimmat lompakot eivät tarvitse mitään.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Tarkista nyt';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Hylkää';

  @override
  String walletTorHostPath(String transport) {
    return 'Sovelluksesi yksityisen reitin kautta ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Sovelluksesi yksityisen reitin kautta ($transport); välityspalvelin voi yhdistää yhteydet toisiinsa';
  }

  @override
  String get walletTorHostOtherTransport => 'yksityinen reitti';

  @override
  String get walletTorHostDirect => 'Ei yksityinen (sovelluksesi suora yhteys)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Tallennettu palvelin käyttää salaamatonta osoitetta, jota sovelluksesi yksityinen reitti ei voi välittää. Käytetään palvelinta $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Lisää aiheesta $label';
  }

  @override
  String get walletSendPaste => 'Liitä';

  @override
  String get walletSendScanQr => 'Skannaa QR-koodi';

  @override
  String get walletSendRecipientGetsLabel => 'Vastaanottaja saa';

  @override
  String get walletSwapDepositCopyAmount => 'Kopioi summa';

  @override
  String get walletSwapDepositAmountCopied => 'Summa kopioitu';

  @override
  String get walletScanOpenSettings => 'Avaa asetukset';

  @override
  String get walletScanOpenSettingsFailed => 'Asetuksia ei voitu avata.';

  @override
  String get walletSendLeaveTitle => 'Lähetys on yhä käynnissä';

  @override
  String get walletSendLeaveBody =>
      'Maksusi jatkuu, vaikka poistuisit. Näet lopputuloksen tapahtumistasi.';

  @override
  String get walletSendLeaveStay => 'Pysy';

  @override
  String get walletSendLeaveConfirm => 'Poistu';

  @override
  String get walletSheetLeaveBody =>
      'Tämä jatkuu, vaikka poistuisit. Näet lopputuloksen tapahtumistasi.';

  @override
  String get walletLoadingLabel => 'Ladataan';

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
      'Tarkista ennen kuin suojaat uudelleen';

  @override
  String get walletShieldUnknownBody =>
      'Emme voineet vahvistaa tätä suojausta. Tarkista Tapahtumat ennen kuin yrität uudelleen.';

  @override
  String get walletMoveUnknownTitle => 'Tarkista ennen kuin siirrät uudelleen';

  @override
  String get walletMoveUnknownBody =>
      'Emme voineet vahvistaa tätä siirtoa. Tarkista Tapahtumat ennen kuin yrität uudelleen.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
