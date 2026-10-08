// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Italian (`it`).
class WalletLocalizationsIt extends WalletLocalizations {
  WalletLocalizationsIt([String locale = 'it']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Impostazioni';

  @override
  String get walletTitle => 'Portafoglio';

  @override
  String get walletNotSetUpTitle => 'Portafoglio non ancora configurato';

  @override
  String get walletNotSetUpBody =>
      'La configurazione del portafoglio arriverà in una versione successiva. Prima di poter ricevere fondi, sarà necessario annotare la frase di recupero — così nulla è mai a rischio senza un backup.';

  @override
  String get walletStartupFailedTitle => 'Impossibile avviare il portafoglio';

  @override
  String get walletStartupFailedBody =>
      'Qualcosa ha impedito il caricamento del portafoglio su questo dispositivo. Se ha già un portafoglio, i suoi fondi non sono interessati — si trovano sulla rete Zcash e possono essere ripristinati con la frase di recupero. Riprovi; se il problema persiste, chiuda e riapra l\'app.';

  @override
  String get walletBalanceLabel => 'Saldo';

  @override
  String get walletHideBalance => 'Nascondi saldo';

  @override
  String get walletShowBalance => 'Mostra saldo';

  @override
  String get walletBalanceHiddenAmount => 'Saldo nascosto';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Spendibile ora';

  @override
  String get walletArrivingLabel => 'In arrivo';

  @override
  String get walletNotSpendableYetLabel => 'Non ancora spendibile';

  @override
  String get walletActivityTitle => 'Attività';

  @override
  String get walletActivityEmpty => 'Nessuna attività';

  @override
  String get walletActivityError => 'Impossibile caricare l\'attività';

  @override
  String get walletActivityReceived => 'Ricevuto';

  @override
  String get walletActivitySent => 'Inviato';

  @override
  String get walletActivityPending => 'In sospeso';

  @override
  String get walletActivityQueued => 'In coda';

  @override
  String get walletActivityRetrying => 'Nuovo tentativo';

  @override
  String get walletActivitySaved => 'Salvata';

  @override
  String get walletActivityExpired => 'Scaduto';

  @override
  String get walletActivityFailed => 'Non riuscito';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count conferme',
      one: '1 conferma',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pagamenti ricevuti',
      one: 'Pagamento ricevuto',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Mostra i dettagli della transazione';

  @override
  String get walletTxDetailStatus => 'Stato';

  @override
  String get walletTxDetailFee => 'Commissione di rete';

  @override
  String get walletTxDetailDate => 'Data';

  @override
  String get walletTxDetailHeight => 'Altezza del blocco';

  @override
  String get walletTxDetailMemo => 'Memo';

  @override
  String get walletTxDetailMemoAttached => 'Incluso';

  @override
  String get walletTxDetailTxid => 'ID transazione';

  @override
  String get walletTxDetailCopyTxid => 'Copia ID transazione';

  @override
  String get walletTxDetailCopied => 'ID transazione copiato';

  @override
  String get walletTxDetailClose => 'Chiudi';

  @override
  String get walletTxFundsKept => 'Nessun fondo è uscito dal portafoglio';

  @override
  String get walletTxExplainQueued =>
      'Salvata su questo dispositivo, in Salvati e in sospeso — può inviarla o annullarla da lì.';

  @override
  String get walletTxExplainPending =>
      'Inviata alla rete Zcash — in attesa di conferma in un blocco.';

  @override
  String get walletTxExplainRetrying =>
      'Il portafoglio non è ancora riuscito a inviare questa transazione alla rete Zcash. Conserva la transazione firmata e riprova a ogni sincronizzazione finché non passa o scade.';

  @override
  String get walletTxExplainSaved =>
      'Il portafoglio ha conservato questa transazione firmata, ma al momento non la invia da solo.';

  @override
  String get walletTxExplainConfirmed => 'Confermata sulla rete Zcash.';

  @override
  String get walletTxExplainExpired =>
      'Questa transazione è scaduta prima che la rete la confermasse, quindi è stata annullata. L\'importo resta disponibile per essere speso.';

  @override
  String get walletTxExplainFailed =>
      'La rete ha rifiutato questa transazione, che quindi non è andata a buon fine. L\'importo resta disponibile per essere speso.';

  @override
  String get walletTxExplainUnknown =>
      'Lo stato attuale di questa transazione non può essere determinato. Sarà aggiornato dopo la prossima sincronizzazione.';

  @override
  String get walletMenuTooltip => 'Altre opzioni';

  @override
  String get walletRescanMenuItem => 'Ripeti la scansione della cronologia…';

  @override
  String get walletCheckOneTimeMenuItem => 'Controlla gli indirizzi monouso…';

  @override
  String get walletRescanTitle => 'Ripeti la scansione della cronologia';

  @override
  String get walletRescanBody =>
      'Mancano fondi meno recenti? Ripeta la scansione della blockchain partendo da più indietro per recuperare i depositi saltati da una data di inizio troppo recente. I fondi e la frase di recupero non sono mai a rischio.';

  @override
  String get walletRescanRangeTitle => 'Da quanto tempo indietro scansionare';

  @override
  String get walletRescanRangeAll =>
      'Scansiona l\'intera cronologia — più lento, ma recupera tutto.';

  @override
  String get walletRescanRangeDefault =>
      'Scansione a partire dall\'inizio del portafoglio. Mancano ancora fondi meno recenti? Scelga una data precedente, oppure Scansiona tutta la cronologia.';

  @override
  String get walletRescanRangeResolving =>
      'Preparazione dell\'intervallo consigliato…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Circa $blocks blocchi da scansionare.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Scansione a partire dal $date. Mancano ancora fondi meno recenti? Scelga una data precedente, oppure Scansiona tutta la cronologia.';
  }

  @override
  String get walletRescanPick => 'Scegli una data';

  @override
  String get walletRescanChange => 'Cambia data';

  @override
  String get walletRescanScanAll => 'Scansiona tutta la cronologia';

  @override
  String get walletRescanDatePick => 'Data più remota da scansionare';

  @override
  String get walletRescanWarning =>
      'Questa operazione ripete la scansione della blockchain. Le date recenti richiedono minuti; risalire molto indietro può richiedere ore. La sincronizzazione avviene in background: è possibile continuare a usare il portafoglio.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Un pagamento da questo portafoglio è ancora in fase di conferma. Di norma il portafoglio rifiuta la nuova scansione finché non si completa — è possibile provare, ma va previsto un rifiuto.';

  @override
  String get walletRescanConfirm => 'Avvia la nuova scansione';

  @override
  String get walletRescanCancel => 'Annulla';

  @override
  String get walletRescanRunning => 'Ricostruzione in corso…';

  @override
  String get walletRescanRebuildingAll =>
      'Ricostruzione della cronologia in corso — scansione dell\'intera catena. Saldo e attività si aggiornano man mano che procede.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Ricostruzione della cronologia dal $date — saldo e attività si aggiornano man mano che procede.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Ricostruzione della cronologia dall\'inizio del portafoglio — saldo e attività si aggiornano man mano che procede.';

  @override
  String get walletCatchUpBanner =>
      'Aggiornamento in corso — saldo e attività si aggiornano man mano che il portafoglio si sincronizza. Tutto ciò che hai ricevuto è al sicuro.';

  @override
  String get walletCatchUpRescanBanner =>
      'Ricostruzione della cronologia dopo una nuova scansione — saldo e attività si aggiornano man mano che procede. Tutto ciò che hai ricevuto è al sicuro.';

  @override
  String get walletRescanFailedNotice =>
      'Impossibile ripetere la scansione in questo momento — i fondi sono al sicuro, anche se il saldo e la cronologia potrebbero impiegare un po\' di tempo per aggiornarsi. Riprovare tra poco.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Un pagamento è ancora in fase di conferma, quindi la nuova scansione è stata sospesa per proteggere i fondi. Il portafoglio è invariato — riprovare tra un paio d\'ore e tenere l\'app aperta e connessa nel frattempo.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'La nuova scansione ricostruisce la cronologia mentre il portafoglio si sincronizza, e la sincronizzazione non è in corso in questo momento. Il portafoglio è invariato — riprovare una volta che la sincronizzazione sarà in corso.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Non c\'è spazio libero sufficiente per ricostruire la cronologia del portafoglio — i fondi sono al sicuro, anche se il saldo e la cronologia potrebbero impiegare un po\' di tempo per aggiornarsi. Liberi dello spazio e riprovi.';

  @override
  String get walletRescanFailedDismiss => 'Chiudi';

  @override
  String get walletActivityRebuilding => 'Ricostruzione della cronologia…';

  @override
  String get walletActivityCatchingUp =>
      'Aggiornamento ancora in corso — tutto ciò che hai ricevuto comparirà qui.';

  @override
  String get walletActivitySyncNotRunning =>
      'Il saldo e la cronologia finiranno di caricarsi una volta che la sincronizzazione sarà in corso.';

  @override
  String get walletActivityLoadMore => 'Carica altro';

  @override
  String get walletPendingChangeLabel => 'Resto in sospeso';

  @override
  String get walletTransparentLabel => 'Non schermato (pubblico)';

  @override
  String get walletTransparentNote =>
      'Non inclusi in \"Spendibile ora\" — schermare questi fondi per poterli spendere. Fino ad allora restano pubblicamente visibili sulla blockchain.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Questi fondi sono pubblicamente visibili sulla blockchain.';

  @override
  String walletPoolShielded(String amount) {
    return 'Schermato $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Pubblico $amount';
  }

  @override
  String get walletPoolAllShielded => 'Tutto schermato · privato';

  @override
  String get walletPoolTapHint => 'Mostra i fondi pubblici';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount del saldo si trova su un indirizzo monouso (recuperabile).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount del saldo si trova su un indirizzo monouso.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Pagamenti per un totale di $amount sono riservati e ancora in completamento tramite indirizzi monouso sotto il controllo del portafoglio. Non li invii di nuovo.',
      one:
          '$amount è riservato per un pagamento che il portafoglio sta ancora completando tramite un indirizzo monouso sotto il suo controllo. Non lo invii di nuovo.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Pagamenti per un totale di $amount sono riservati e a metà strada tramite indirizzi monouso sotto il controllo del portafoglio. Sono in pausa finché il portafoglio non torna a sincronizzarsi. Non li invii di nuovo.',
      one:
          '$amount è riservato per un pagamento a metà strada tramite un indirizzo monouso sotto il controllo del portafoglio. È in pausa finché il portafoglio non torna a sincronizzarsi. Non lo invii di nuovo.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Impossibile verificare se un pagamento è ancora in corso. Nuovo tentativo in corso: nel frattempo, cerca un pagamento in sospeso nella tua attività prima di inviare di nuovo.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount del saldo si trova su un indirizzo monouso (in fase di conferma).';
  }

  @override
  String get walletShieldButton => 'Scherma';

  @override
  String get walletShieldSheetTitle => 'Scherma i fondi pubblici';

  @override
  String get walletShieldNote =>
      'Questa operazione sposta i fondi dal saldo pubblico, visibile sulla blockchain, al saldo privato schermato.';

  @override
  String get walletShieldPreparing => 'Preparazione in corso…';

  @override
  String get walletShieldAmountLabel => 'Schermatura';

  @override
  String get walletShieldFeeLabel => 'Commissione di rete';

  @override
  String get walletShieldNetLabel => 'Netto schermato';

  @override
  String get walletShieldConfirmButton => 'Scherma ora';

  @override
  String get walletShieldSubmitting => 'Schermatura in corso…';

  @override
  String get walletShieldNothingTitle => 'Niente da schermare per ora';

  @override
  String get walletShieldNothingBody =>
      'Questi fondi sono al momento inferiori all\'importo minimo conveniente da schermare: la commissione di rete supererebbe il beneficio. Saranno schermabili non appena arriverà qualcosa in più.';

  @override
  String get walletShieldDoneTitle => 'Schermatura inviata';

  @override
  String get walletShieldDoneBody =>
      'I fondi si stanno spostando nel saldo schermato. La conferma sulla blockchain avverrà a breve.';

  @override
  String get walletShieldSavedTitle => 'Salvato — completeremo la schermatura';

  @override
  String get walletShieldSavedBody =>
      'Non è stato possibile raggiungere la rete in questo momento. La schermatura è stata salvata e il portafoglio la completerà a una sincronizzazione successiva. Nulla è andato perso.';

  @override
  String get walletShieldAlreadyTitle => 'Già inviato';

  @override
  String get walletShieldFailedTitle =>
      'Impossibile schermare in questo momento';

  @override
  String get walletShieldStaleBody =>
      'Il portafoglio è ancora in sincronizzazione. Riprovare a schermare tra poco.';

  @override
  String get walletShieldTransientBody =>
      'Impossibile preparare la schermatura in questo momento. Riprova tra un istante.';

  @override
  String get walletShieldStorageFullBody =>
      'Non c\'è spazio libero sufficiente per schermare ora. Liberare dello spazio e riprovare. I fondi sono al sicuro.';

  @override
  String get walletShieldClose => 'Chiudi';

  @override
  String get walletShieldRetry => 'Riprova';

  @override
  String get walletMoveMenuItem => 'Sposta su pubblico…';

  @override
  String get walletMoveSheetTitle => 'Sposta su pubblico';

  @override
  String get walletMoveSheetSubtitle =>
      'Invia ZEC schermati al proprio indirizzo pubblico — utile per un exchange che non accetta depositi schermati.';

  @override
  String get walletMoveDestinationLabel => 'Il proprio indirizzo pubblico';

  @override
  String walletMoveAvailable(String amount) {
    return 'Disponibile per lo spostamento: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Disponibile per lo spostamento: $amount ZEC — il saldo si sta ancora aggiornando';
  }

  @override
  String get walletMoveDeshieldTitle =>
      'Questo spostamento rende pubblici i fondi';

  @override
  String get walletMoveDeshieldBody =>
      'Spostare questi fondi su un indirizzo pubblico li fa uscire dal saldo schermato — l\'importo e l\'indirizzo pubblico diventano pubblicamente visibili sulla blockchain di Zcash.';

  @override
  String get walletMoveWalletEnded =>
      'La sessione del portafoglio è terminata. Chiudere e riaprire per riprovare.';

  @override
  String get walletMoveLoading => 'Preparazione in corso…';

  @override
  String get walletMovePreparing => 'Verifica dell\'importo…';

  @override
  String get walletMoveSubmitting => 'Spostamento in corso…';

  @override
  String get walletMoveReviewButton => 'Verifica';

  @override
  String get walletMoveCancel => 'Annulla';

  @override
  String get walletMoveReviewTitle => 'Verifica lo spostamento';

  @override
  String get walletMoveOwnAddressNote =>
      'Lo spostamento avviene verso il proprio indirizzo pubblico. Sarà possibile schermare di nuovo questi fondi in seguito, ma questo movimento resta permanentemente nel registro pubblico.';

  @override
  String get walletMoveConfirmButton => 'Sposta su pubblico';

  @override
  String get walletMoveBackButton => 'Indietro';

  @override
  String get walletMoveDoneTitle => 'Spostato su pubblico';

  @override
  String get walletMoveDoneBody =>
      'I fondi si stanno spostando verso l\'indirizzo pubblico. Saranno confermati sulla blockchain a breve.';

  @override
  String get walletMoveSavedTitle => 'Salvato — completeremo lo spostamento';

  @override
  String get walletMoveSavedBody =>
      'Questo spostamento è stato salvato e il portafoglio lo invierà a una sincronizzazione successiva. Nulla è andato perso.';

  @override
  String get walletMoveAlreadyTitle => 'Già inviato';

  @override
  String get walletMoveAlreadyBody =>
      'Questi fondi sono già stati inviati e sono in transito verso l\'indirizzo pubblico.';

  @override
  String get walletMoveFailedTitle => 'Impossibile completare lo spostamento';

  @override
  String get walletMoveNothingTitle => 'Niente da spostare per ora';

  @override
  String get walletMoveNothingBody =>
      'Al momento non è disponibile alcun saldo schermato da spostare. Una volta confermati, i fondi potranno essere spostati sull\'indirizzo pubblico.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Il portafoglio si sta ancora aggiornando — tutto ciò che hai ricevuto diventerà disponibile per lo spostamento una volta completata la sincronizzazione.';

  @override
  String get walletMoveCouldNotLoad =>
      'Impossibile caricare l\'indirizzo pubblico. Riprovare.';

  @override
  String get walletMoveRetry => 'Riprova';

  @override
  String get walletMoveClose => 'Chiudi';

  @override
  String get walletSnapshotUnavailable =>
      'Impossibile leggere il portafoglio in questo momento. Si aggiornerà automaticamente.';

  @override
  String get walletBalanceStale =>
      'Impossibile aggiornare — viene mostrato l\'ultimo saldo noto.';

  @override
  String get walletSyncStartFailed =>
      'Impossibile avviare la sincronizzazione. Verranno effettuati nuovi tentativi automaticamente.';

  @override
  String get walletSyncRetry => 'Riprova';

  @override
  String get walletSyncTryNow => 'Prova ora';

  @override
  String get walletSyncIdle => 'Sincronizzazione non ancora avviata';

  @override
  String get walletSyncIdleDetail =>
      'La sincronizzazione si avvia automaticamente.';

  @override
  String get walletSyncDisabled => 'Sincronizzazione disattivata';

  @override
  String get walletSyncDisabledDetail =>
      'Attiva la sincronizzazione nelle impostazioni di questa app per aggiornare il saldo.';

  @override
  String get walletSyncExplainDisabled =>
      'La sincronizzazione è disattivata nelle impostazioni di questa app. I fondi sono al sicuro. Il saldo e l\'attività mostrano l\'ultimo stato sincronizzato e non verranno aggiornati finché la sincronizzazione non sarà riattivata.';

  @override
  String get walletParkedSyncPausedNote =>
      'Il portafoglio non è in sincronizzazione, quindi questi pagamenti non verranno inviati da soli. Usare Invia ora per inviarne uno manualmente.';

  @override
  String get walletSyncPausedMoneyNote =>
      'In pausa finché il portafoglio non torna a sincronizzarsi.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Connessione in corso…';

  @override
  String get walletSyncStartingDetail =>
      'Connessione alla rete Zcash e preparazione della scansione.';

  @override
  String get walletSyncConnecting => 'Connessione in corso…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Connessione in corso… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Scansione $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'Scansione in corso…';

  @override
  String get walletSyncSpendableReady =>
      'I fondi sono pronti per essere spesi.';

  @override
  String get walletSyncCatchingUp =>
      'Recupero dei dati della rete in corso — una sincronizzazione iniziale approfondita può richiedere tempo. È possibile continuare a usare l\'app nel frattempo';

  @override
  String walletSyncScanRemaining(String count) {
    return '$count blocchi rimanenti';
  }

  @override
  String get walletSyncUpToDate => 'Aggiornato';

  @override
  String get walletSyncOffline => 'Offline';

  @override
  String get walletSyncOfflineDetail =>
      'Gli invii in coda restano salvati in Salvati e in sospeso.';

  @override
  String get walletSyncUnknown => 'Sincronizzazione in corso…';

  @override
  String get walletSyncStalled => 'Sincronizzazione in pausa';

  @override
  String get walletStallEndpoint =>
      'Impossibile raggiungere la rete Zcash in questo momento. Verranno effettuati nuovi tentativi automaticamente — verificare la connessione a Internet, oppure il server potrebbe essere temporaneamente non disponibile.';

  @override
  String get walletStallTor =>
      'Il percorso privato della tua app non è disponibile, quindi il portafoglio non si connette. Controlla le impostazioni di rete della tua app o disattiva il percorso privato. La sincronizzazione riprenderà appena il percorso tornerà disponibile.';

  @override
  String get walletStallStorage =>
      'La memoria del dispositivo è piena. Liberare spazio per riprendere la sincronizzazione.';

  @override
  String get walletStallReorg =>
      'La catena si è riorganizzata; i blocchi recenti sono in fase di ricontrollo.';

  @override
  String get walletStallInternal =>
      'Un problema locale ha interrotto la sincronizzazione. Se il problema persiste, ripristinare dalla frase di recupero.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Questo server ha inviato dati che non possono essere corretti, quindi la sincronizzazione si è fermata. Non è un problema di connessione: passa a un altro server. Se ogni server viene rifiutato, ripeti la scansione della cronologia: il portafoglio potrebbe conservare un record errato di un server precedente.';

  @override
  String get walletStallBirthdayInFuture =>
      'Questo portafoglio è impostato per partire da un blocco che questo server non ha ancora raggiunto. Controlla il blocco iniziale impostato per questo portafoglio oppure prova un altro server.';

  @override
  String get walletStallStorageUnavailable =>
      'Sincronizzazione in pausa su questo dispositivo. Nuovo tentativo in corso.';

  @override
  String get walletStallUnknown =>
      'La sincronizzazione si è interrotta per un motivo sconosciuto.';

  @override
  String get walletSyncBadgeHint => 'Mostra i dettagli della sincronizzazione';

  @override
  String get walletSyncSheetClose => 'Chiudi';

  @override
  String get walletSyncSheetProgress => 'Avanzamento';

  @override
  String get walletSyncSheetBlocksLeft => 'Blocchi rimanenti';

  @override
  String get walletSyncSheetSyncedTo => 'Sincronizzato fino al blocco';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Indietro di almeno $blocks blocchi',
      one: 'Indietro di almeno 1 blocco',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'La sincronizzazione non è ancora iniziata — si avvia automaticamente. Non è richiesta alcuna azione.';

  @override
  String get walletSyncExplainStartFailed =>
      'La sincronizzazione non è riuscita ad avviarsi. I fondi sono al sicuro — il portafoglio semplicemente non sta controllando nuove attività. Riprovi qui sotto, oppure riapra l\'app.';

  @override
  String get walletSyncExplainStarting =>
      'Il portafoglio sta contattando la rete Zcash e preparando la scansione. Di solito richiede pochi secondi.';

  @override
  String get walletSyncExplainConnecting =>
      'Connessione alla rete Zcash in corso.';

  @override
  String get walletSyncExplainScanning =>
      'Il portafoglio sta controllando i blocchi della blockchain alla ricerca dei fondi. Saldo e attività si aggiornano man mano che vengono trovate nuove transazioni — è possibile continuare a usare l\'app nel frattempo.';

  @override
  String get walletSyncExplainUpToDate =>
      'Completamente sincronizzato con la rete Zcash. Saldo e attività sono aggiornati.';

  @override
  String get walletSyncExplainStalled =>
      'La sincronizzazione ha riscontrato un problema ed è in pausa. Verranno effettuati nuovi tentativi automaticamente.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Impossibile raggiungere la rete Zcash — è normale se si è offline, oppure il server potrebbe essere temporaneamente non disponibile. I fondi sono al sicuro: il saldo mostra l\'ultimo stato sincronizzato e gli invii in coda restano salvati in Salvati e in sospeso. La connessione riprova automaticamente.';

  @override
  String get walletSyncExplainOffline =>
      'Nessuna connessione di rete. I fondi sono al sicuro — il saldo mostra l\'ultimo stato sincronizzato e gli invii in coda restano salvati in Salvati e in sospeso.';

  @override
  String get walletSyncExplainUnknown =>
      'Il portafoglio è in fase di sincronizzazione. Saldo e attività si aggiornano con l\'avanzamento.';

  @override
  String get walletTorOff => 'Tor disattivato';

  @override
  String get walletTorBootstrapping => 'Avvio del percorso privato…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return 'Avvio di $transport…';
  }

  @override
  String get walletTorActive => 'Tor attivo';

  @override
  String get walletTorActiveUnverified => 'Tor attivo (runtime non verificato)';

  @override
  String get walletTorActiveUnattested =>
      'Percorso privato in uso (riservatezza non verificata)';

  @override
  String get walletTorFellBack =>
      'Tor non disponibile — connessione diretta in uso';

  @override
  String get walletTorUnavailable =>
      'Percorso privato non disponibile — non connesso';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport non disponibile — non connesso';
  }

  @override
  String get walletTorUnanswered =>
      'Percorso privato connesso — non torna nulla';

  @override
  String get walletTorUnansweredUnattested =>
      'Percorso privato connesso — non torna nulla (riservatezza non verificata)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport connesso — non torna nulla';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Non privato (connessione diretta della tua app) — non torna nulla';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Connesso tramite $transport — non torna nulla; il proxy può collegare le connessioni';
  }

  @override
  String get walletTorUnknown =>
      'Stato di Tor sconosciuto — considerare la connessione non protetta';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Saldo (al blocco $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Saldo · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Saldo (al blocco $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Connessione';

  @override
  String get walletSyncSheetServer => 'Server';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Server, $host, apre il selettore del server';
  }

  @override
  String get walletSyncServerSheetTitle => 'Server di sincronizzazione';

  @override
  String get walletSyncServerInUse => 'In uso';

  @override
  String get walletSyncServerAppDefault => 'Predefinito dell\'app';

  @override
  String get walletSyncServerCustom => 'Server personalizzato…';

  @override
  String get walletSyncServerCustomHint => 'https://host:porta';

  @override
  String get walletSyncServerCheck => 'Verifica server';

  @override
  String get walletSyncServerChecking => 'Verifica in corso…';

  @override
  String get walletSyncServerUse => 'Usa questo server';

  @override
  String get walletSyncServerSwitching => 'Cambio in corso…';

  @override
  String get walletSyncServerContinue => 'Continua';

  @override
  String get walletSyncServerCancel => 'Annulla';

  @override
  String get walletSyncServerTrustTitle => 'Fidarsi di questo server?';

  @override
  String get walletSyncServerTrustNotice =>
      'Ti stai fidando di questo server per il saldo, la cronologia e l\'inoltro dei tuoi pagamenti. Vedrà il tuo indirizzo IP a meno che Tor sia attivo, all\'incirca quando è stato creato il portafoglio, gli indirizzi pubblici che il portafoglio controlla, le transazioni che consulta e le transazioni che invii.';

  @override
  String get walletSyncServerKeyLabel => 'Chiave di accesso (facoltativa)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Intestazione della chiave';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Inserisci l\'intestazione che il tuo server si aspetta';

  @override
  String get walletSyncServerKeyInvalid =>
      'Questa chiave o intestazione non può essere usata';

  @override
  String get walletSyncServerKeySaved => 'Chiave salvata';

  @override
  String get walletSyncServerKeyShow => 'Mostra';

  @override
  String get walletSyncServerKeyHide => 'Nascondi';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'La tua chiave ti identifica presso questo server. Può collegare i tuoi pagamenti al tuo portafoglio, anche tramite Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Il cambio riavvia la sincronizzazione in corso. Saldo e cronologia restano. I fondi possono risultare in arrivo finché la scansione del nuovo server non si allinea.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Il cambio riconnette al nuovo server. Saldo e cronologia restano.';

  @override
  String get walletSyncServerUnreachable =>
      'Impossibile raggiungere questo server. Controlla l\'indirizzo — e se è giusto, o questo server non risponde oppure la tua app non riesce a raggiungerlo in questo momento. Riprova o scegli un altro server.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Impossibile raggiungere questo server. Il portafoglio non può sapere se questo server non risponde o se la tua app non riesce a raggiungerlo in questo momento. Scegli un altro server o riprova più tardi.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Questo server è su un\'altra rete Zcash.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Non sembra un indirizzo di server. Usa https://host:porta.';

  @override
  String get walletSyncServerNotOffered =>
      'Questo server non è offerto da questa app.';

  @override
  String get walletSyncServerBusy =>
      'Il portafoglio è occupato al momento. Riprova tra un istante.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'Il server scelto non è più offerto da questa app. In uso $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'La scelta del server memorizzata non è leggibile. In uso $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Cambio non riuscito: ancora in uso $host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Il traffico del portafoglio si connette direttamente al server. Il server può vedere l\'indirizzo IP.';

  @override
  String get walletTransportExplainTor =>
      'Il traffico del portafoglio viene instradato attraverso la rete Tor, che nasconde l\'indirizzo IP al server.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Il percorso privato della tua app si sta avviando. Il traffico del portafoglio attende prima di connettersi.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport si sta avviando. Il traffico del portafoglio attende prima di connettersi.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Non è stato possibile raggiungere Tor, quindi il traffico è passato a una connessione diretta. Il server può vedere l\'indirizzo IP.';

  @override
  String get walletTransportExplainUnavailable =>
      'Il percorso privato della tua app non è disponibile, quindi il portafoglio non si connette. Disattiva il percorso privato o controlla le impostazioni di rete della tua app.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport non è disponibile, quindi il portafoglio non si connette. Disattivalo o controlla le impostazioni di rete della tua app.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'Il percorso privato ha accettato la connessione, ma da un minuto non torna nulla. Può essere il percorso o il server del portafoglio — il portafoglio non può saperlo. Continua a riprovare; se non si risolve, prova un altro server o controlla le impostazioni di rete della tua app.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport ha accettato la connessione, ma da un minuto non torna nulla. Può essere il percorso o il server del portafoglio — il portafoglio non può saperlo. Continua a riprovare; se non si risolve, prova un altro server o controlla le impostazioni di rete della tua app.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Il traffico del portafoglio si connette direttamente al server. Il server può vedere l\'indirizzo IP. La connessione è stata accettata, ma da un minuto non torna nulla. Può essere il percorso o il server del portafoglio — il portafoglio non può saperlo. Continua a riprovare; se non si risolve, prova un altro server o controlla le impostazioni di rete della tua app.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'La riservatezza di questa connessione non può essere verificata — considerarla non privata. La connessione è stata accettata, ma da un minuto non torna nulla. Può essere il percorso o il server del portafoglio — il portafoglio non può saperlo. Continua a riprovare; se non si risolve, prova un altro server o controlla le impostazioni di rete della tua app.';

  @override
  String get walletTransportExplainUnverified =>
      'La riservatezza di questa connessione non può essere verificata — considerarla non privata.';

  @override
  String get walletTransportExplainHostProxy =>
      'Il traffico del portafoglio viene instradato attraverso il trasporto privato di questa app, che nasconde l\'indirizzo IP al server.';

  @override
  String get walletOnboardingWelcomeTitle => 'Configura il portafoglio';

  @override
  String get walletOnboardingWelcomeBody =>
      'Crei un nuovo portafoglio per ricevere e conservare ZEC. Verrà generata una frase di recupero, con le indicazioni per eseguirne il backup prima che sia possibile ricevere fondi — così nulla è mai a rischio senza un backup.';

  @override
  String get walletCreateButton => 'Crea un nuovo portafoglio';

  @override
  String get walletRestoreButton => 'Ripristina da una frase di recupero';

  @override
  String get walletWatchOnlyButton =>
      'Osserva un portafoglio (sola visualizzazione)';

  @override
  String get walletWatchOnlyTitle => 'Osserva un portafoglio';

  @override
  String get walletWatchOnlyBody =>
      'Incollare una chiave di visualizzazione per osservare un portafoglio senza le sue chiavi di spesa. Sarà possibile vedere il saldo e la cronologia, ma non sarà possibile inviare fondi. Scegliere la data di inizio approssimativa del portafoglio, così sapremo fino a che punto risalire.';

  @override
  String get walletWatchOnlyKeyLabel => 'Chiave di visualizzazione';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip =>
      'Scansiona il codice QR della chiave di visualizzazione';

  @override
  String get walletWatchOnlyScanTitle => 'Scansiona chiave di visualizzazione';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Inquadrare il codice QR della chiave di visualizzazione con la fotocamera.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Fotocamera non disponibile. Incollare invece la chiave manualmente.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Incollare invece';

  @override
  String get walletWatchOnlyScanHint =>
      'Oppure toccare il pulsante di scansione per leggere un codice QR della chiave di visualizzazione.';

  @override
  String get walletWatchOnlyScanFilled =>
      'Chiave di visualizzazione scansionata.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Data di inizio del portafoglio';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Scansione a partire dal $date — i fondi ricevuti prima di tale data non compariranno. Portafoglio più vecchio? Scelga una data precedente.';
  }

  @override
  String get walletWatchOnlyBirthdayPick =>
      'Scegliere la data di inizio del portafoglio';

  @override
  String get walletWatchOnlyBirthdayChange => 'Cambia data';

  @override
  String get walletWatchOnlySubmit => 'Osserva questo portafoglio';

  @override
  String get walletWatchOnlyBack => 'Indietro';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Questo non sembra una chiave di visualizzazione valida. Verificarla e riprovare.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Questa chiave di visualizzazione appartiene a una rete diversa. Non può essere utilizzata qui.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'Su questo dispositivo esiste già un portafoglio. Tornare indietro per aprirlo.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Questa data di inizio è troppo recente. Scegliere una data precedente.';

  @override
  String get walletRestoreTitle => 'Ripristina il portafoglio';

  @override
  String get walletRestoreBody =>
      'Inserire la frase di recupero per ripristinare il portafoglio — digitare o incollare le parole in ordine, separate da spazi. Solo frasi standard: se il portafoglio utilizzava una passphrase aggiuntiva (una \"25ª parola\"), questa app non può ancora ripristinarlo — verrebbe mostrato un portafoglio vuoto, non un errore.';

  @override
  String get walletRestorePhraseHint => 'parola uno  parola due  parola tre  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count parole',
      one: '1 parola',
      zero: 'Nessuna parola',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'le frasi di recupero hanno 12, 15, 18, 21 o 24 parole';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count parole non sono parole di recupero — corregga quelle evidenziate',
      one:
          '1 parola non è una parola di recupero — corregga quella evidenziata',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'parola $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'parola $index: non è una parola di recupero';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Rimuovi parola $index';
  }

  @override
  String get walletRestoreSubmit => 'Ripristina portafoglio';

  @override
  String get walletRestoreBack => 'Indietro';

  @override
  String get walletRestoreBirthdayTitle =>
      'Da quanto tempo indietro scansionare';

  @override
  String get walletRestoreBirthdayNone =>
      'Verrà scansionata l\'intera cronologia — più lento, ma non si perde nulla.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Scansione a partire dal $date — i fondi ricevuti prima di tale data non compariranno. Portafoglio più vecchio? Scelga una data precedente, oppure Scansiona tutta la cronologia.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Scegli una data';

  @override
  String get walletRestoreBirthdayChange => 'Cambia data';

  @override
  String get walletRestoreBirthdayClear => 'Scansiona tutta la cronologia';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'La parola $index non è una parola di recupero. Verificare la frase per eventuali errori di battitura, poi riprovare.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Questa frase di recupero non è valida. Verificare le parole e il loro ordine, poi riprovare.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Questa frase non corrisponde al portafoglio presente su questo dispositivo. Verificarla attentamente e riprovare.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'Su questo dispositivo esiste già un portafoglio. Tornare indietro per aprirlo.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Questa data è troppo recente. Scegliere una data precedente, oppure scansionare tutto.';

  @override
  String get walletGeneratingLabel => 'Creazione del portafoglio…';

  @override
  String get walletOpeningLabel => 'Apertura del portafoglio…';

  @override
  String get walletBackupTitle => 'Esegui il backup della frase di recupero';

  @override
  String get walletBackupBody =>
      'Queste parole sono l\'UNICO modo per recuperare il portafoglio e i fondi. Le trascriva in ordine e le conservi in un luogo sicuro e privato. Non le condivida né le salvi online: chiunque le conosca può impossessarsi dei fondi.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Gli screenshot sono disattivati in questa schermata.';

  @override
  String get walletBackupSecureNoteOther =>
      'Si assicuri che nessuno possa vedere lo schermo.';

  @override
  String get walletBackupReveal => 'Mostra la frase di recupero';

  @override
  String get walletBackupRevealing => 'Preparazione della frase di recupero…';

  @override
  String get walletBackupRevealFailed =>
      'Impossibile mostrare la frase di recupero in questo momento. Si assicuri che il dispositivo sia sbloccato, quindi riprovi.';

  @override
  String get walletBackupRetryReveal => 'Riprova';

  @override
  String get walletBackupReauthFailed =>
      'Impossibile verificare l\'identità. Riprovare.';

  @override
  String get walletBackupConfirmCheckbox =>
      'Ho trascritto la mia frase di recupero e l\'ho conservata in modo sicuro.';

  @override
  String get walletBackupContinue => 'Continua';

  @override
  String get walletBackupSaveFailed =>
      'Impossibile salvare la conferma. Riprovare.';

  @override
  String get walletBackupStartOver => 'Ricomincia';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Ricominciare senza questo portafoglio?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Questa operazione elimina questo portafoglio dal dispositivo e la riporta all\'inizio. Nulla può essere depositato tramite questa app prima che la configurazione sia completata.\n\nSe questo portafoglio ha mai contenuto fondi — o è stato ripristinato da una frase di recupero — solo quella frase può ripristinarlo.';

  @override
  String get walletBackupStartOverConfirm => 'Elimina e ricomincia';

  @override
  String get walletBackupStartOverKeep => 'Mantieni questo portafoglio';

  @override
  String get walletBackupSectionTitle => 'Frase di recupero';

  @override
  String get walletBackupTileTitle =>
      'Esegui il backup della frase di recupero';

  @override
  String get walletBackupTileSubtitle =>
      'Mostra le parole che consentono di recuperare il portafoglio e i fondi.';

  @override
  String get walletBackupScreenTitle => 'Frase di recupero';

  @override
  String get walletBackupDone => 'Fatto';

  @override
  String get walletBackupManagedTitle => 'Nessuna frase di recupero separata';

  @override
  String get walletBackupManagedBody =>
      'Questo portafoglio è stato configurato utilizzando l\'account dell\'app che lo ha installato, quindi non ha una propria frase di recupero. I fondi vengono recuperati insieme a quell\'account: utilizzi il backup di quell\'account per tenerli al sicuro.';

  @override
  String get walletExportViewingKeyTitle =>
      'Esporta la chiave di visualizzazione';

  @override
  String get walletExportViewingKeyTileTitle =>
      'Esporta la chiave di visualizzazione';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Condivida una copia di sola visualizzazione del suo portafoglio: può vedere la sua cronologia, ma non può spendere i suoi fondi.';

  @override
  String get walletExportViewingKeyWarning =>
      'Questa chiave permette a chiunque la possieda di vedere tutto ciò che questo portafoglio ha ricevuto e inviato finora — e tutto ciò che riceverà e invierà in futuro. Non consente di spendere i suoi fondi né di recuperare il suo portafoglio. La condivida solo con una persona di cui si fida e a cui desidera mostrare l\'intera cronologia, ad esempio un commercialista o un suo secondo dispositivo. L\'unico modo per revocare la condivisione in seguito è trasferire i fondi in un nuovo portafoglio.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Questa chiave permette a chiunque la possieda di vedere tutto ciò che questo portafoglio ha ricevuto e inviato finora — e tutto ciò che riceverà e invierà in futuro. Non consente di spendere i suoi fondi né di recuperare il suo portafoglio. La condivida solo con una persona di cui si fida e a cui desidera mostrare l\'intera cronologia, ad esempio un commercialista o un suo secondo dispositivo. Una volta condivisa, la condivisione non può essere revocata.';

  @override
  String get walletExportViewingKeyReveal =>
      'Mostra la chiave di visualizzazione';

  @override
  String get walletExportViewingKeyRetry => 'Riprova';

  @override
  String get walletExportViewingKeyRevealing =>
      'Preparazione della chiave di visualizzazione…';

  @override
  String get walletExportViewingKeyFailed =>
      'Impossibile mostrare la chiave di visualizzazione in questo momento. Riprovi tra poco.';

  @override
  String get walletExportViewingKeyQrLabel =>
      'Codice QR della chiave di visualizzazione';

  @override
  String get walletExportViewingKeyCopy => 'Copia chiave di visualizzazione';

  @override
  String get walletExportViewingKeyCopied =>
      'Chiave di visualizzazione copiata';

  @override
  String get walletExportViewingKeyDone => 'Fatto';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Gli screenshot sono disattivati in questa schermata.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Si assicuri che nessuno possa vedere lo schermo.';

  @override
  String get walletWatchOnlySectionTitle =>
      'Informazioni su questo portafoglio di sola visualizzazione';

  @override
  String get walletWatchOnlyAboutBody =>
      'Questo è un portafoglio di sola visualizzazione. È stato configurato a partire da una chiave di visualizzazione: può quindi vedere il saldo e la cronologia, ma non contiene alcuna chiave di spesa — qui non c\'è alcun backup da eseguire, e non può inviare fondi.';

  @override
  String get walletWatchOnlyBadge => 'Sola visualizzazione';

  @override
  String get walletOnboardingFailedTitle =>
      'Impossibile completare la configurazione del portafoglio';

  @override
  String get walletOnboardingRetry => 'Riprova';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'La memoria sicura del telefono non risponde. Sblocchi il dispositivo e riprovi. Se il problema persiste, riavvii il telefono.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Questo portafoglio è aperto in un\'altra finestra o app, oppure sta ancora completando un\'operazione precedente. Chiuda le altre finestre che lo utilizzano — oppure attenda un momento — quindi riprovi.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'La chiave sicura di questo portafoglio non è più disponibile, quindi non può essere aperto su questo dispositivo. I fondi sono al sicuro — ripristini dalla frase di recupero per recuperarli.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Ripristina da frase di recupero';

  @override
  String get walletOnboardingRecoverConfirmTitle =>
      'Ripristinare questo portafoglio?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Si assicuri di avere la frase di recupero prima di continuare — sarà necessaria nella schermata successiva per recuperare i fondi. I fondi sono al sicuro sulla blockchain e sono controllati da quella frase. Questa operazione rimuove da questo dispositivo i dati del portafoglio non leggibili, in modo che possa essere ricostruito.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Annulla';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Non c\'è spazio libero sufficiente per configurare il portafoglio. Liberi dello spazio e riprovi.';

  @override
  String get walletOnboardingFailedNoVault =>
      'Questo dispositivo non dispone di un archivio sicuro per le chiavi, quindi il portafoglio non può proteggere qui la frase di recupero.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Impossibile raggiungere la rete durante la configurazione. Verifichi la connessione e riprovi.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'La configurazione del portafoglio non è stata completata. Riprovi per completarla — nulla è andato perso.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Si è verificato un problema durante la configurazione del portafoglio. Riprovare.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'La configurazione del portafoglio di questa app non è corretta, quindi il portafoglio non può avviarsi. Riprovare non risolverà il problema — segnali il problema allo sviluppatore dell\'app. I fondi non sono interessati.';

  @override
  String get walletSendButton => 'Invia';

  @override
  String get walletSendSyncNotRunning =>
      'La sincronizzazione non è in corso — il saldo spendibile non potrà aggiornarsi';

  @override
  String get walletSendWaitingForFunds =>
      'Sincronizzazione ancora in corso — invio disponibile quando è presente un saldo spendibile';

  @override
  String get walletSendNoSpendableYet => 'Nessun saldo disponibile per ora';

  @override
  String get walletSendSyncUnavailable =>
      'Invio disponibile alla ripresa della sincronizzazione';

  @override
  String get walletSendTitle => 'Invia';

  @override
  String get walletSendUnavailable =>
      'Il portafoglio non è pronto in questo momento. Torni indietro e riprovi.';

  @override
  String get walletSendWatchOnly =>
      'Questo è un portafoglio di sola visualizzazione. Può mostrare il saldo e ricevere pagamenti, ma non contiene chiavi di spesa — quindi non può inviare.';

  @override
  String get walletSendExpiredTitle =>
      'Questa richiesta di pagamento è scaduta';

  @override
  String get walletSendExpiredBody =>
      'La schermata di invio ha impiegato più di cinque secondi ad aprirsi, quindi l\'app è stata informata che non è stato inviato nulla. Quella risposta è definitiva: questa richiesta non può essere pagata da qui. Per pagare, ricomincia dall\'app.';

  @override
  String get walletSendFaultWatchOnly =>
      'Questo è un portafoglio di sola visualizzazione — non contiene chiavi di spesa, quindi non può inviare.';

  @override
  String walletSendAvailable(String amount) {
    return 'Disponibile per l\'invio: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Disponibile per l\'invio: $amount ZEC — il saldo si sta ancora aggiornando';
  }

  @override
  String get walletSendRecipientLabel => 'Indirizzo del destinatario';

  @override
  String get walletSendRecipientHint => 'Indirizzo Zcash (inizia con u, z o t)';

  @override
  String get walletSendRecipientLocked =>
      'Il destinatario non può essere modificato qui';

  @override
  String get walletSendAmountLabel => 'Importo (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Memo (facoltativo)';

  @override
  String get walletSendMemoHint =>
      'Recapitato solo a destinatari schermati (privati)';

  @override
  String get walletSendMemoTransparentDisabled =>
      'I memo richiedono un destinatario schermato. Questo indirizzo pubblico non può riceverne uno.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Questo pagamento porta già un riferimento dell\'app, quindi non può contenere anche una nota scritta.';

  @override
  String get walletSendMachineMemoTitle =>
      'L\'app sta allegando un riferimento';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Dice che serve per: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Resta con la transazione e non potrà essere rimosso in seguito. Il wallet non può verificarne il contenuto.';

  @override
  String get walletSendRecipientShielded => 'Schermato · privato';

  @override
  String get walletSendRecipientTransparent => 'Pubblico';

  @override
  String get walletSendRecipientInvalid =>
      'Questo non sembra un indirizzo Zcash valido.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Questo indirizzo appartiene a una rete Zcash diversa.';

  @override
  String get walletSendReviewButton => 'Verifica il pagamento';

  @override
  String get walletSendQueueButton => 'Metti in coda per inviare più tardi';

  @override
  String get walletSendQueueHint =>
      'Un pagamento in coda resta in Salvati e in sospeso, dove può inviarlo o annullarlo. La commissione di rete viene calcolata al momento dell\'invio.';

  @override
  String get walletSendPreparing => 'Preparazione del pagamento…';

  @override
  String get walletSendSubmitting => 'Invio in corso…';

  @override
  String get walletSendQueuing => 'Accodamento in corso…';

  @override
  String get walletSendReviewTitle => 'Conferma pagamento';

  @override
  String get walletSendTotalLabel => 'Totale';

  @override
  String get walletSendFeeLabel => 'Commissione di rete';

  @override
  String get walletSendChangeLabel => 'Resto restituito';

  @override
  String get walletSendDeshieldTitle => 'Questo pagamento non è privato';

  @override
  String get walletSendDeshieldBody =>
      'Viene inviato a un indirizzo pubblico, quindi l\'importo e il destinatario saranno pubblicamente visibili sulla blockchain di Zcash.';

  @override
  String get walletSendPublicAckLabel =>
      'Capisco che questo pagamento sarà pubblico.';

  @override
  String get walletSendConfirmButton => 'Invia ora';

  @override
  String get walletSendBackButton => 'Indietro';

  @override
  String get walletSendSelfSendNote =>
      'L\'invio è verso il proprio portafoglio. La commissione di rete si applica comunque.';

  @override
  String get walletSendLargeConfirmTitle => 'Inviare un importo elevato?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Si tratta quasi dell\'intero saldo. Un pagamento inviato non può essere annullato.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Si tratta di un pagamento di importo elevato. Un pagamento inviato non può essere annullato.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Si tratta di un pagamento di importo elevato — quasi l\'intero saldo. Un pagamento inviato non può essere annullato.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Invia $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Torna indietro';

  @override
  String get walletSendSentTitle => 'Pagamento inviato';

  @override
  String get walletSendSentBody => 'Il pagamento è stato trasmesso alla rete.';

  @override
  String get walletSendSavedTitle => 'Salvato — l\'invio sarà completato';

  @override
  String get walletSendSavedBody =>
      'Non è stato possibile inviare il pagamento in questo momento: è stato salvato e il portafoglio lo invierà a una sincronizzazione successiva. Nulla è andato perso.';

  @override
  String get walletSendKeptTitle => 'Salvata';

  @override
  String get walletSendKeptBody =>
      'Il portafoglio ha conservato questa transazione, ma non si è impegnato a inviarla da solo. Controlla Attività per vedere a che punto è.';

  @override
  String get walletSendPartialBody =>
      'Parte del pagamento è stata inviata; il portafoglio completerà il resto a una sincronizzazione successiva. Nulla è andato perso.';

  @override
  String get walletSendInMotionTitle => 'Pagamento in corso';

  @override
  String get walletSendInMotionBody =>
      'Il pagamento è stato avviato e sta transitando attraverso un indirizzo monouso sotto il controllo del portafoglio. Non lo invii di nuovo. Se non si completa, potrà recuperare i fondi dalla schermata del portafoglio.';

  @override
  String get walletSendAlreadyTitle => 'Già inviato';

  @override
  String get walletSendAlreadyBody =>
      'Questo pagamento è già stato inviato — non verrà inviato due volte.';

  @override
  String get walletSendFailedTitle => 'Impossibile completare il pagamento';

  @override
  String get walletSendFailedBody =>
      'Si è verificato un problema nel completare questo pagamento e non è stato inviato nulla. Potrà riprovare.';

  @override
  String get walletSendTryAgain => 'Riprova';

  @override
  String get walletSendDone => 'Fatto';

  @override
  String get walletSendAnother => 'Invia un altro pagamento';

  @override
  String get walletSendQueuedTitle => 'In coda per l\'invio';

  @override
  String get walletSendQueuedBody =>
      'Questo pagamento è salvato. È disponibile in Salvati e in sospeso, da dove è possibile inviarlo ora o annullarlo.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Saldo disponibile insufficiente — sono disponibili $available ZEC e ne servono $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'La rete Zcash è stata aggiornata e questa app necessita di un aggiornamento prima di poter inviare. I tuoi fondi sono al sicuro.';

  @override
  String get walletSyncUpToDateLimited =>
      'Aggiornato fin dove questa versione può leggere';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'La rete Zcash è stata aggiornata. Questa versione ha scansionato tutto ciò che può leggere, ma i blocchi più recenti potrebbero contenere fondi che non può ancora mostrare, e i memo dei pagamenti recenti non sono disponibili. Aggiorna l\'app per vedere tutto.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Aggiornato, ma questo server non serve tutti i pool';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Questo server rifiuta, trattiene o riporta in modo errato uno dei pool schermati di Zcash. I fondi ricevuti in quel pool non possono essere spesi tramite questo server e il saldo mostrato è un minimo. Passa a un altro server per usarli: non è un problema di connessione.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: questo server rifiuta di servirlo';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: questo server ne trattiene una parte';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: questo server lo segnala in modo errato';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: non è noto se questo server lo serva';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Aggiornato con questo server, ma il server è indietro rispetto alla rete';

  @override
  String get walletSyncExplainEndpointBehind =>
      'La catena di questo server si ferma a un blocco che la rete aveva già superato prima che questa versione dell\'app fosse compilata, quindi il saldo è aggiornato solo fino a quel blocco. I nuovi pagamenti ricevuti potrebbero non essere ancora visibili e un pagamento inviato da qui potrebbe non andare a buon fine. Passa a un altro server per recuperare: non è un problema di connessione.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'In attesa di un aggiornamento dell\'app — i tuoi fondi sono al sicuro e non è stato inviato nulla.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'In attesa di un server che comunichi la versione della rete — cambia server. I tuoi fondi sono al sicuro e non è stato inviato nulla.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'In attesa di un server che comunichi la versione della rete. Se data e ora di questo dispositivo sono sbagliate, correggile prima — poi cambia server. I tuoi fondi sono al sicuro e non è stato inviato nulla.';

  @override
  String get walletSyncUnverified =>
      'Aggiornato, ma questo server non comunica la versione della rete';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'L\'invio funziona ancora per circa $hours ore — poi cambia server.',
      one: 'L\'invio funziona ancora per circa 1 ora — poi cambia server.',
      zero: 'L\'invio funziona ancora per meno di un\'ora — poi cambia server.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'L\'invio funziona ancora per circa $blocks blocchi — poi cambia server.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Questo server non comunica la versione della rete da $blocks blocchi, quindi questa app non può confermare che inviare sia sicuro. Passa a un altro server.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Questo server non comunica la versione della rete da un giorno, quindi questa app non può confermare che inviare sia sicuro. Se data e ora di questo dispositivo sono sbagliate, correggile prima — poi passa a un server che comunica la versione della rete.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Questo server non ha mai comunicato la versione della rete, quindi questa app non può confermare che inviare sia sicuro. Passa a un altro server.';

  @override
  String get walletSyncExplainUnverified =>
      'Questo server non dice su quale versione della rete Zcash si trova, quindi questa app non può confermare che un pagamento da lei firmato verrà accettato. Il tuo saldo è aggiornato. Passa a un altro server — non è un problema di connessione.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Questo server non dice su quale versione della rete Zcash si trova, quindi questa app non può confermare che un pagamento da lei firmato verrà accettato. Ha inoltre continuato a servire blocchi che questo portafoglio ha poi dovuto annullare, quindi il tuo saldo potrebbe non essere aggiornato. Passa a un altro server — non è un problema di connessione.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Questo server continua anche a servire blocchi che questo portafoglio deve poi annullare — cambia server.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Il saldo si sta ancora aggiornando — potrebbe diventare disponibile altro man mano che il portafoglio si sincronizza.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC sono ancora in arrivo e saranno disponibili una volta che il portafoglio si sarà messo in pari.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Inserire un importo da inviare.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Inserire l\'importo come numero, ad esempio 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'ZEC ha al massimo 8 cifre decimali.';

  @override
  String get walletSendFaultAmountNotPositive =>
      'Inserire un importo maggiore di zero.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Questo importo supera l\'offerta totale di ZEC.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Al momento questa app limita gli invii a $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Questo non sembra un indirizzo Zcash valido per questa rete. Verificarlo e riprovare.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Questo destinatario non può ricevere un memo. Rimuovere il memo, oppure inviare a un indirizzo schermato (privato).';

  @override
  String get walletSendFaultMemoTooLong =>
      'Il memo è troppo lungo. Accorciarlo e riprovare.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Questo memo non può essere inviato. Rimuoverlo e riprovare.';

  @override
  String get walletSendFaultMemoConflict =>
      'Impossibile inviare questo pagamento: l\'app vi ha allegato due note. Non è stato inviato nulla.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Questo indirizzo appartiene a una rete diversa.';

  @override
  String get walletSendFaultUriInvalid =>
      'Impossibile creare questo pagamento. Verificare l\'indirizzo e l\'importo.';

  @override
  String get walletSendFaultNotSynced =>
      'Il portafoglio non è ancora sincronizzato a sufficienza. Attendere che la sincronizzazione avanzi, oppure mettere in coda per inviare più tardi.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Il portafoglio non è ancora sincronizzato a sufficienza. Attendere che la sincronizzazione avanzi.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Il portafoglio non è ancora sincronizzato a sufficienza, e la sincronizzazione non è in corso in questo momento. Verificare lo stato della sincronizzazione nella schermata del portafoglio.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Gli importi sono scaduti durante la verifica. Verificare di nuovo il pagamento.';

  @override
  String get walletSendFaultQueueFull =>
      'Troppi invii sono in attesa di partire. Attendere che vengano inviati, quindi riprovare.';

  @override
  String get walletSendFaultWalletBusy =>
      'Il portafoglio è occupato in questo momento. Riprovare tra poco.';

  @override
  String get walletSendFaultStorageFull =>
      'Non c\'è spazio libero sufficiente per completare questo invio. Liberare dello spazio e riprovare.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Troppi indirizzi monouso sono in uso in questo momento. Alcuni potrebbero liberarsi quando gli invii verranno confermati, ma la situazione potrebbe non risolversi da sola. I fondi sono al sicuro.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Impossibile preparare questo pagamento. Verificare i dettagli e riprovare.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Impossibile preparare questo pagamento in questo momento. Riprova tra un istante.';

  @override
  String get walletSwapButton => 'Scambia';

  @override
  String get walletSwapTitle => 'Scambia ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'Il portafoglio non è pronto in questo momento. Tornare indietro e riprovare.';

  @override
  String get walletSwapUnavailableOff =>
      'Lo scambio non è disponibile in questo momento.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Questo è un portafoglio di sola visualizzazione — non può scambiare.';

  @override
  String get walletSwapDone => 'Fatto';

  @override
  String get walletSwapBackToWallet => 'Torna al portafoglio';

  @override
  String walletSwapAvailable(String amount) {
    return 'Disponibile per lo scambio: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Disponibile per lo scambio: $amount ZEC — il saldo si sta ancora aggiornando';
  }

  @override
  String get walletSwapAssetLabel => 'Asset da ricevere';

  @override
  String get walletSwapAmountLabel => 'Importo da scambiare (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Indirizzo di destinazione';

  @override
  String get walletSwapDestinationHint =>
      'Indirizzo di ricezione sulla chain di destinazione';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Indirizzo di ricezione $chain';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Un indirizzo $chain — dove viene inviato l\'asset ricevuto dallo scambio. Verificare attentamente che la chain sia corretta.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Scansiona il codice QR dell\'indirizzo di destinazione';

  @override
  String get walletSwapTargetAssetHint => 'Seleziona un asset da ricevere';

  @override
  String get walletSwapQuoteButton => 'Richiedi quotazione';

  @override
  String get walletSwapQuoting => 'Richiesta della quotazione…';

  @override
  String get walletSwapExecuting => 'Avvio dello scambio…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Ancora in corso — lo scambio si sta avviando. Può richiedere fino a un minuto.';

  @override
  String get walletSwapReviewTitle => 'Conferma scambio';

  @override
  String get walletSwapYouSendLabel => 'Invia';

  @override
  String get walletSwapYouReceiveLabel => 'Riceve almeno';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Commissione di rete';

  @override
  String get walletSwapNetworkFeeValue => 'Aggiunta all\'invio del deposito';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Quotazione valida ancora per circa $time — confermare prima della scadenza.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Quotazione valida ancora per meno di un minuto — confermare prima della scadenza.';

  @override
  String get walletSwapQuoteExpired =>
      'Questa quotazione è scaduta. Torni indietro e ne richieda una nuova — il tasso non è più garantito e inviare ora comporta il rischio di un rimborso.';

  @override
  String get walletCountdownUnderMinute => 'meno di un minuto';

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
  String get walletSwapDeshieldTitle => 'Questo scambio non è privato';

  @override
  String get walletSwapDeshieldBody =>
      'Scambiare in uscita rimuove la schermatura dei ZEC — il deposito è una transazione pubblica, e il lato del fornitore è pubblico sulla sua rete.';

  @override
  String get walletSwapDiscloseTitle => 'Cosa vedrà il fornitore dello scambio';

  @override
  String get walletSwapDiscloseAmounts => 'Gli importi di entrambe le parti';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Che questi ZEC e l\'asset ricevuto fanno parte dello stesso scambio';

  @override
  String get walletSwapDiscloseDestination => 'L\'indirizzo di destinazione';

  @override
  String get walletSwapDiscloseSource => 'L\'indirizzo di origine';

  @override
  String get walletSwapDiscloseIp =>
      'L\'indirizzo IP (a meno di instradamento tramite Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'Altri dettagli di questo scambio';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'Le transazioni del fornitore sono pubbliche sulla sua rete';

  @override
  String get walletSwapAckLabel =>
      'Comprendo che il fornitore vedrà le informazioni sopra indicate.';

  @override
  String get walletSwapConfirmButton => 'Avvia scambio';

  @override
  String get walletSwapBackButton => 'Indietro';

  @override
  String get walletSwapStatusPendingTitle => 'Scambio avviato';

  @override
  String get walletSwapStatusCheckingTitle =>
      'Verifica dello stato dello scambio…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Il portafoglio sta inviando il deposito di ZEC al fornitore. Se si è brevemente offline, l\'invio avviene automaticamente non appena si torna online — ma la finestra di invio è breve, e se si chiude prima, lo scambio termina semplicemente e non avviene alcuno scambio. Lo ZEC resta comunque proprio, ma può richiedere fino a un\'ora prima di essere di nuovo mostrato come disponibile.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'In attesa dell\'arrivo del deposito. Se i fondi non sono ancora stati inviati dall\'altro portafoglio, inviarli prima della scadenza della quotazione.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Questo scambio è ancora in attesa del deposito. Le istruzioni per il deposito non sono più disponibili su questo dispositivo — se i fondi sono già stati inviati, verranno rilevati; in caso contrario, lasciare scadere questo scambio e avviarne uno nuovo.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'La finestra di deposito termina $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'La finestra di deposito è scaduta. Se il deposito non è stato inviato in tempo, lo scambio termina e lo ZEC resta nel portafoglio.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'La finestra di deposito è scaduta. Se non è stato ancora inviato il deposito, questo scambio termina semplicemente — si potrà richiedere una nuova quotazione quando si è pronti.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Scambi in corso',
      one: 'Scambio in corso',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'Lo ZEC è in viaggio verso il fornitore.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'In attesa che il deposito raggiunga il fornitore.';

  @override
  String get walletSwapInFlightRowGeneric => 'Uno scambio è in corso.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'La finestra di deposito è scaduta — verificare lo stato di questo scambio.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Questo scambio non ha ancora raggiunto un esito confermato qui — aprirlo per verificare. Eventuali ZEC in arrivo su questo portafoglio compaiono nel saldo dopo una sincronizzazione.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Questo scambio non ha ancora raggiunto un esito confermato qui — aprirlo per verificare. Eventuali ZEC consegnati da questo scambio a questo portafoglio compaiono nel saldo dopo una sincronizzazione.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Scambio completato.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Scambio rimborsato.';

  @override
  String get walletSwapRowOutcomeFailed => 'Scambio non completato.';

  @override
  String get walletSwapRemove => 'Rimuovi';

  @override
  String get walletSwapRemoveTitle => 'Rimuovere questo scambio dall\'elenco?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Questo rimuove lo scambio solo da questo elenco — non annulla lo scambio, e questo portafoglio smetterà di monitorare il relativo rimborso. Lo ZEC rimborsato in seguito appartiene comunque a questo portafoglio; una nuova scansione completa può trovarlo.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Questo rimuove lo scambio solo da questo elenco — non annulla lo scambio, e questo portafoglio smetterà di monitorare lo ZEC in arrivo. Lo ZEC consegnato in seguito appartiene comunque a questo portafoglio; una nuova scansione completa può trovarlo. Se invece lo scambio viene rimborsato, il rimborso torna nell\'asset inviato, al di fuori di questo portafoglio.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Questo rimuove lo scambio solo da questo elenco — non annulla lo scambio, e questo portafoglio smetterà di monitorare lo ZEC ancora in arrivo da esso. Lo ZEC che arriva in seguito appartiene comunque a questo portafoglio; una nuova scansione completa può trovarlo.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Questo rimuove lo scambio concluso dall\'elenco.';

  @override
  String get walletSwapRemoveCancel => 'Annulla';

  @override
  String get walletSwapRemoveConfirm => 'Rimuovi';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Avviato $time';
  }

  @override
  String get walletSwapViewSwap => 'Mostra lo scambio';

  @override
  String get walletSwapsInFlightError =>
      'Al momento non è stato possibile caricare gli scambi in corso.';

  @override
  String get walletSwapsInFlightRetry => 'Riprova';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Tentativo in corso…';

  @override
  String get walletSwapStartAnother => 'Avvia un altro scambio';

  @override
  String get walletSwapStatusUnderTitle => 'In attesa del deposito completo';

  @override
  String get walletSwapStatusUnderBody =>
      'Parte del deposito è arrivata. Il resto si sta completando, oppure il fornitore effettuerà un rimborso.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Parte del deposito è arrivata. Inviare l\'importo mancante entro la scadenza, altrimenti il fornitore rimborserà quanto arrivato.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Ricevuto $received; mancano ancora $missing. La finestra di deposito termina: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Deposito ricevuto';

  @override
  String get walletSwapStatusDetectedBody =>
      'Il fornitore ha ricevuto il deposito ed elaborerà lo scambio.';

  @override
  String get walletSwapStatusProcessingTitle =>
      'Elaborazione dello scambio in corso';

  @override
  String get walletSwapStatusProcessingBody =>
      'Il fornitore sta completando lo scambio.';

  @override
  String get walletSwapStatusSuccessTitle => 'Scambio completato';

  @override
  String get walletSwapStatusSuccessBody =>
      'Lo scambio è stato completato con successo.';

  @override
  String get walletSwapStatusRefundedTitle => 'Scambio rimborsato';

  @override
  String get walletSwapStatusRefundedBody =>
      'Lo scambio non si è completato, quindi il fornitore ha rimandato i fondi all\'indirizzo di rimborso.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'Lo scambio non si è completato, quindi il fornitore ha rimandato i tuoi ZEC a questo portafoglio. Arrivano come fondi non schermati e compaiono nel tuo saldo dopo la successiva sincronizzazione del portafoglio — l\'operazione può richiedere un po\' di tempo.';

  @override
  String get walletSwapStatusFailedTitle => 'Scambio non riuscito';

  @override
  String get walletSwapStatusFailedBody =>
      'Non è stato possibile completare lo scambio. Eventuali fondi depositati verranno liquidati o rimborsati lato fornitore.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Scambio non trovato';

  @override
  String get walletSwapStatusNotFoundBody =>
      'Il fornitore non ha più traccia di questo scambio — molto probabilmente è scaduto. Se è stato effettuato un deposito, il fornitore dovrebbe rimborsarlo all\'indirizzo di rimborso. Lo scambio resta nell\'elenco e questo portafoglio continua a monitorare il suo ZEC nel caso arrivi ancora; è possibile rimuoverlo dall\'elenco in qualsiasi momento.';

  @override
  String get walletSwapStatusUnknownTitle => 'Stato non disponibile';

  @override
  String get walletSwapStatusUnknownBody =>
      'Non è possibile leggere lo stato di questo scambio in questo momento.';

  @override
  String get walletSwapTrackingUnavailableTitle =>
      'Monitoraggio non disponibile';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'Lo scambio è disattivato, quindi non è possibile monitorarlo qui. Eventuali fondi verranno liquidati o rimborsati lato fornitore.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'Lo scambio è disattivato qui, quindi al momento non è possibile monitorare questo scambio. Se è stato rimborsato, lo ZEC torna a questo portafoglio — compare nel saldo dopo che lo scambio viene riattivato e il portafoglio si sincronizza.';

  @override
  String get walletSwapTrackingError =>
      'Non è stato possibile monitorare questo scambio.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Non è stato possibile aprire il monitoraggio per questo scambio. Lo scambio stesso potrebbe comunque essere ancora in corso — eventuali fondi depositati verranno liquidati o rimborsati lato fornitore.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Inserire l\'indirizzo dove ricevere l\'asset scambiato.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Questo indirizzo di destinazione non è valido per questo asset. Verificarlo e riprovare.';

  @override
  String get walletSwapFaultExpired =>
      'Questa quotazione è scaduta. Richiederne una nuova per continuare.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'Il prezzo del fornitore è uscito dal limite impostato, quindi lo scambio è stato interrotto prima che qualcosa si muovesse. Riprovare.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'Il limite di slippage è troppo alto per uno scambio sicuro. Riprovare.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'Il fornitore dello scambio non è disponibile in questo momento. Riprovare tra poco.';

  @override
  String get walletSwapFaultConnection =>
      'Impossibile raggiungere il servizio di scambio. Verificare la connessione a Internet e riprovare.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'Il fornitore dello scambio ha restituito una risposta imprevista, quindi lo scambio è stato interrotto. Riprovare.';

  @override
  String get walletSwapFaultSwapOff =>
      'Lo scambio è disattivato in questo momento.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Non è stato possibile inviare il deposito, quindi nulla è uscito dal portafoglio. Richiedere una nuova quotazione per riprovare.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Uno scambio è già in corso. Sarà possibile avviarne uno nuovo dopo che sarà liquidato completamente o alla scadenza della quotazione: l\'attesa può richiedere un po\' di tempo.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Il portafoglio non può ancora impostare un indirizzo di rimborso — di solito significa solo che la prima sincronizzazione non è terminata. Attendere il completamento della sincronizzazione, quindi riprovare.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Il portafoglio non può ancora impostare un indirizzo di ricezione per questo scambio — di solito significa solo che la prima sincronizzazione non è terminata. Attendere il completamento della sincronizzazione, quindi riprovare.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'Non è stato possibile avviare lo scambio in tempo — la connessione potrebbe essere stata lenta, oppure il portafoglio era occupato. Richiedere una nuova quotazione e riprovare.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'Il portafoglio è momentaneamente occupato. Riprovare.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Questa quotazione non corrisponde a quella emessa dal portafoglio, quindi non è stato inviato nulla. Richiedi una nuova quotazione e riprova.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Questo scambio richiede circa $needed ZEC, commissione di rete inclusa, ma al momento sono disponibili solo $spendable ZEC.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Al momento questa app limita gli scambi a $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Questo scambio richiede circa $needed ZEC, commissione di rete inclusa, ma al momento sono disponibili solo $spendable ZEC. Il saldo si sta ancora aggiornando — a breve potrebbe diventare disponibile altro saldo.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'Il portafoglio non è riuscito a registrare in modo sicuro questo scambio, quindi nulla si è mosso. Riprovare.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Non è stato possibile elaborare questa richiesta di scambio. Richiedere una nuova quotazione e riprovare.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Impossibile ottenere una quotazione per lo scambio. Verificare i dettagli e riprovare.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Il portafoglio non è pronto in questo momento. Tornare indietro e riprovare.';

  @override
  String get walletSwapDirectionBuy => 'Acquista ZEC';

  @override
  String get walletSwapDirectionSell => 'Vendi ZEC';

  @override
  String get walletSwapRefundLabel => 'Indirizzo di rimborso';

  @override
  String get walletSwapRefundHint =>
      'Dove torneranno le monete se lo scambio non va a buon fine';

  @override
  String get walletSwapRefundHelper =>
      'Sulla chain di invio — non un indirizzo Zcash.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Indirizzo di rimborso $chain';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Un indirizzo $chain — dove torneranno le monete se lo scambio non va a buon fine. Non un indirizzo Zcash.';
  }

  @override
  String get walletSwapRefundInfoTitle =>
      'Informazioni sull\'indirizzo di rimborso';

  @override
  String get walletSwapRefundInfoBody =>
      'Se lo scambio non può completarsi, il fornitore restituisce le monete a questo indirizzo sulla chain da cui è stato effettuato il pagamento. Inserire un indirizzo sotto il proprio controllo — il portafoglio non può verificare un indirizzo esterno, quindi va controllato con attenzione.';

  @override
  String get walletSwapRefundScanTooltip =>
      'Scansiona il codice QR dell\'indirizzo di rimborso';

  @override
  String get walletSwapScanTitle => 'Scansiona indirizzo';

  @override
  String get walletSwapScanInstruction =>
      'Inquadrare il codice QR dell\'indirizzo con la fotocamera.';

  @override
  String get walletSwapScanManualEntry => 'Inserisci manualmente';

  @override
  String get walletSwapScanCancel => 'Annulla';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Fotocamera non disponibile. Inserire l\'indirizzo manualmente qui sotto.';

  @override
  String get walletSwapSourceAssetLabel => 'Asset di partenza';

  @override
  String get walletSwapSourceAssetHint => 'Seleziona un asset';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Importo da inviare ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Importo da inviare';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol su $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Scegli l\'asset di partenza';

  @override
  String get walletSwapPickerTitleReceive => 'Scegli l\'asset da ricevere';

  @override
  String get walletSwapPickerStale =>
      'Impossibile aggiornare l\'elenco degli asset — viene mostrato l\'ultimo elenco noto.';

  @override
  String get walletSwapPickerEmpty =>
      'Nessun asset disponibile per lo scambio in questo momento. Riprovare più tardi.';

  @override
  String get walletSwapPickerSearchHint => 'Cerca per nome o chain';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Nessun asset corrisponde a \"$query\".';
  }

  @override
  String get walletSwapPickerError =>
      'Impossibile caricare l\'elenco degli asset. Verificare la connessione e riprovare.';

  @override
  String get walletSwapPickerRetry => 'Riprova';

  @override
  String get walletSwapSlippageLabel => 'Tolleranza di slippage';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Personalizzata';

  @override
  String get walletSwapSlippageCustomLabel => 'Slippage personalizzato';

  @override
  String get walletSwapSlippageMayFail =>
      'Molto bassa — lo scambio potrebbe non riuscire se il prezzo si muove.';

  @override
  String get walletSwapSlippageNormal => 'Una tolleranza sicura.';

  @override
  String get walletSwapSlippageRisky =>
      'Alta — si potrebbe ricevere un importo notevolmente inferiore a quello indicato.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Troppo alta — lo scambio verrà rifiutato. Ridurla al 10% o meno.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Riceverà almeno $zec ZEC — la soglia minima allo $slippage% di slippage. L\'importo finale non scenderà sotto questo valore.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'Riceverà i ZEC sul proprio indirizzo';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Finché non verrà schermato — un tocco, con promemoria all\'arrivo — l\'importo ricevuto resta brevemente pubblico e visibile sulla blockchain. Un importo ridotto potrebbe restare pubblico finché non si accumula.';

  @override
  String get walletSwapRefundVerifyTitle => 'Verifica l\'indirizzo di rimborso';

  @override
  String get walletSwapRefundVerifyBody =>
      'Lo verifichi carattere per carattere — è qui che le monete torneranno se lo scambio non va a buon fine. Il portafoglio non può verificare un indirizzo esterno.';

  @override
  String get walletSwapRefundVerifyAck =>
      'Ho verificato che l\'indirizzo di rimborso sia corretto.';

  @override
  String get walletSwapPayoutVerifyTitle =>
      'Verifica l\'indirizzo di ricezione';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Lo verifichi carattere per carattere — è qui che riceverà $asset. Il portafoglio non può verificare un indirizzo esterno.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'Ho verificato che l\'indirizzo di ricezione sia corretto.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'Lo scambio è disattivato qui. Eventuali ZEC già in transito compariranno nel portafoglio dopo la prossima sincronizzazione.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Inserire l\'importo da scambiare.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Inserire l\'indirizzo di rimborso sulla chain di origine.';

  @override
  String get walletSwapDepositTitle => 'Invia il pagamento';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Inviare esattamente $amount $asset su $chain all\'indirizzo sottostante.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Inviare l\'importo esatto. Inviare meno, oppure inviare dopo la chiusura della finestra, comporta il rimborso da parte del fornitore all\'indirizzo di rimborso.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Finestra di deposito: $time rimanenti';
  }

  @override
  String get walletSwapDepositExpired =>
      'Questa finestra di deposito si è chiusa. Non inviare fondi ora — avviare un nuovo scambio. Se sono già stati inviati, il fornitore dovrebbe rimborsare all\'indirizzo di rimborso.';

  @override
  String get walletSwapDepositQrLabel =>
      'Codice QR dell\'indirizzo di deposito';

  @override
  String get walletSwapDepositAddressLabel => 'Indirizzo di deposito';

  @override
  String get walletSwapDepositCopy => 'Copia indirizzo di deposito';

  @override
  String get walletSwapDepositCopied => 'Indirizzo di deposito copiato';

  @override
  String get walletSwapDepositMemoRequired =>
      'Questo deposito richiede un memo / tag';

  @override
  String get walletSwapDepositMemoWarning =>
      'È OBBLIGATORIO includere esattamente questo memo con il deposito. Inviare senza di esso — o con un memo errato — può causare la perdita permanente dei fondi.';

  @override
  String get walletSwapDepositMemoLabel => 'Memo / tag di deposito';

  @override
  String get walletSwapDepositMemoCopy => 'Copia memo';

  @override
  String get walletSwapDepositMemoCopied => 'Memo copiato';

  @override
  String get walletSwapDepositSent => 'Ho inviato i fondi';

  @override
  String get walletSwapDepositBackTitle => 'Uscire da questa schermata?';

  @override
  String get walletSwapDepositBackBody =>
      'Questa azione non annulla lo scambio, che prosegue in background. Tuttavia l\'indirizzo di deposito sarà necessario per pagare: copiarlo prima, se non è già stato fatto.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Questa azione non annulla lo scambio, che prosegue in background. La finestra di deposito si è chiusa. Non inviare fondi all\'indirizzo di deposito ora. Se sono già stati inviati, il fornitore dovrebbe rimborsare all\'indirizzo di rimborso.';

  @override
  String get walletSwapDepositBackStay => 'Resta';

  @override
  String get walletSwapDepositBackLeave => 'Esci';

  @override
  String get walletReceive => 'Ricevi';

  @override
  String get walletReceiveSubtitle =>
      'Condividi questo indirizzo per ricevere ZEC. È sicuro condividerlo pubblicamente.';

  @override
  String get walletReceiveCopy => 'Copia indirizzo';

  @override
  String get walletReceiveCopied => 'Indirizzo copiato';

  @override
  String get walletReceiveUnavailable => 'Il portafoglio non è ancora pronto.';

  @override
  String get walletReceiveError =>
      'Impossibile caricare l\'indirizzo. Riprovare.';

  @override
  String get walletReceivePreparing => 'Preparazione dell\'indirizzo…';

  @override
  String get walletReceivePreparingHint =>
      'Il portafoglio prepara questo indirizzo sul dispositivo — potrebbe richiedere qualche istante se è occupato con altre operazioni.';

  @override
  String get walletReceiveRetry => 'Riprova';

  @override
  String get walletReceiveQrLabel => 'Codice QR dell\'indirizzo di ricezione';

  @override
  String get walletReceiveTypeShielded => 'Schermato';

  @override
  String get walletReceiveTypeTransparent => 'Pubblico';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Condividi questo indirizzo pubblico per ricevere ZEC da un mittente che non può pagare un indirizzo schermato.';

  @override
  String get walletReceiveTransparentWarning =>
      'Questo è un indirizzo pubblico: è visibile sulla blockchain e collega i pagamenti se riutilizzato. Preferire l\'indirizzo schermato; schermare questi fondi dopo averli ricevuti.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'Codice QR dell\'indirizzo di ricezione pubblico';

  @override
  String get walletReceiveFreshAddress => 'Usa un nuovo indirizzo';

  @override
  String get walletReceiveFreshCaption =>
      'Nuovo indirizzo — non può essere collegato ai tuoi altri indirizzi. I pagamenti a questo indirizzo continuano ad arrivare in questo portafoglio, e i tuoi indirizzi precedenti continuano a funzionare. Non verrà mostrato di nuovo qui — copialo ora.';

  @override
  String get walletReceiveFreshError =>
      'Impossibile creare un nuovo indirizzo. Riprovare.';

  @override
  String get walletReceiveFreshBusy =>
      'Il portafoglio è occupato al momento. Riprovare tra poco con il nuovo indirizzo.';

  @override
  String get walletReceiveShare => 'Condividi';

  @override
  String get walletReceiveRequestAmount => 'Richiedi importo';

  @override
  String get walletReceiveRequestAmountLabel => 'Importo (facoltativo)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Non verrà mostrato di nuovo qui — copialo ora.';

  @override
  String get walletSecurityMenuItem => 'Sicurezza…';

  @override
  String get securityTitle => 'Sicurezza';

  @override
  String get securityUnavailableBody =>
      'Le impostazioni di sicurezza del portafoglio sono gestite da questa app, non dal portafoglio stesso.';

  @override
  String get securityCustodySectionTitle => 'Custodia delle chiavi';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (hardware)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (hardware)';

  @override
  String get securityCustodyTierTee => 'Keystore hardware (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Keystore software';

  @override
  String get securityCustodyTierKeychain => 'Keychain (crittografia software)';

  @override
  String get securityCustodyTierNone => 'Nessun keystore hardware';

  @override
  String get securityCustodyTierUnknown => 'Sconosciuto';

  @override
  String get securityCustodyHardwareKey =>
      'La chiave che blocca questo portafoglio è custodita nell\'hardware sicuro di questo dispositivo e viene eliminata con il portafoglio.';

  @override
  String get securityCustodyBestEffort =>
      'L\'eliminazione rimuove le chiavi con il massimo impegno possibile; una breve finestra di recupero forense può restare aperta finché il dispositivo non recupera lo spazio di archiviazione. Per la massima sicurezza, utilizzare anche la funzione di cancellazione totale dei contenuti del dispositivo.';

  @override
  String get securityCustodyProbeError =>
      'Impossibile leggere lo stato di custodia. Tornare indietro e riprovare.';

  @override
  String get securityDeleteWalletButton => 'Elimina portafoglio';

  @override
  String get securityDeleteWalletSubtitle =>
      'Elimina questo portafoglio e la sua chiave da questo dispositivo. I fondi restano sulla blockchain e sono ripristinabili tramite la frase di recupero.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Elimina questo portafoglio e la sua chiave da questo dispositivo. Non contiene alcuna chiave di spesa, quindi non c\'è alcun backup da eseguire — lo aggiunga di nuovo in qualsiasi momento con la sua chiave di visualizzazione.';

  @override
  String get securityDeleteDialogTitle => 'Eliminare questo portafoglio?';

  @override
  String get securityDeleteDialogBody =>
      'Questa operazione rimuove il portafoglio e la sua chiave da questo dispositivo. Si assicuri di aver eseguito il backup della frase di recupero: è l\'UNICO modo per ripristinare i fondi.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Questa operazione rimuove il portafoglio e la sua chiave da questo dispositivo. Non contiene alcuna chiave di spesa, quindi non è necessario eseguire alcun backup — può aggiungerlo di nuovo in seguito con la sua chiave di visualizzazione.';

  @override
  String get securityDeleteDialogConfirm => 'Elimina';

  @override
  String get securityDeleteDialogCancel => 'Annulla';

  @override
  String get securityDeleteFailedSnack =>
      'Impossibile eliminare il portafoglio — il portafoglio è invariato. Riprovare.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Completare prima il cambio di server: termina o si interrompe entro $seconds secondi. Poi riprovare a eliminare il portafoglio.';
  }

  @override
  String get walletParkedTitle => 'Salvati e in sospeso';

  @override
  String get walletParkedSubtitle =>
      'Questi pagamenti non sono ancora stati inviati. I relativi importi restano parte del saldo.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Questi pagamenti non sono ancora stati inviati. I relativi importi restano parte del saldo — tranne quelli che il portafoglio sta inviando, il cui importo potrebbe già essere riservato.';

  @override
  String get walletParkedCancel => 'Annulla';

  @override
  String get walletParkedPausedHint =>
      'In pausa — questo pagamento non verrà inviato da solo. I fondi sono al sicuro. Inviare ora o annullare.';

  @override
  String get walletParkedRetryStale =>
      'Questo pagamento non è più in attesa. Verificare i pagamenti in sospeso e l\'attività.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Questo pagamento non è più in attesa — è possibile che il portafoglio lo stia già inviando. Verificare Salvati e in sospeso e l\'attività.';

  @override
  String get walletReclaimExplainer =>
      'Gli invii tramite indirizzo monouso sono bloccati. È possibile riaprirli — l\'operazione sposta un piccolo importo tra i propri indirizzi e lo restituisce.';

  @override
  String get walletReclaimButton => 'Riapri l\'invio';

  @override
  String get walletReclaimInProgress => 'Riapertura in corso…';

  @override
  String get walletReclaimConfirmTitle =>
      'Riaprire l\'invio tramite indirizzo monouso?';

  @override
  String get walletReclaimConfirmBody =>
      'Questa operazione sposta un piccolo importo tra i propri indirizzi per liberare l\'invio tramite indirizzo monouso, quindi lo restituisce. Comporta un paio di commissioni di rete. Una volta confermata, l\'importo spostato potrà essere recuperato con Recupera ora.';

  @override
  String get walletReclaimConfirmCancel => 'Non ora';

  @override
  String get walletReclaimConfirmAction => 'Riapri';

  @override
  String get walletReclaimStarted =>
      'Riapertura avviata. Una volta confermata, inviare il pagamento in pausa, quindi recuperare l\'importo spostato con Recupera ora.';

  @override
  String get walletReclaimNothing => 'Niente da riaprire in questo momento.';

  @override
  String get walletReclaimNotBroadcast =>
      'Impossibile confermare che l\'operazione abbia raggiunto la rete. Potrebbe comunque essere andata a buon fine. Riprovare tra poco.';

  @override
  String get walletReclaimNeedsFunds =>
      'Servono ZEC schermati per riaprire l\'invio.';

  @override
  String get walletReclaimFailed =>
      'Impossibile riaprire l\'invio in questo momento. I fondi sono invariati. Riprovare.';

  @override
  String get walletReclaimUnknown =>
      'Riapertura terminata. Verificare i propri invii tramite indirizzo monouso e recuperare l\'eventuale importo spostato con Recupera ora.';

  @override
  String get walletParkedError =>
      'Impossibile caricare i pagamenti in sospeso in questo momento.';

  @override
  String get walletParkedErrorRetry => 'Riprova';

  @override
  String get walletParkedErrorRetryInProgress => 'Tentativo in corso…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Annullare questo pagamento in sospeso?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Questa operazione elimina il pagamento salvato. Non è stato inviato, quindi nulla esce dal portafoglio — ma l\'operazione non può essere annullata.';

  @override
  String get walletParkedCancelConfirmKeep => 'Mantieni';

  @override
  String get walletParkedCancelConfirmDiscard => 'Elimina pagamento';

  @override
  String get walletParkedCancelDone => 'Pagamento in sospeso annullato.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Questo pagamento potrebbe essere già in transito — verificare l\'attività.';

  @override
  String get walletParkedCancelFailed =>
      'Impossibile annullare in questo momento. Il pagamento è invariato. Riprovare.';

  @override
  String get walletRecoverNow => 'Recupera ora';

  @override
  String get walletRecoverConfirmTitle => 'Recuperare nel saldo schermato?';

  @override
  String get walletRecoverConfirmBody =>
      'Questa operazione controlla gli indirizzi monouso e sposta quanto trovato nel saldo privato schermato. Può essere eseguita di nuovo in qualsiasi momento in sicurezza.';

  @override
  String get walletRecoverConfirmCancel => 'Non ora';

  @override
  String get walletRecoverConfirmAction => 'Recupera';

  @override
  String get walletRecoverInProgress => 'Recupero in corso…';

  @override
  String walletRecoverDone(String amount) {
    return 'Recupero di $amount in corso verso il saldo schermato.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'Recupero di $amount in corso — alcuni fondi richiedono un altro tentativo.';
  }

  @override
  String get walletRecoverRetry =>
      'Alcuni fondi richiedono un altro tentativo — eseguire di nuovo il recupero.';

  @override
  String get walletRecoverTruncated =>
      'Non tutti gli indirizzi monouso sono stati controllati — eseguire di nuovo per controllare i restanti.';

  @override
  String get walletRecoverNothing => 'Niente da recuperare in questo momento.';

  @override
  String get walletRecoverFailed =>
      'Impossibile recuperare in questo momento. I fondi sono invariati. Riprovare.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount salvato e in sospeso · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Annulla il pagamento di $amount salvato $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount in pausa · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount in preparazione per l\'invio · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Il portafoglio sta preparando questo pagamento — l\'importo potrebbe già essere riservato. I fondi sono al sicuro. Se non si completa, torna da solo nell\'elenco.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Il portafoglio sta preparando questo pagamento — l\'importo potrebbe già essere riservato. I fondi sono al sicuro, ma potrà completarsi solo quando il portafoglio tornerà a sincronizzarsi.';

  @override
  String get walletParkedSendNow => 'Invia ora';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Invio in corso del pagamento di $amount salvato $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Invia ora il pagamento di $amount salvato $time';
  }

  @override
  String get walletParkedSendNowInProgress => 'Invio in corso…';

  @override
  String get walletParkedAuthorizeSent => 'Invio del pagamento in corso.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Invio del pagamento in corso. Se non va a buon fine, il portafoglio potrà completarlo solo quando tornerà a sincronizzarsi.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Non ancora pronto per l\'invio. Il pagamento è salvato e invariato.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Non ancora pronto per l\'invio. Il pagamento è salvato e non è più in pausa — riprovi più tardi con Invia ora, oppure lo annulli.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Impossibile inviarlo in questo momento. Il pagamento è invariato. Riprovare.';

  @override
  String get walletTransparentFundsMenuItem => 'Fondi pubblici…';

  @override
  String get walletTransparentFundsTitle => 'Fondi pubblici';

  @override
  String get walletTransparentFundsIntro =>
      'I fondi pubblici sono pubblicamente visibili sulla blockchain — l\'importo, gli indirizzi e la cronologia delle monete.';

  @override
  String get walletExpertToggleLabel => 'Avanzate: fondi pubblici';

  @override
  String get walletExpertToggleDescription =>
      'Mostra i controlli avanzati per gestire i fondi pubblici e disattivare la schermatura automatica.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Mostra i controlli avanzati per gestire i fondi pubblici.';

  @override
  String get walletAutoShieldToggleLabel => 'Scherma automaticamente';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Quando il saldo pubblico raggiunge $minZec ZEC, viene spostato automaticamente nel saldo schermato. Con questa opzione disattivata, i fondi pubblici restano pubblicamente visibili finché non vengono schermati manualmente.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Impossibile salvare l\'impostazione. Riprovare.';

  @override
  String get walletAutoShieldIncomplete =>
      'La schermatura automatica non è stata completata — questi fondi sono ancora pubblicamente visibili. È possibile schermarli ora.';

  @override
  String get walletSendPrivacyShielded =>
      'Pagamento schermato — l\'importo e il destinatario restano privati sulla blockchain.';

  @override
  String get walletSendPrivacyTransparent =>
      'Pagamento pubblico — l\'importo e gli indirizzi sono visibili sulla blockchain.';

  @override
  String get walletActivityPublicBadge =>
      'Pubblicamente visibile sulla blockchain';

  @override
  String get walletShieldWalletEnded =>
      'La sessione del portafoglio è terminata. Chiudere e riaprire per riprovare.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'I nuovi fondi pubblici vengono schermati automaticamente nel saldo privato non appena raggiungono $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'La schermatura automatica è disattivata — i fondi pubblici restano pubblicamente visibili finché non vengono schermati.';

  @override
  String get walletMoveAutoShieldNote =>
      'La schermatura automatica è attiva: dopo l\'arrivo di questi fondi, verranno schermati di nuovo automaticamente (con un\'ulteriore commissione). Per mantenerli pubblici, disattivare prima la schermatura automatica in Fondi pubblici.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Dopo questo spostamento il saldo pubblico sarà di $amount ZEC, sotto i $floor ZEC necessari per schermarlo di nuovo. Resterà pubblico finché non arrivano altri fondi.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Lo spostamento avviene verso il proprio indirizzo pubblico. Questo movimento resta permanentemente nel registro pubblico.';

  @override
  String get walletTxDetailVisibility => 'Visibilità';

  @override
  String get walletTransparentFundsAutoDenied =>
      'La schermatura automatica è in pausa per questa sessione — non è stata approvata. È ancora possibile schermare i fondi manualmente.';

  @override
  String get walletDeepScanMenuItem =>
      'Verifica gli indirizzi di scambio meno recenti…';

  @override
  String get walletMenuSyncNotRunningHint =>
      'La sincronizzazione non è in corso in questo momento.';

  @override
  String get walletDeepScanTitle =>
      'Verifica gli indirizzi di scambio meno recenti';

  @override
  String get walletDeepScanBody =>
      'Se ha ripristinato questo portafoglio e un tempo utilizzava molto gli scambi, i fondi dei suoi scambi più vecchi potrebbero richiedere un passaggio in più per essere trovati. Questo controllo li cerca — tutto ciò che viene trovato compare nel saldo man mano che il portafoglio si sincronizza.';

  @override
  String get walletDeepScanCoverage =>
      'I suoi indirizzi di scambio meno recenti sono stati verificati fino a qui. Se pensa che manchino ancora fondi di uno scambio più vecchio, può verificare ancora più a fondo.';

  @override
  String get walletDeepScanCoveragePending =>
      'L\'intervallo attuale è ancora in fase di verifica — tutto ciò che viene trovato compare nel saldo. L\'operazione può richiedere un po\' di tempo.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Verifica la presenza di fondi provenienti dagli scambi più vecchi del portafoglio.';

  @override
  String get walletDeepScanCheckButton => 'Verifica indirizzi meno recenti';

  @override
  String get walletDeepScanCheckDeeperButton =>
      'Verifica indirizzi ancora meno recenti';

  @override
  String get walletDeepScanChecking => 'Verifica in corso…';

  @override
  String get walletDeepScanClose => 'Chiudi';

  @override
  String get walletDeepScanTorHint =>
      'Al momento non è connesso tramite Tor. Per una maggiore privacy, valuti di attendere che Tor sia attivo prima di procedere con la verifica.';

  @override
  String get walletDeepScanRescanBusy =>
      'Potrà verificare gli indirizzi di scambio meno recenti al termine della nuova scansione.';

  @override
  String get walletDeepScanRan =>
      'Verifica degli indirizzi di scambio meno recenti in corso — tutto ciò che viene trovato comparirà nel saldo.';

  @override
  String get walletDeepScanFailed =>
      'Impossibile avviare la verifica. Nulla è cambiato — riprovi.';

  @override
  String get walletDeepScanSlow =>
      'L\'operazione sta richiedendo più tempo del solito. Se i suoi indirizzi di scambio meno recenti sono stati verificati, tutto ciò che viene trovato comparirà nel saldo — controlli di nuovo a breve.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'Lo scambio è disattivato al momento, quindi questa operazione non può essere eseguita. Riprovi quando lo scambio sarà disponibile.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'L\'ultimo intervallo è ancora in fase di verifica — può richiedere fino a un paio di giorni, ma di solito molto meno. Si conclude da solo: riprovi più tardi.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'Al momento non è ancora possibile confermare la privacy della connessione. Per una maggiore privacy, valuti di verificare una volta che Tor sia attivo.';

  @override
  String get walletDeepScanBannerChecking =>
      'Verifica degli indirizzi di scambio meno recenti ancora in corso — tutto ciò che viene trovato comparirà nel saldo.';

  @override
  String get walletRescanSwapPointer =>
      'Sta cercando fondi provenienti da uno scambio precedente? Una nuova scansione non li troverà — utilizzi invece “Verifica gli indirizzi di scambio meno recenti”.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Ha ripristinato un portafoglio che utilizzava gli scambi?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Se questo portafoglio aveva una cronologia di scambi molto lunga, i fondi dei suoi scambi più vecchi potrebbero richiedere un passaggio in più per essere trovati. La maggior parte dei portafogli non richiede alcuna azione.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Verifica ora';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Chiudi';

  @override
  String walletTorHostPath(String transport) {
    return 'Tramite il percorso privato della tua app ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Tramite il percorso privato della tua app ($transport); il proxy può collegare le connessioni';
  }

  @override
  String get walletTorHostOtherTransport => 'un percorso privato';

  @override
  String get walletTorHostDirect =>
      'Non privato (connessione diretta della tua app)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Il server salvato usa un indirizzo non cifrato, che il percorso privato della tua app non può trasportare. In uso $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Altro su $label';
  }

  @override
  String get walletSendPaste => 'Incolla';

  @override
  String get walletSendScanQr => 'Scansiona codice QR';

  @override
  String get walletSendRecipientGetsLabel => 'Il destinatario riceve';

  @override
  String get walletSwapDepositCopyAmount => 'Copia importo';

  @override
  String get walletSwapDepositAmountCopied => 'Importo copiato';

  @override
  String get walletScanOpenSettings => 'Apri impostazioni';

  @override
  String get walletScanOpenSettingsFailed =>
      'Impossibile aprire le impostazioni.';

  @override
  String get walletSendLeaveTitle => 'Invio in corso';

  @override
  String get walletSendLeaveBody =>
      'Il pagamento continua anche se esci. Vedrai com\'è andato nella tua attività.';

  @override
  String get walletSendLeaveStay => 'Resta';

  @override
  String get walletSendLeaveConfirm => 'Esci';

  @override
  String get walletSheetLeaveBody =>
      'Continua anche se esci. Vedrai com\'è andata nella tua attività.';

  @override
  String get walletLoadingLabel => 'Caricamento';

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
      'Controlla prima di schermare di nuovo';

  @override
  String get walletShieldUnknownBody =>
      'Non è stato possibile confermare questa schermatura. Controlla Attività prima di riprovare.';

  @override
  String get walletMoveUnknownTitle => 'Controlla prima di spostare di nuovo';

  @override
  String get walletMoveUnknownBody =>
      'Non è stato possibile confermare questo spostamento. Controlla Attività prima di riprovare.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
