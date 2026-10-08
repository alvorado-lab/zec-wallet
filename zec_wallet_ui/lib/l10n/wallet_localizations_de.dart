// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for German (`de`).
class WalletLocalizationsDe extends WalletLocalizations {
  WalletLocalizationsDe([String locale = 'de']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Einstellungen';

  @override
  String get walletTitle => 'Wallet';

  @override
  String get walletNotSetUpTitle => 'Wallet noch nicht eingerichtet';

  @override
  String get walletNotSetUpBody =>
      'Die Wallet-Einrichtung folgt in einer späteren Version. Sie führt Sie durch das Notieren Ihrer Wiederherstellungsphrase, bevor Guthaben empfangen werden kann – so ist nie etwas ohne Sicherung gefährdet.';

  @override
  String get walletStartupFailedTitle =>
      'Das Wallet konnte nicht gestartet werden';

  @override
  String get walletStartupFailedBody =>
      'Etwas hat verhindert, dass das Wallet auf diesem Gerät geladen wird. Falls Sie bereits ein Wallet haben, ist dessen Guthaben davon nicht betroffen – es befindet sich im Zcash-Netzwerk und kann mit Ihrer Wiederherstellungsphrase wiederhergestellt werden. Versuchen Sie es erneut; wenn dies weiterhin auftritt, schließen Sie die App und öffnen Sie sie erneut.';

  @override
  String get walletBalanceLabel => 'Guthaben';

  @override
  String get walletHideBalance => 'Guthaben ausblenden';

  @override
  String get walletShowBalance => 'Guthaben anzeigen';

  @override
  String get walletBalanceHiddenAmount => 'Guthaben ausgeblendet';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Jetzt verfügbar';

  @override
  String get walletArrivingLabel => 'Eingehend';

  @override
  String get walletNotSpendableYetLabel => 'Noch nicht verfügbar';

  @override
  String get walletActivityTitle => 'Aktivität';

  @override
  String get walletActivityEmpty => 'Noch keine Aktivität';

  @override
  String get walletActivityError => 'Aktivität konnte nicht geladen werden';

  @override
  String get walletActivityReceived => 'Empfangen';

  @override
  String get walletActivitySent => 'Gesendet';

  @override
  String get walletActivityPending => 'Ausstehend';

  @override
  String get walletActivityQueued => 'In Warteschlange';

  @override
  String get walletActivityRetrying => 'Wird erneut versucht';

  @override
  String get walletActivitySaved => 'Gespeichert';

  @override
  String get walletActivityExpired => 'Abgelaufen';

  @override
  String get walletActivityFailed => 'Fehlgeschlagen';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Bestätigungen',
      one: '1 Bestätigung',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Zahlungen erhalten',
      one: 'Zahlung erhalten',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Transaktionsdetails anzeigen';

  @override
  String get walletTxDetailStatus => 'Status';

  @override
  String get walletTxDetailFee => 'Netzwerkgebühr';

  @override
  String get walletTxDetailDate => 'Datum';

  @override
  String get walletTxDetailHeight => 'Blockhöhe';

  @override
  String get walletTxDetailMemo => 'Memo';

  @override
  String get walletTxDetailMemoAttached => 'Enthalten';

  @override
  String get walletTxDetailTxid => 'Transaktions-ID';

  @override
  String get walletTxDetailCopyTxid => 'Transaktions-ID kopieren';

  @override
  String get walletTxDetailCopied => 'Transaktions-ID kopiert';

  @override
  String get walletTxDetailClose => 'Schließen';

  @override
  String get walletTxFundsKept => 'Kein Guthaben hat Ihr Wallet verlassen';

  @override
  String get walletTxExplainQueued =>
      'Auf diesem Gerät gespeichert, unter „Gespeichert & ausstehend“ – Sie können sie dort senden oder abbrechen.';

  @override
  String get walletTxExplainPending =>
      'An das Zcash-Netzwerk gesendet – wartet auf Bestätigung in einem Block.';

  @override
  String get walletTxExplainRetrying =>
      'Ihr Wallet konnte dies noch nicht an das Zcash-Netzwerk senden. Es behält die signierte Transaktion und versucht es bei jeder Synchronisierung erneut, bis sie durchgeht oder abläuft.';

  @override
  String get walletTxExplainSaved =>
      'Ihr Wallet hat diese signierte Transaktion behalten, sendet sie momentan aber nicht von selbst.';

  @override
  String get walletTxExplainConfirmed => 'Im Zcash-Netzwerk bestätigt.';

  @override
  String get walletTxExplainExpired =>
      'Diese Transaktion ist abgelaufen, bevor das Netzwerk sie bestätigt hat, und wurde daher storniert. Der Betrag steht Ihnen weiterhin zur Verfügung.';

  @override
  String get walletTxExplainFailed =>
      'Das Netzwerk hat diese Transaktion abgelehnt, sie wurde daher nicht ausgeführt. Der Betrag steht Ihnen weiterhin zur Verfügung.';

  @override
  String get walletTxExplainUnknown =>
      'Der aktuelle Status dieser Transaktion kann nicht ermittelt werden. Er wird nach der nächsten Synchronisierung aktualisiert.';

  @override
  String get walletMenuTooltip => 'Weitere Optionen';

  @override
  String get walletRescanMenuItem => 'Verlauf erneut scannen…';

  @override
  String get walletCheckOneTimeMenuItem => 'Einmalige Adressen prüfen…';

  @override
  String get walletRescanTitle => 'Verlauf erneut scannen';

  @override
  String get walletRescanBody =>
      'Fehlen ältere Guthaben? Scannen Sie die Blockchain weiter zurück, um Einzahlungen wiederherzustellen, die ein späteres Startdatum übersprungen hat. Ihr Guthaben und Ihre Wiederherstellungsphrase sind dabei nie gefährdet.';

  @override
  String get walletRescanRangeTitle => 'Wie weit zurück gescannt werden soll';

  @override
  String get walletRescanRangeAll =>
      'Gesamten Verlauf scannen – am langsamsten, aber stellt alles wieder her.';

  @override
  String get walletRescanRangeDefault =>
      'Scannt ab dem Start Ihres Wallets. Fehlen weiterhin ältere Guthaben? Wählen Sie ein früheres Datum oder scannen Sie den gesamten Verlauf.';

  @override
  String get walletRescanRangeResolving =>
      'Empfohlener Bereich wird vorbereitet…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Etwa $blocks Blöcke zu scannen.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Scannt ab $date. Fehlen weiterhin ältere Guthaben? Wählen Sie ein früheres Datum oder scannen Sie den gesamten Verlauf.';
  }

  @override
  String get walletRescanPick => 'Datum auswählen';

  @override
  String get walletRescanChange => 'Datum ändern';

  @override
  String get walletRescanScanAll => 'Gesamten Verlauf scannen';

  @override
  String get walletRescanDatePick => 'Frühestes zu scannendes Datum';

  @override
  String get walletRescanWarning =>
      'Dies scannt die Blockchain erneut. Aktuelle Daten dauern Minuten, weiter zurückliegende können Stunden dauern. Die Synchronisierung läuft im Hintergrund – Sie können Ihr Wallet weiterhin nutzen.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Eine Zahlung aus diesem Wallet wird noch bestätigt. Das Wallet lehnt einen erneuten Scan üblicherweise ab, bis diese abgeschlossen ist – Sie können es versuchen, aber rechnen Sie mit einer Ablehnung.';

  @override
  String get walletRescanConfirm => 'Erneuten Scan starten';

  @override
  String get walletRescanCancel => 'Abbrechen';

  @override
  String get walletRescanRunning => 'Wird neu aufgebaut…';

  @override
  String get walletRescanRebuildingAll =>
      'Ihr Verlauf wird neu aufgebaut – die gesamte Chain wird gescannt. Guthaben und Aktivität füllen sich, sobald der Rückstand aufgeholt ist.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Ihr Verlauf wird ab $date neu aufgebaut – Guthaben und Aktivität füllen sich, sobald der Rückstand aufgeholt ist.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Ihr Verlauf wird ab dem Start Ihres Wallets neu aufgebaut – Guthaben und Aktivität füllen sich, sobald der Rückstand aufgeholt ist.';

  @override
  String get walletCatchUpBanner =>
      'Wird aufgeholt – Guthaben und Aktivität füllen sich, während das Wallet synchronisiert. Alles, was Sie empfangen haben, ist sicher.';

  @override
  String get walletCatchUpRescanBanner =>
      'Ihr Verlauf wird nach einem erneuten Scan neu aufgebaut – Guthaben und Aktivität füllen sich, sobald der Rückstand aufgeholt ist. Alles, was Sie empfangen haben, ist sicher.';

  @override
  String get walletRescanFailedNotice =>
      'Erneuter Scan momentan nicht möglich – Ihr Guthaben ist sicher, auch wenn Guthaben und Verlauf ein wenig Zeit brauchen, um wieder aktuell zu sein. Versuchen Sie es gleich noch einmal.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Eine Zahlung wird noch bestätigt, daher ist der erneute Scan pausiert, um Ihr Guthaben zu schützen. Ihr Wallet ist unverändert – versuchen Sie es in ein paar Stunden erneut und halten Sie die App währenddessen geöffnet und online.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'Der erneute Scan baut Ihren Verlauf neu auf, während Ihr Wallet synchronisiert, und die Synchronisierung läuft momentan nicht. Ihr Wallet ist unverändert – versuchen Sie es erneut, sobald die Synchronisierung läuft.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Es ist nicht genügend freier Speicherplatz vorhanden, um Ihren Verlauf neu aufzubauen – Ihr Guthaben ist sicher, auch wenn Guthaben und Verlauf ein wenig Zeit brauchen, um wieder aktuell zu sein. Geben Sie Speicherplatz frei und versuchen Sie es erneut.';

  @override
  String get walletRescanFailedDismiss => 'Schließen';

  @override
  String get walletActivityRebuilding => 'Verlauf wird neu aufgebaut…';

  @override
  String get walletActivityCatchingUp =>
      'Wird noch aufgeholt – Alles, was Sie empfangen haben, wird hier angezeigt.';

  @override
  String get walletActivitySyncNotRunning =>
      'Ihr Guthaben und Verlauf werden fertig geladen, sobald die Synchronisierung läuft.';

  @override
  String get walletActivityLoadMore => 'Mehr laden';

  @override
  String get walletPendingChangeLabel => 'Ausstehendes Wechselgeld';

  @override
  String get walletTransparentLabel => 'Ungeschirmt (öffentlich)';

  @override
  String get walletTransparentNote =>
      'Nicht in „Jetzt verfügbar“ enthalten – schirmen Sie dieses Guthaben, um es ausgeben zu können. Bis dahin bleibt es auf der Chain öffentlich sichtbar.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Dieses Guthaben bleibt auf der Chain öffentlich sichtbar.';

  @override
  String walletPoolShielded(String amount) {
    return 'Geschirmt $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Öffentlich $amount';
  }

  @override
  String get walletPoolAllShielded => 'Alles geschirmt · privat';

  @override
  String get walletPoolTapHint => 'Öffentliches Guthaben anzeigen';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount Ihres Guthabens liegt auf einer einmaligen Adresse (wiederherstellbar).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount Ihres Guthabens liegt auf einer einmaligen Adresse.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Zahlungen über insgesamt $amount sind reserviert und werden noch über einmalige Adressen abgeschlossen, die Ihr Wallet kontrolliert. Senden Sie sie nicht erneut.',
      one:
          '$amount ist für eine Zahlung reserviert, die Ihr Wallet noch über eine einmalige Adresse abschließt, die es kontrolliert. Senden Sie den Betrag nicht erneut.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Zahlungen über insgesamt $amount sind reserviert und befinden sich auf halbem Weg durch einmalige Adressen, die Ihr Wallet kontrolliert. Sie sind pausiert, bis Ihr Wallet wieder synchronisiert. Senden Sie sie nicht erneut.',
      one:
          '$amount ist für eine Zahlung reserviert, die sich auf halbem Weg durch eine einmalige Adresse befindet, die Ihr Wallet kontrolliert. Sie ist pausiert, bis Ihr Wallet wieder synchronisiert. Senden Sie den Betrag nicht erneut.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Es konnte nicht geprüft werden, ob eine Zahlung noch abgeschlossen wird. Neuer Versuch läuft – sehen Sie bis dahin in Ihren Aktivitäten nach einer ausstehenden Zahlung, bevor Sie erneut senden.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount Ihres Guthabens liegt auf einer einmaligen Adresse (wird noch bestätigt).';
  }

  @override
  String get walletShieldButton => 'Schirmen';

  @override
  String get walletShieldSheetTitle => 'Öffentliches Guthaben schirmen';

  @override
  String get walletShieldNote =>
      'Dies verschiebt Guthaben von Ihrem öffentlichen, auf der Chain sichtbaren Bestand in Ihr privates, geschirmtes Guthaben.';

  @override
  String get walletShieldPreparing => 'Wird vorbereitet…';

  @override
  String get walletShieldAmountLabel => 'Wird geschirmt';

  @override
  String get walletShieldFeeLabel => 'Netzwerkgebühr';

  @override
  String get walletShieldNetLabel => 'Kommt geschirmt an';

  @override
  String get walletShieldConfirmButton => 'Jetzt schirmen';

  @override
  String get walletShieldSubmitting => 'Wird geschirmt…';

  @override
  String get walletShieldNothingTitle => 'Noch nichts zu schirmen';

  @override
  String get walletShieldNothingBody =>
      'Dieses Guthaben liegt derzeit unter dem Betrag, der sich zum Schirmen lohnt – die Netzwerkgebühr würde den Nutzen übersteigen. Es lässt sich schirmen, sobald etwas mehr eingegangen ist.';

  @override
  String get walletShieldDoneTitle => 'Schirmen übermittelt';

  @override
  String get walletShieldDoneBody =>
      'Ihr Guthaben wird in Ihr geschirmtes Guthaben verschoben. Es wird in Kürze auf der Chain bestätigt.';

  @override
  String get walletShieldSavedTitle =>
      'Gespeichert – wir schließen den Schirmvorgang ab';

  @override
  String get walletShieldSavedBody =>
      'Das Netzwerk war momentan nicht erreichbar. Ihr Schirmvorgang ist gespeichert, und Ihr Wallet schließt ihn bei einer späteren Synchronisierung ab. Es geht nichts verloren.';

  @override
  String get walletShieldAlreadyTitle => 'Bereits übermittelt';

  @override
  String get walletShieldFailedTitle => 'Schirmen momentan nicht möglich';

  @override
  String get walletShieldStaleBody =>
      'Das Wallet synchronisiert noch. Versuchen Sie es gleich noch einmal mit dem Schirmen.';

  @override
  String get walletShieldTransientBody =>
      'Das Schirmen konnte gerade nicht vorbereitet werden. Versuchen Sie es gleich noch einmal.';

  @override
  String get walletShieldStorageFullBody =>
      'Es ist nicht genügend freier Speicherplatz vorhanden, um jetzt zu schirmen. Geben Sie Speicherplatz frei und versuchen Sie es erneut. Ihr Guthaben ist sicher.';

  @override
  String get walletShieldClose => 'Schließen';

  @override
  String get walletShieldRetry => 'Erneut versuchen';

  @override
  String get walletMoveMenuItem => 'Zu öffentlich verschieben…';

  @override
  String get walletMoveSheetTitle => 'Zu öffentlich verschieben';

  @override
  String get walletMoveSheetSubtitle =>
      'Senden Sie geschirmte ZEC an Ihre eigene öffentliche Adresse – nützlich für eine Börse, die keine geschirmte Einzahlung akzeptiert.';

  @override
  String get walletMoveDestinationLabel => 'Ihre öffentliche Adresse';

  @override
  String walletMoveAvailable(String amount) {
    return 'Verfügbar zum Verschieben: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Verfügbar zum Verschieben: $amount ZEC – Ihr Guthaben holt noch auf';
  }

  @override
  String get walletMoveDeshieldTitle =>
      'Diese Verschiebung macht Ihr Guthaben öffentlich';

  @override
  String get walletMoveDeshieldBody =>
      'Das Verschieben zu einer öffentlichen Adresse entnimmt dieses Guthaben Ihrem geschirmten Bestand – der Betrag und Ihre öffentliche Adresse werden auf der Zcash-Blockchain öffentlich sichtbar.';

  @override
  String get walletMoveWalletEnded =>
      'Die Wallet-Sitzung wurde beendet. Schließen und erneut öffnen, um es noch einmal zu versuchen.';

  @override
  String get walletMoveLoading => 'Wird vorbereitet…';

  @override
  String get walletMovePreparing => 'Betrag wird geprüft…';

  @override
  String get walletMoveSubmitting => 'Wird verschoben…';

  @override
  String get walletMoveReviewButton => 'Prüfen';

  @override
  String get walletMoveCancel => 'Abbrechen';

  @override
  String get walletMoveReviewTitle => 'Verschiebung prüfen';

  @override
  String get walletMoveOwnAddressNote =>
      'Sie verschieben zu Ihrer eigenen öffentlichen Adresse. Sie können dieses Guthaben später wieder schirmen, doch diese Verschiebung bleibt dauerhaft im öffentlichen Register sichtbar.';

  @override
  String get walletMoveConfirmButton => 'Zu öffentlich verschieben';

  @override
  String get walletMoveBackButton => 'Zurück';

  @override
  String get walletMoveDoneTitle => 'Zu öffentlich verschoben';

  @override
  String get walletMoveDoneBody =>
      'Ihr Guthaben wird zu Ihrer öffentlichen Adresse verschoben. Es wird in Kürze auf der Chain bestätigt.';

  @override
  String get walletMoveSavedTitle =>
      'Gespeichert – wir schließen die Verschiebung ab';

  @override
  String get walletMoveSavedBody =>
      'Diese Verschiebung ist gespeichert, und Ihr Wallet sendet sie bei einer späteren Synchronisierung. Es ist nichts verloren gegangen.';

  @override
  String get walletMoveAlreadyTitle => 'Bereits übermittelt';

  @override
  String get walletMoveAlreadyBody =>
      'Dieses Guthaben wurde bereits übermittelt und ist auf dem Weg zu Ihrer öffentlichen Adresse.';

  @override
  String get walletMoveFailedTitle =>
      'Verschiebung konnte nicht abgeschlossen werden';

  @override
  String get walletMoveNothingTitle => 'Noch nichts zu verschieben';

  @override
  String get walletMoveNothingBody =>
      'Sie haben derzeit kein geschirmtes Guthaben, das verschoben werden kann. Sobald Guthaben bestätigt ist, können Sie es zu Ihrer öffentlichen Adresse verschieben.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Ihr Wallet holt noch auf – alles, was Sie empfangen haben, wird zum Verschieben verfügbar, sobald die Synchronisierung abgeschlossen ist.';

  @override
  String get walletMoveCouldNotLoad =>
      'Ihre öffentliche Adresse konnte nicht geladen werden. Versuchen Sie es erneut.';

  @override
  String get walletMoveRetry => 'Erneut versuchen';

  @override
  String get walletMoveClose => 'Schließen';

  @override
  String get walletSnapshotUnavailable =>
      'Das Wallet konnte momentan nicht gelesen werden. Es aktualisiert sich von selbst.';

  @override
  String get walletBalanceStale =>
      'Aktualisierung nicht möglich – zeigt Ihr zuletzt bekanntes Guthaben.';

  @override
  String get walletSyncStartFailed =>
      'Synchronisierung konnte nicht gestartet werden. Wir versuchen es weiter.';

  @override
  String get walletSyncRetry => 'Erneut versuchen';

  @override
  String get walletSyncTryNow => 'Jetzt versuchen';

  @override
  String get walletSyncIdle => 'Synchronisierung ausstehend';

  @override
  String get walletSyncIdleDetail => 'Synchronisierung startet automatisch.';

  @override
  String get walletSyncDisabled => 'Synchronisierung aus';

  @override
  String get walletSyncDisabledDetail =>
      'Aktivieren Sie die Synchronisierung in den Einstellungen dieser App, um Ihr Guthaben zu aktualisieren.';

  @override
  String get walletSyncExplainDisabled =>
      'Die Synchronisierung ist in den Einstellungen dieser App deaktiviert. Ihr Guthaben ist sicher. Ihr Guthaben und Ihre Aktivität zeigen den zuletzt synchronisierten Stand und werden erst aktualisiert, wenn die Synchronisierung wieder aktiviert wird.';

  @override
  String get walletParkedSyncPausedNote =>
      'Ihr Wallet synchronisiert gerade nicht, daher werden diese Zahlungen nicht von selbst gesendet. Verwenden Sie „Jetzt senden“, um eine davon selbst zu senden.';

  @override
  String get walletSyncPausedMoneyNote =>
      'Pausiert, bis Ihr Wallet wieder synchronisiert.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Verbindung wird hergestellt…';

  @override
  String get walletSyncStartingDetail =>
      'Verbindung zum Zcash-Netzwerk wird hergestellt, Scan wird vorbereitet.';

  @override
  String get walletSyncConnecting => 'Verbindung wird hergestellt…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Verbindung wird hergestellt… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Scan $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'Scan läuft…';

  @override
  String get walletSyncSpendableReady => 'Guthaben ist zum Ausgeben bereit.';

  @override
  String get walletSyncCatchingUp =>
      'Wird mit dem Netzwerk synchronisiert – eine umfangreiche Erstsynchronisierung kann eine Weile dauern. Sie können die App währenddessen weiter nutzen';

  @override
  String walletSyncScanRemaining(String count) {
    return 'Noch $count Blöcke';
  }

  @override
  String get walletSyncUpToDate => 'Aktuell';

  @override
  String get walletSyncOffline => 'Offline';

  @override
  String get walletSyncOfflineDetail =>
      'Eingereihte Zahlungen bleiben unter „Gespeichert & ausstehend“ gespeichert.';

  @override
  String get walletSyncUnknown => 'Wird synchronisiert…';

  @override
  String get walletSyncStalled => 'Synchronisierung pausiert';

  @override
  String get walletStallEndpoint =>
      'Das Zcash-Netzwerk ist momentan nicht erreichbar. Wir versuchen es automatisch weiter – prüfen Sie Ihre Verbindung, oder der Server ist vorübergehend nicht verfügbar.';

  @override
  String get walletStallTor =>
      'Der private Pfad Ihrer App ist nicht verfügbar, daher verbindet sich das Wallet nicht. Prüfen Sie die Netzwerkeinstellungen Ihrer App oder schalten Sie den privaten Pfad aus. Die Synchronisierung wird fortgesetzt, sobald der Pfad wieder da ist.';

  @override
  String get walletStallStorage =>
      'Der Gerätespeicher ist voll. Geben Sie Speicherplatz frei, damit die Synchronisierung fortgesetzt wird.';

  @override
  String get walletStallReorg =>
      'Die Chain wurde reorganisiert; aktuelle Blöcke werden erneut geprüft.';

  @override
  String get walletStallInternal =>
      'Ein lokales Problem hat die Synchronisierung gestoppt. Falls dies wiederholt auftritt, stellen Sie aus Ihrer Wiederherstellungsphrase wieder her.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Dieser Server hat Daten gesendet, die nicht stimmen können, daher wurde die Synchronisierung gestoppt. Das ist kein Verbindungsproblem – wechseln Sie zu einem anderen Server. Wird jeder Server abgelehnt, scannen Sie den Verlauf erneut: Die Wallet hält möglicherweise einen fehlerhaften Eintrag eines früheren Servers.';

  @override
  String get walletStallBirthdayInFuture =>
      'Diese Wallet ist so eingestellt, dass sie bei einem Block beginnt, den dieser Server noch nicht erreicht hat. Prüfen Sie den Startblock, auf den diese Wallet eingestellt ist, oder versuchen Sie einen anderen Server.';

  @override
  String get walletStallStorageUnavailable =>
      'Synchronisierung auf diesem Gerät pausiert. Neuer Versuch läuft.';

  @override
  String get walletStallUnknown =>
      'Synchronisierung aus unbekanntem Grund gestoppt.';

  @override
  String get walletSyncBadgeHint => 'Synchronisierungsdetails anzeigen';

  @override
  String get walletSyncSheetClose => 'Schließen';

  @override
  String get walletSyncSheetProgress => 'Fortschritt';

  @override
  String get walletSyncSheetBlocksLeft => 'Verbleibende Blöcke';

  @override
  String get walletSyncSheetSyncedTo => 'Synchronisiert bis Block';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Mindestens $blocks Blöcke im Rückstand',
      one: 'Mindestens 1 Block im Rückstand',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'Die Synchronisierung hat noch nicht begonnen – sie startet automatisch. Keine Aktion erforderlich.';

  @override
  String get walletSyncExplainStartFailed =>
      'Die Synchronisierung konnte nicht gestartet werden. Ihr Guthaben ist sicher – das Wallet prüft momentan nur nicht auf neue Aktivität. Versuchen Sie es unten erneut, oder öffnen Sie die App erneut.';

  @override
  String get walletSyncExplainStarting =>
      'Das Wallet nimmt Kontakt zum Zcash-Netzwerk auf und bereitet den Scan vor. Das dauert normalerweise nur wenige Sekunden.';

  @override
  String get walletSyncExplainConnecting =>
      'Verbindung zum Zcash-Netzwerk wird hergestellt.';

  @override
  String get walletSyncExplainScanning =>
      'Das Wallet prüft Blockchain-Blöcke auf Ihr Guthaben. Guthaben und Aktivität werden aktualisiert, sobald neue Transaktionen gefunden werden – Sie können die App währenddessen weiter nutzen.';

  @override
  String get walletSyncExplainUpToDate =>
      'Vollständig mit dem Zcash-Netzwerk synchronisiert. Guthaben und Aktivität sind aktuell.';

  @override
  String get walletSyncExplainStalled =>
      'Bei der Synchronisierung ist ein Problem aufgetreten, sie ist pausiert. Sie wird automatisch erneut versucht.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Das Zcash-Netzwerk ist nicht erreichbar – das ist normal, wenn Sie offline sind, oder der Server ist vorübergehend nicht verfügbar. Ihr Guthaben ist sicher: Es zeigt den zuletzt synchronisierten Stand, und eingereihte Zahlungen bleiben unter „Gespeichert & ausstehend“ gespeichert. Die Verbindung versucht es von selbst erneut.';

  @override
  String get walletSyncExplainOffline =>
      'Keine Netzwerkverbindung. Ihr Guthaben ist sicher – es zeigt den zuletzt synchronisierten Stand, und eingereihte Zahlungen bleiben unter „Gespeichert & ausstehend“ gespeichert.';

  @override
  String get walletSyncExplainUnknown =>
      'Das Wallet synchronisiert. Guthaben und Aktivität werden mit dem Fortschritt aktualisiert.';

  @override
  String get walletTorOff => 'Tor aus';

  @override
  String get walletTorBootstrapping => 'Privater Pfad startet…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport startet…';
  }

  @override
  String get walletTorActive => 'Tor aktiv';

  @override
  String get walletTorActiveUnverified =>
      'Tor aktiv (nicht verifizierte Laufzeitumgebung)';

  @override
  String get walletTorActiveUnattested =>
      'Privater Pfad in Verwendung (Privatsphäre nicht verifiziert)';

  @override
  String get walletTorFellBack =>
      'Tor nicht verfügbar – Direktverbindung wird genutzt';

  @override
  String get walletTorUnavailable =>
      'Privater Pfad nicht verfügbar – nicht verbunden';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport nicht verfügbar – nicht verbunden';
  }

  @override
  String get walletTorUnanswered =>
      'Privater Pfad verbunden – es kommt nichts zurück';

  @override
  String get walletTorUnansweredUnattested =>
      'Privater Pfad verbunden – es kommt nichts zurück (Privatsphäre nicht verifiziert)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport verbunden – es kommt nichts zurück';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Nicht privat (Direktverbindung Ihrer App) – es kommt nichts zurück';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Verbunden über $transport – es kommt nichts zurück; Verbindungen können vom Proxy verknüpft werden';
  }

  @override
  String get walletTorUnknown =>
      'Tor-Status unbekannt – als ungeschützt behandeln';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Guthaben (Stand Block $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Guthaben · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Guthaben (Stand Block $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Verbindung';

  @override
  String get walletSyncSheetServer => 'Server';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Server, $host, öffnet die Serverauswahl';
  }

  @override
  String get walletSyncServerSheetTitle => 'Sync-Server';

  @override
  String get walletSyncServerInUse => 'In Verwendung';

  @override
  String get walletSyncServerAppDefault => 'App-Standard';

  @override
  String get walletSyncServerCustom => 'Eigener Server…';

  @override
  String get walletSyncServerCustomHint => 'https://host:port';

  @override
  String get walletSyncServerCheck => 'Server prüfen';

  @override
  String get walletSyncServerChecking => 'Wird geprüft…';

  @override
  String get walletSyncServerUse => 'Diesen Server verwenden';

  @override
  String get walletSyncServerSwitching => 'Wird gewechselt…';

  @override
  String get walletSyncServerContinue => 'Weiter';

  @override
  String get walletSyncServerCancel => 'Abbrechen';

  @override
  String get walletSyncServerTrustTitle => 'Diesem Server vertrauen?';

  @override
  String get walletSyncServerTrustNotice =>
      'Sie vertrauen diesem Server, Ihren Kontostand und Verlauf zu melden und Ihre Zahlungen weiterzuleiten. Er sieht Ihre IP-Adresse (außer bei aktivem Tor), ungefähr wann Ihre Wallet erstellt wurde, die öffentlichen Adressen, die Ihre Wallet prüft, die Transaktionen, die sie nachschlägt, und die Transaktionen, die Sie senden.';

  @override
  String get walletSyncServerKeyLabel => 'Zugangsschlüssel (optional)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Header für den Schlüssel';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Geben Sie den Header ein, den Ihr Server erwartet';

  @override
  String get walletSyncServerKeyInvalid =>
      'Dieser Schlüssel oder Header kann nicht verwendet werden';

  @override
  String get walletSyncServerKeySaved => 'Schlüssel gespeichert';

  @override
  String get walletSyncServerKeyShow => 'Anzeigen';

  @override
  String get walletSyncServerKeyHide => 'Verbergen';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Ihr Schlüssel identifiziert Sie gegenüber diesem Server. Er kann Ihre Zahlungen mit Ihrer Wallet verknüpfen, auch über Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Der Wechsel startet die laufende Synchronisierung neu. Kontostand und Verlauf bleiben erhalten. Guthaben kann als eingehend erscheinen, bis der Scan des neuen Servers aufgeholt hat.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Der Wechsel verbindet neu mit dem neuen Server. Kontostand und Verlauf bleiben erhalten.';

  @override
  String get walletSyncServerUnreachable =>
      'Dieser Server ist nicht erreichbar. Prüfen Sie die Adresse – und wenn sie stimmt, antwortet entweder dieser Server nicht, oder Ihre App erreicht ihn gerade nicht. Versuchen Sie es erneut, oder wählen Sie einen anderen Server.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Dieser Server ist nicht erreichbar. Das Wallet kann nicht unterscheiden, ob dieser Server nicht antwortet oder Ihre App ihn gerade nicht erreicht. Wählen Sie einen anderen Server, oder versuchen Sie es später erneut.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Dieser Server gehört zu einem anderen Zcash-Netzwerk.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Das sieht nicht wie eine Serveradresse aus. Verwenden Sie https://host:port.';

  @override
  String get walletSyncServerNotOffered =>
      'Dieser Server wird von dieser App nicht angeboten.';

  @override
  String get walletSyncServerBusy =>
      'Die Wallet ist gerade beschäftigt. Versuchen Sie es gleich noch einmal.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'Der gewählte Server wird von dieser App nicht mehr angeboten. Es wird $host verwendet.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'Die gespeicherte Serverauswahl konnte nicht gelesen werden. Es wird $host verwendet.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Wechsel nicht möglich – weiterhin $host in Verwendung.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Der Wallet-Datenverkehr verbindet sich direkt mit dem Server. Der Server kann Ihre IP-Adresse sehen.';

  @override
  String get walletTransportExplainTor =>
      'Der Wallet-Datenverkehr wird über das Tor-Netzwerk geleitet, das Ihre IP-Adresse vor dem Server verbirgt.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Der private Pfad Ihrer App startet. Der Wallet-Datenverkehr wartet darauf, bevor er sich verbindet.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport startet. Der Wallet-Datenverkehr wartet darauf, bevor er sich verbindet.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Tor war nicht erreichbar, daher wurde auf eine Direktverbindung zurückgegriffen. Der Server kann Ihre IP-Adresse sehen.';

  @override
  String get walletTransportExplainUnavailable =>
      'Der private Pfad Ihrer App ist nicht verfügbar, daher verbindet sich das Wallet nicht. Schalten Sie den privaten Pfad aus oder prüfen Sie die Netzwerkeinstellungen Ihrer App.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport ist nicht verfügbar, daher verbindet sich das Wallet nicht. Schalten Sie es aus oder prüfen Sie die Netzwerkeinstellungen Ihrer App.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'Der private Pfad hat die Verbindung angenommen, aber seit einer Minute kommt nichts zurück. Es kann am Pfad oder am Wallet-Server liegen – das Wallet kann das nicht unterscheiden. Es versucht es weiter; wenn es nicht aufhört, probieren Sie einen anderen Server oder prüfen Sie die Netzwerkeinstellungen Ihrer App.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport hat die Verbindung angenommen, aber seit einer Minute kommt nichts zurück. Es kann am Pfad oder am Wallet-Server liegen – das Wallet kann das nicht unterscheiden. Es versucht es weiter; wenn es nicht aufhört, probieren Sie einen anderen Server oder prüfen Sie die Netzwerkeinstellungen Ihrer App.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Der Wallet-Datenverkehr verbindet sich direkt mit dem Server. Der Server kann Ihre IP-Adresse sehen. Die Verbindung wurde angenommen, aber seit einer Minute kommt nichts zurück. Es kann am Pfad oder am Wallet-Server liegen – das Wallet kann das nicht unterscheiden. Es versucht es weiter; wenn es nicht aufhört, probieren Sie einen anderen Server oder prüfen Sie die Netzwerkeinstellungen Ihrer App.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'Die Privatsphäre dieser Verbindung kann nicht überprüft werden – behandeln Sie sie als nicht privat. Die Verbindung wurde angenommen, aber seit einer Minute kommt nichts zurück. Es kann am Pfad oder am Wallet-Server liegen – das Wallet kann das nicht unterscheiden. Es versucht es weiter; wenn es nicht aufhört, probieren Sie einen anderen Server oder prüfen Sie die Netzwerkeinstellungen Ihrer App.';

  @override
  String get walletTransportExplainUnverified =>
      'Die Privatsphäre dieser Verbindung kann nicht überprüft werden – behandeln Sie sie als nicht privat.';

  @override
  String get walletTransportExplainHostProxy =>
      'Der Wallet-Datenverkehr wird über den Privatsphäre-Transport dieser App geleitet, der Ihre IP-Adresse vor dem Server verbirgt.';

  @override
  String get walletOnboardingWelcomeTitle => 'Wallet einrichten';

  @override
  String get walletOnboardingWelcomeBody =>
      'Erstellen Sie ein neues Wallet, um ZEC zu empfangen und zu halten. Wir erstellen eine Wiederherstellungsphrase und führen Sie durch deren Sicherung, bevor Guthaben eingehen kann – so ist nie etwas ohne Sicherung gefährdet.';

  @override
  String get walletCreateButton => 'Neues Wallet erstellen';

  @override
  String get walletRestoreButton =>
      'Aus Wiederherstellungsphrase wiederherstellen';

  @override
  String get walletWatchOnlyButton => 'Wallet beobachten (Nur-Lese)';

  @override
  String get walletWatchOnlyTitle => 'Wallet beobachten';

  @override
  String get walletWatchOnlyBody =>
      'Fügen Sie einen Prüfschlüssel ein, um eine Wallet ohne ihre Sendeschlüssel zu beobachten. Sie sehen Guthaben und Verlauf, können aber kein Guthaben senden. Wählen Sie das ungefähre Startdatum der Wallet, damit wir wissen, wie weit wir zurückschauen müssen.';

  @override
  String get walletWatchOnlyKeyLabel => 'Prüfschlüssel';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => 'QR-Code des Prüfschlüssels scannen';

  @override
  String get walletWatchOnlyScanTitle => 'Prüfschlüssel scannen';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Richten Sie Ihre Kamera auf den QR-Code des Prüfschlüssels.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Kamera nicht verfügbar. Fügen Sie den Schlüssel stattdessen manuell ein.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Stattdessen einfügen';

  @override
  String get walletWatchOnlyScanHint =>
      'Oder tippen Sie auf die Scan-Schaltfläche, um den QR-Code eines Prüfschlüssels zu lesen.';

  @override
  String get walletWatchOnlyScanFilled => 'Prüfschlüssel gescannt.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Startdatum der Wallet';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Scannt ab $date – zuvor empfangenes Guthaben wird nicht angezeigt. Älteres Wallet? Wählen Sie ein früheres Datum.';
  }

  @override
  String get walletWatchOnlyBirthdayPick =>
      'Wählen Sie das Startdatum der Wallet';

  @override
  String get walletWatchOnlyBirthdayChange => 'Datum ändern';

  @override
  String get walletWatchOnlySubmit => 'Diese Wallet beobachten';

  @override
  String get walletWatchOnlyBack => 'Zurück';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Das sieht nicht wie ein gültiger Prüfschlüssel aus. Prüfen Sie ihn und versuchen Sie es erneut.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Dieser Prüfschlüssel ist für ein anderes Netzwerk. Er kann hier nicht verwendet werden.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'Auf diesem Gerät ist bereits eine Wallet vorhanden. Gehen Sie zurück und öffnen Sie sie stattdessen.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Dieses Startdatum ist zu aktuell. Wählen Sie ein früheres Datum.';

  @override
  String get walletRestoreTitle => 'Wallet wiederherstellen';

  @override
  String get walletRestoreBody =>
      'Geben Sie Ihre Wiederherstellungsphrase ein, um Ihr Wallet wiederherzustellen – tippen oder fügen Sie die Wörter der Reihe nach ein, getrennt durch Leerzeichen. Nur Standardphrasen: Wenn Ihr Wallet eine zusätzliche Passphrase (ein „25. Wort“) verwendet hat, kann diese App es noch nicht wiederherstellen – Sie würden ein leeres Wallet sehen, keine Fehlermeldung.';

  @override
  String get walletRestorePhraseHint => 'Wort eins  Wort zwei  Wort drei  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count Wörter',
      one: '1 Wort',
      zero: 'Noch keine Wörter',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'Wiederherstellungsphrasen haben 12, 15, 18, 21 oder 24 Wörter';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count Wörter sind keine Wiederherstellungswörter – korrigieren Sie die markierten Wörter',
      one:
          '1 Wort ist kein Wiederherstellungswort – korrigieren Sie das markierte Wort',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'Wort $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'Wort $index: kein Wiederherstellungswort';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Wort $index entfernen';
  }

  @override
  String get walletRestoreSubmit => 'Wallet wiederherstellen';

  @override
  String get walletRestoreBack => 'Zurück';

  @override
  String get walletRestoreBirthdayTitle =>
      'Wie weit zurück gescannt werden soll';

  @override
  String get walletRestoreBirthdayNone =>
      'Wir scannen Ihren gesamten Verlauf – langsamer, aber es wird nichts übersehen.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Scannt ab $date – zuvor empfangenes Guthaben wird nicht angezeigt. Älteres Wallet? Wählen Sie ein früheres Datum oder scannen Sie den gesamten Verlauf.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Datum auswählen';

  @override
  String get walletRestoreBirthdayChange => 'Datum ändern';

  @override
  String get walletRestoreBirthdayClear => 'Gesamten Verlauf scannen';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'Wort $index ist kein Wiederherstellungswort. Prüfen Sie Ihre Phrase auf Tippfehler und versuchen Sie es erneut.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Diese Wiederherstellungsphrase ist ungültig. Prüfen Sie die Wörter und ihre Reihenfolge und versuchen Sie es erneut.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Diese Phrase stimmt nicht mit dem Wallet auf diesem Gerät überein. Prüfen Sie sie sorgfältig und versuchen Sie es erneut.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'Auf diesem Gerät ist bereits ein Wallet vorhanden. Gehen Sie zurück, um es zu öffnen.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Dieses Datum ist zu aktuell. Wählen Sie ein früheres Datum oder scannen Sie alles.';

  @override
  String get walletGeneratingLabel => 'Ihr Wallet wird erstellt…';

  @override
  String get walletOpeningLabel => 'Ihr Wallet wird geöffnet…';

  @override
  String get walletBackupTitle => 'Wiederherstellungsphrase sichern';

  @override
  String get walletBackupBody =>
      'Diese Wörter sind der EINZIGE Weg, Ihr Wallet und Ihr Guthaben wiederherzustellen. Schreiben Sie sie der Reihe nach auf und bewahren Sie sie an einem sicheren, privaten Ort auf. Teilen Sie sie niemals und speichern Sie sie nie online – wer diese Wörter kennt, kann Ihr Guthaben entwenden.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Screenshots sind auf diesem Bildschirm deaktiviert.';

  @override
  String get walletBackupSecureNoteOther =>
      'Stellen Sie sicher, dass niemand Ihren Bildschirm sehen kann.';

  @override
  String get walletBackupReveal => 'Wiederherstellungsphrase anzeigen';

  @override
  String get walletBackupRevealing =>
      'Ihre Wiederherstellungsphrase wird vorbereitet…';

  @override
  String get walletBackupRevealFailed =>
      'Ihre Wiederherstellungsphrase konnte momentan nicht angezeigt werden. Stellen Sie sicher, dass Ihr Gerät entsperrt ist, und versuchen Sie es erneut.';

  @override
  String get walletBackupRetryReveal => 'Erneut versuchen';

  @override
  String get walletBackupReauthFailed =>
      'Ihre Identität konnte nicht bestätigt werden. Bitte versuchen Sie es erneut.';

  @override
  String get walletBackupConfirmCheckbox =>
      'Ich habe meine Wiederherstellungsphrase aufgeschrieben und sicher aufbewahrt.';

  @override
  String get walletBackupContinue => 'Weiter';

  @override
  String get walletBackupSaveFailed =>
      'Ihre Bestätigung konnte nicht gespeichert werden. Bitte versuchen Sie es erneut.';

  @override
  String get walletBackupStartOver => 'Von vorn beginnen';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Ohne dieses Wallet von vorn beginnen?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Dadurch wird dieses Wallet von diesem Gerät gelöscht und Sie kehren zum Anfang zurück. Bevor die Einrichtung abgeschlossen ist, kann über diese App nichts eingezahlt werden.\n\nFalls dieses Wallet je Guthaben enthielt oder aus einer Wiederherstellungsphrase wiederhergestellt wurde, kann nur diese Phrase es wiederherstellen.';

  @override
  String get walletBackupStartOverConfirm => 'Löschen und von vorn beginnen';

  @override
  String get walletBackupStartOverKeep => 'Dieses Wallet behalten';

  @override
  String get walletBackupSectionTitle => 'Wiederherstellungsphrase';

  @override
  String get walletBackupTileTitle => 'Wiederherstellungsphrase sichern';

  @override
  String get walletBackupTileSubtitle =>
      'Zeigen Sie die Wörter an, mit denen Sie Ihr Wallet und Ihr Guthaben wiederherstellen können.';

  @override
  String get walletBackupScreenTitle => 'Wiederherstellungsphrase';

  @override
  String get walletBackupDone => 'Fertig';

  @override
  String get walletBackupManagedTitle =>
      'Keine eigene Wiederherstellungsphrase';

  @override
  String get walletBackupManagedBody =>
      'Dieses Wallet wurde mit Ihrem Konto aus der App eingerichtet, die es installiert hat, und besitzt daher keine eigene Wiederherstellungsphrase. Ihr Guthaben wird zusammen mit diesem Konto wiederhergestellt – nutzen Sie dessen Backup, um es zu schützen.';

  @override
  String get walletExportViewingKeyTitle => 'Prüfschlüssel exportieren';

  @override
  String get walletExportViewingKeyTileTitle => 'Prüfschlüssel exportieren';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Teilen Sie eine Nur-Lese-Kopie Ihres Wallets – sie kann Ihren Verlauf sehen, aber nicht ausgeben.';

  @override
  String get walletExportViewingKeyWarning =>
      'Dieser Schlüssel ermöglicht es jedem, der ihn besitzt, alles zu sehen, was dieses Wallet je empfangen und gesendet hat – und alles, was es in Zukunft empfangen und senden wird. Er kann Ihr Guthaben nicht ausgeben und Ihr Wallet nicht wiederherstellen. Teilen Sie ihn nur mit jemandem, dem Sie Ihren vollständigen Verlauf anvertrauen würden, etwa einem Buchhalter oder Ihrem eigenen zweiten Gerät. Die einzige Möglichkeit, die Freigabe später wieder rückgängig zu machen, besteht darin, Ihr Guthaben in ein neues Wallet zu verschieben.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Dieser Schlüssel ermöglicht es jedem, der ihn besitzt, alles zu sehen, was dieses Wallet je empfangen und gesendet hat – und alles, was es in Zukunft empfangen und senden wird. Er kann Ihr Guthaben nicht ausgeben und Ihr Wallet nicht wiederherstellen. Teilen Sie ihn nur mit jemandem, dem Sie Ihren vollständigen Verlauf anvertrauen würden, etwa einem Buchhalter oder Ihrem eigenen zweiten Gerät. Ist er einmal geteilt, lässt sich die Freigabe nicht mehr rückgängig machen.';

  @override
  String get walletExportViewingKeyReveal => 'Prüfschlüssel anzeigen';

  @override
  String get walletExportViewingKeyRetry => 'Erneut versuchen';

  @override
  String get walletExportViewingKeyRevealing =>
      'Ihr Prüfschlüssel wird vorbereitet…';

  @override
  String get walletExportViewingKeyFailed =>
      'Ihr Prüfschlüssel konnte momentan nicht angezeigt werden. Versuchen Sie es in Kürze erneut.';

  @override
  String get walletExportViewingKeyQrLabel => 'QR-Code des Prüfschlüssels';

  @override
  String get walletExportViewingKeyCopy => 'Prüfschlüssel kopieren';

  @override
  String get walletExportViewingKeyCopied => 'Prüfschlüssel kopiert';

  @override
  String get walletExportViewingKeyDone => 'Fertig';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Screenshots sind auf diesem Bildschirm deaktiviert.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Stellen Sie sicher, dass niemand Ihren Bildschirm sehen kann.';

  @override
  String get walletWatchOnlySectionTitle => 'Über dieses Nur-Lese-Wallet';

  @override
  String get walletWatchOnlyAboutBody =>
      'Dies ist ein Nur-Lese-Wallet. Es wurde aus einem Prüfschlüssel eingerichtet und kann daher Ihr Guthaben und Ihren Verlauf sehen, besitzt jedoch keine Sendeschlüssel – hier gibt es nichts zu sichern, und es kann kein Guthaben senden.';

  @override
  String get walletWatchOnlyBadge => 'Nur-Lese';

  @override
  String get walletOnboardingFailedTitle =>
      'Wallet-Einrichtung konnte nicht abgeschlossen werden';

  @override
  String get walletOnboardingRetry => 'Erneut versuchen';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'Der sichere Speicher Ihres Telefons reagiert nicht. Entsperren Sie Ihr Gerät und versuchen Sie es erneut. Wenn das weiterhin passiert, starten Sie Ihr Telefon neu.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Dieses Wallet ist in einem anderen Fenster oder einer anderen App geöffnet oder schließt gerade noch einen vorherigen Vorgang ab. Schließen Sie andere Fenster, die es verwenden – oder warten Sie einen Moment – und versuchen Sie es dann erneut.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'Der sichere Schlüssel dieses Wallets ist nicht mehr verfügbar, daher kann es auf diesem Gerät nicht geöffnet werden. Ihr Guthaben ist sicher – stellen Sie es über Ihre Wiederherstellungsphrase wieder her.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Aus Wiederherstellungsphrase wiederherstellen';

  @override
  String get walletOnboardingRecoverConfirmTitle =>
      'Dieses Wallet wiederherstellen?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Stellen Sie sicher, dass Sie Ihre Wiederherstellungsphrase zur Hand haben, bevor Sie fortfahren – Sie benötigen sie auf dem nächsten Bildschirm, um Ihr Guthaben wiederherzustellen. Ihr Guthaben ist auf der Blockchain sicher und wird durch diese Phrase kontrolliert. Dadurch werden die unlesbaren Wallet-Daten von diesem Gerät entfernt, damit es neu aufgebaut werden kann.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Abbrechen';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Es ist nicht genügend freier Speicherplatz vorhanden, um Ihr Wallet einzurichten. Geben Sie Speicherplatz frei und versuchen Sie es erneut.';

  @override
  String get walletOnboardingFailedNoVault =>
      'Dieses Gerät verfügt über keinen sicheren Schlüsselspeicher, daher kann das Wallet Ihre Wiederherstellungsphrase hier nicht schützen.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Das Netzwerk war während der Einrichtung nicht erreichbar. Prüfen Sie Ihre Verbindung und versuchen Sie es erneut.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'Die Wallet-Einrichtung wurde nicht abgeschlossen. Versuchen Sie es erneut, um sie abzuschließen – es ist nichts verloren gegangen.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Bei der Einrichtung Ihres Wallets ist etwas schiefgelaufen. Versuchen Sie es erneut.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'Die Wallet-Einrichtung dieser App ist fehlerhaft, daher kann das Wallet nicht gestartet werden. Erneutes Versuchen hilft nicht – bitte melden Sie dies dem Entwickler der App. Ihr Guthaben ist davon nicht betroffen.';

  @override
  String get walletSendButton => 'Senden';

  @override
  String get walletSendSyncNotRunning =>
      'Die Synchronisierung läuft nicht – Ihr verfügbares Guthaben kann sich nicht aktualisieren';

  @override
  String get walletSendWaitingForFunds =>
      'Synchronisierung läuft noch – Sie können senden, sobald Sie verfügbares Guthaben haben';

  @override
  String get walletSendNoSpendableYet => 'Noch kein verfügbares Guthaben';

  @override
  String get walletSendSyncUnavailable =>
      'Sie können senden, sobald die Synchronisierung fortgesetzt wird';

  @override
  String get walletSendTitle => 'Senden';

  @override
  String get walletSendUnavailable =>
      'Ihr Wallet ist momentan nicht bereit. Gehen Sie zurück und versuchen Sie es erneut.';

  @override
  String get walletSendWatchOnly =>
      'Dies ist ein Nur-Lese-Wallet. Es kann Guthaben anzeigen und Zahlungen empfangen, hält aber keine Sendeschlüssel – daher kann es nicht senden.';

  @override
  String get walletSendExpiredTitle =>
      'Diese Zahlungsanforderung ist abgelaufen';

  @override
  String get walletSendExpiredBody =>
      'Der Sendebildschirm brauchte länger als fünf Sekunden zum Öffnen, daher wurde der App mitgeteilt, dass nichts gesendet wurde. Diese Antwort ist endgültig: Diese Anforderung kann von hier aus nicht bezahlt werden. Um zu bezahlen, beginnen Sie erneut in der App.';

  @override
  String get walletSendFaultWatchOnly =>
      'Dies ist ein Nur-Lese-Wallet – es hält keine Sendeschlüssel, daher kann es nicht senden.';

  @override
  String walletSendAvailable(String amount) {
    return 'Verfügbar zum Senden: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Verfügbar zum Senden: $amount ZEC – Ihr Guthaben holt noch auf';
  }

  @override
  String get walletSendRecipientLabel => 'Empfängeradresse';

  @override
  String get walletSendRecipientHint =>
      'Zcash-Adresse (beginnt mit u, z oder t)';

  @override
  String get walletSendRecipientLocked =>
      'Empfänger kann hier nicht geändert werden';

  @override
  String get walletSendAmountLabel => 'Betrag (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Memo (optional)';

  @override
  String get walletSendMemoHint =>
      'Wird nur an geschirmte (private) Empfänger zugestellt';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Memos erfordern einen geschirmten Empfänger. Diese öffentliche Adresse kann keines empfangen.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Diese Zahlung trägt bereits eine Referenz der App und kann deshalb keine geschriebene Notiz aufnehmen.';

  @override
  String get walletSendMachineMemoTitle => 'Die App hängt eine Referenz an';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Sie gibt als Zweck an: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Sie bleibt bei der Transaktion und lässt sich später nicht entfernen. Die Wallet kann ihren Inhalt nicht prüfen.';

  @override
  String get walletSendRecipientShielded => 'Geschirmt · privat';

  @override
  String get walletSendRecipientTransparent => 'Öffentlich';

  @override
  String get walletSendRecipientInvalid =>
      'Das sieht nicht wie eine gültige Zcash-Adresse aus.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Diese Adresse gehört zu einem anderen Zcash-Netzwerk.';

  @override
  String get walletSendReviewButton => 'Zahlung prüfen';

  @override
  String get walletSendQueueButton => 'Für späteren Versand einreihen';

  @override
  String get walletSendQueueHint =>
      'Eine eingereihte Zahlung wartet unter „Gespeichert & ausstehend“, wo Sie sie senden oder abbrechen können. Die Netzwerkgebühr wird beim Senden berechnet.';

  @override
  String get walletSendPreparing => 'Ihre Zahlung wird vorbereitet…';

  @override
  String get walletSendSubmitting => 'Wird gesendet…';

  @override
  String get walletSendQueuing => 'Wird eingereiht…';

  @override
  String get walletSendReviewTitle => 'Zahlung bestätigen';

  @override
  String get walletSendTotalLabel => 'Gesamt';

  @override
  String get walletSendFeeLabel => 'Netzwerkgebühr';

  @override
  String get walletSendChangeLabel => 'Rückgeld';

  @override
  String get walletSendDeshieldTitle => 'Diese Zahlung ist nicht privat';

  @override
  String get walletSendDeshieldBody =>
      'Sie sendet an eine öffentliche Adresse, daher werden Betrag und Empfänger auf der Zcash-Blockchain öffentlich sichtbar.';

  @override
  String get walletSendPublicAckLabel =>
      'Ich verstehe, dass diese Zahlung öffentlich sein wird.';

  @override
  String get walletSendConfirmButton => 'Jetzt senden';

  @override
  String get walletSendBackButton => 'Zurück';

  @override
  String get walletSendSelfSendNote =>
      'Sie senden an Ihr eigenes Wallet. Die Netzwerkgebühr fällt trotzdem an.';

  @override
  String get walletSendLargeConfirmTitle => 'Großen Betrag senden?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Dies ist nahezu Ihr gesamtes Guthaben. Eine gesendete Zahlung kann nicht rückgängig gemacht werden.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Dies ist eine große Zahlung. Eine gesendete Zahlung kann nicht rückgängig gemacht werden.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Dies ist eine große Zahlung – nahezu Ihr gesamtes Guthaben. Eine gesendete Zahlung kann nicht rückgängig gemacht werden.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return '$amount senden';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Zurück';

  @override
  String get walletSendSentTitle => 'Zahlung gesendet';

  @override
  String get walletSendSentBody =>
      'Ihre Zahlung wurde an das Netzwerk übermittelt.';

  @override
  String get walletSendSavedTitle =>
      'Gespeichert – wir schließen den Versand ab';

  @override
  String get walletSendSavedBody =>
      'Ihre Zahlung konnte gerade nicht gesendet werden. Sie ist daher gespeichert, und Ihr Wallet sendet sie bei einer späteren Synchronisierung. Es geht nichts verloren.';

  @override
  String get walletSendKeptTitle => 'Gespeichert';

  @override
  String get walletSendKeptBody =>
      'Ihr Wallet hat diese Transaktion behalten, aber nicht zugesagt, sie von selbst zu senden. Unter „Aktivität“ sehen Sie, wo sie steht.';

  @override
  String get walletSendPartialBody =>
      'Ein Teil Ihrer Zahlung wurde gesendet; Ihr Wallet schließt den Rest bei einer späteren Synchronisierung ab. Es geht nichts verloren.';

  @override
  String get walletSendInMotionTitle => 'Zahlung in Bearbeitung';

  @override
  String get walletSendInMotionBody =>
      'Ihre Zahlung wurde gestartet und bewegt sich über eine einmalige Adresse, die Ihr Wallet kontrolliert. Senden Sie sie nicht erneut. Falls sie nicht abgeschlossen wird, können Sie das Guthaben über den Wallet-Bildschirm wiederherstellen.';

  @override
  String get walletSendAlreadyTitle => 'Bereits übermittelt';

  @override
  String get walletSendAlreadyBody =>
      'Diese Zahlung wurde bereits übermittelt – sie wird nicht zweimal gesendet.';

  @override
  String get walletSendFailedTitle =>
      'Zahlung konnte nicht abgeschlossen werden';

  @override
  String get walletSendFailedBody =>
      'Beim Abschließen dieser Zahlung ist etwas schiefgelaufen, es wurde nichts gesendet. Sie können es erneut versuchen.';

  @override
  String get walletSendTryAgain => 'Erneut versuchen';

  @override
  String get walletSendDone => 'Fertig';

  @override
  String get walletSendAnother => 'Weitere Zahlung senden';

  @override
  String get walletSendQueuedTitle => 'Zum Senden eingereiht';

  @override
  String get walletSendQueuedBody =>
      'Diese Zahlung ist gespeichert. Sie finden sie unter „Gespeichert & ausstehend“, wo Sie sie jetzt senden oder abbrechen können.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Nicht genügend verfügbares Guthaben – Sie haben $available ZEC, benötigt werden $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Das Zcash-Netzwerk wurde aktualisiert und diese App benötigt ein Update, bevor sie senden kann. Ihr Guthaben ist sicher.';

  @override
  String get walletSyncUpToDateLimited =>
      'So weit aufgeholt, wie diese Version lesen kann';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Das Zcash-Netzwerk wurde aktualisiert. Diese Version hat alles gescannt, was sie lesen kann, aber neuere Blöcke könnten Guthaben enthalten, das sie noch nicht anzeigen kann, und Memos zu neueren Zahlungen sind nicht verfügbar. Aktualisieren Sie die App, um alles zu sehen.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Aufgeholt, aber dieser Server bedient nicht jeden Pool';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Dieser Server verweigert, verschweigt oder meldet einen der geschützten Zcash-Pools falsch. Guthaben, das in diesem Pool eingegangen ist, kann über ihn nicht ausgegeben werden, und das angezeigte Guthaben ist eine Untergrenze. Wechseln Sie zu einem anderen Server, um es zu verwenden – das ist kein Verbindungsproblem.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: Dieser Server weigert sich, ihn bereitzustellen';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: Dieser Server hält einen Teil davon zurück';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: Dieser Server meldet ihn falsch';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: Ob dieser Server ihn bereitstellt, ist unbekannt';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Aufgeholt, aber dieser Server hinkt dem Netzwerk hinterher';

  @override
  String get walletSyncExplainEndpointBehind =>
      'Die Chain dieses Servers endet bei einem Block, den das Netzwerk bereits vor dem Build dieser App-Version hinter sich gelassen hat. Ihr Guthaben ist daher nur bis zu diesem Block aktuell: Neue Zahlungen an Sie werden möglicherweise noch nicht angezeigt, und eine von hier gesendete Zahlung kommt möglicherweise nicht an. Wechseln Sie zu einem anderen Server, um aufzuholen – das ist kein Verbindungsproblem.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Warten auf ein App-Update — Ihr Guthaben ist sicher und es wurde nichts gesendet.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Warten auf einen Server, der die Netzwerkversion meldet — wechseln Sie den Server. Ihr Guthaben ist sicher und es wurde nichts gesendet.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Warten auf einen Server, der die Netzwerkversion meldet. Falls Datum und Uhrzeit dieses Geräts falsch sind, korrigieren Sie sie zuerst – und wechseln Sie dann den Server. Ihr Guthaben ist sicher und es wurde nichts gesendet.';

  @override
  String get walletSyncUnverified =>
      'Aufgeholt, aber dieser Server meldet die Netzwerkversion nicht';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'Senden funktioniert noch etwa $hours Stunden — wechseln Sie dann den Server.',
      one:
          'Senden funktioniert noch etwa 1 Stunde — wechseln Sie dann den Server.',
      zero:
          'Senden funktioniert noch weniger als eine Stunde — wechseln Sie dann den Server.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'Senden funktioniert noch etwa $blocks Blöcke lang — wechseln Sie dann den Server.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Dieser Server hat die Netzwerkversion seit $blocks Blöcken nicht gemeldet, daher kann diese App nicht bestätigen, dass Senden sicher ist. Wechseln Sie zu einem anderen Server.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Dieser Server hat die Netzwerkversion seit einem Tag nicht gemeldet, daher kann diese App nicht bestätigen, dass Senden sicher ist. Falls Datum und Uhrzeit dieses Geräts falsch sind, korrigieren Sie sie zuerst – und wechseln Sie dann zu einem Server, der die Netzwerkversion meldet.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Dieser Server hat die Netzwerkversion noch nie gemeldet, daher kann diese App nicht bestätigen, dass Senden sicher ist. Wechseln Sie zu einem anderen Server.';

  @override
  String get walletSyncExplainUnverified =>
      'Dieser Server sagt nicht, auf welcher Version des Zcash-Netzwerks er läuft, daher kann diese App nicht bestätigen, dass eine von ihr signierte Zahlung akzeptiert wird. Ihr Guthaben ist aktuell. Wechseln Sie zu einem anderen Server – das ist kein Verbindungsproblem.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Dieser Server sagt nicht, auf welcher Version des Zcash-Netzwerks er läuft, daher kann diese App nicht bestätigen, dass eine von ihr signierte Zahlung akzeptiert wird. Er hat außerdem wiederholt Blöcke geliefert, die dieses Wallet danach zurücknehmen musste, daher ist Ihr Guthaben möglicherweise nicht aktuell. Wechseln Sie zu einem anderen Server – das ist kein Verbindungsproblem.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Dieser Server liefert außerdem wiederholt Blöcke, die dieses Wallet danach zurücknehmen muss – wechseln Sie den Server.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Ihr Guthaben holt noch auf – möglicherweise wird mehr verfügbar, während das Wallet synchronisiert.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC gehen noch ein und sind verfügbar, sobald die Wallet aufgeholt hat.';
  }

  @override
  String get walletSendFaultAmountEmpty =>
      'Geben Sie einen zu sendenden Betrag ein.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Geben Sie den Betrag als Zahl ein, zum Beispiel 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'ZEC hat höchstens 8 Dezimalstellen.';

  @override
  String get walletSendFaultAmountNotPositive =>
      'Geben Sie einen Betrag größer als null ein.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Dieser Betrag ist größer als das gesamte ZEC-Angebot.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Diese App begrenzt Sendungen momentan auf $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Das sieht nicht wie eine für dieses Netzwerk gültige Zcash-Adresse aus. Prüfen Sie sie und versuchen Sie es erneut.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Dieser Empfänger kann kein Memo empfangen. Entfernen Sie das Memo, oder senden Sie an eine geschirmte (private) Adresse.';

  @override
  String get walletSendFaultMemoTooLong =>
      'Ihr Memo ist zu lang. Kürzen Sie es und versuchen Sie es erneut.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Dieses Memo kann nicht gesendet werden. Entfernen Sie es und versuchen Sie es erneut.';

  @override
  String get walletSendFaultMemoConflict =>
      'Diese Zahlung konnte nicht gesendet werden – die App hat ihr zwei Notizen angehängt. Es wurde nichts gesendet.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Diese Adresse gehört zu einem anderen Netzwerk.';

  @override
  String get walletSendFaultUriInvalid =>
      'Diese Zahlung konnte nicht erstellt werden. Prüfen Sie Adresse und Betrag.';

  @override
  String get walletSendFaultNotSynced =>
      'Ihr Wallet ist noch nicht weit genug synchronisiert. Warten Sie, bis die Synchronisierung aufgeholt hat, oder reihen Sie dies für späteren Versand ein.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Ihr Wallet ist noch nicht weit genug synchronisiert. Warten Sie, bis die Synchronisierung aufgeholt hat.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Ihr Wallet ist noch nicht weit genug synchronisiert, und die Synchronisierung läuft momentan nicht. Prüfen Sie den Synchronisierungsstatus auf dem Wallet-Bildschirm.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Die Beträge sind während der Prüfung abgelaufen. Bitte prüfen Sie die Zahlung erneut.';

  @override
  String get walletSendFaultQueueFull =>
      'Zu viele Zahlungen warten auf den Versand. Lassen Sie diese zuerst senden und versuchen Sie es dann erneut.';

  @override
  String get walletSendFaultWalletBusy =>
      'Das Wallet ist momentan beschäftigt. Versuchen Sie es gleich noch einmal.';

  @override
  String get walletSendFaultStorageFull =>
      'Es ist nicht genügend freier Speicherplatz vorhanden, um diesen Versand abzuschließen. Geben Sie Speicherplatz frei und versuchen Sie es erneut.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Momentan sind zu viele einmalige Adressen in Verwendung. Einige werden möglicherweise frei, sobald Transaktionen bestätigt sind – das klärt sich aber nicht unbedingt von selbst. Ihr Guthaben ist sicher.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Diese Zahlung konnte nicht vorbereitet werden. Prüfen Sie die Angaben und versuchen Sie es erneut.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Diese Zahlung konnte gerade nicht vorbereitet werden. Versuchen Sie es gleich noch einmal.';

  @override
  String get walletSwapButton => 'Tauschen';

  @override
  String get walletSwapTitle => 'ZEC tauschen';

  @override
  String get walletSwapUnavailableWallet =>
      'Ihr Wallet ist momentan nicht bereit. Gehen Sie zurück und versuchen Sie es erneut.';

  @override
  String get walletSwapUnavailableOff =>
      'Tauschen ist momentan nicht verfügbar.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Dies ist ein Nur-Lese-Wallet – es kann nicht tauschen.';

  @override
  String get walletSwapDone => 'Fertig';

  @override
  String get walletSwapBackToWallet => 'Zurück zum Wallet';

  @override
  String walletSwapAvailable(String amount) {
    return 'Verfügbar zum Tauschen: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Verfügbar zum Tauschen: $amount ZEC – Ihr Guthaben holt noch auf';
  }

  @override
  String get walletSwapAssetLabel => 'Empfangenes Asset';

  @override
  String get walletSwapAmountLabel => 'Zu tauschender Betrag (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Zieladresse';

  @override
  String get walletSwapDestinationHint =>
      'Ihre Empfangsadresse auf der Ziel-Chain';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Ihre $chain-Empfangsadresse';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Eine $chain-Adresse – dorthin wird Ihr getauschtes Asset gesendet. Prüfen Sie sorgfältig, ob die Chain stimmt.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'QR-Code der Zieladresse scannen';

  @override
  String get walletSwapTargetAssetHint => 'Zu empfangendes Asset auswählen';

  @override
  String get walletSwapQuoteButton => 'Kurs abrufen';

  @override
  String get walletSwapQuoting => 'Kurs wird abgerufen…';

  @override
  String get walletSwapExecuting => 'Ihr Tausch wird gestartet…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Noch in Arbeit — der Tausch wird gestartet. Dies kann bis zu einer Minute dauern.';

  @override
  String get walletSwapReviewTitle => 'Tausch bestätigen';

  @override
  String get walletSwapYouSendLabel => 'Sie senden';

  @override
  String get walletSwapYouReceiveLabel => 'Sie erhalten mindestens';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Netzwerkgebühr';

  @override
  String get walletSwapNetworkFeeValue =>
      'Wird beim Senden der Einzahlung hinzugefügt';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Kurs noch etwa $time gültig — bestätigen Sie, bevor er abläuft.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Kurs noch weniger als eine Minute gültig — bestätigen Sie, bevor er abläuft.';

  @override
  String get walletSwapQuoteExpired =>
      'Dieser Kurs ist abgelaufen. Gehen Sie zurück und holen Sie einen neuen ein – der Kurs ist nicht mehr garantiert, und ein jetziger Versand riskiert eine Rückerstattung.';

  @override
  String get walletCountdownUnderMinute => 'weniger als eine Minute';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes Min.';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds Sek.';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '$hours Std. $minutes Min.';
  }

  @override
  String get walletSwapDeshieldTitle => 'Dieser Tausch ist nicht privat';

  @override
  String get walletSwapDeshieldBody =>
      'Beim Tausch aus ZEC heraus wird Ihr ZEC entschirmt – die Einzahlung ist eine öffentliche Transaktion, und die Seite des Anbieters ist auf dessen Netzwerk öffentlich.';

  @override
  String get walletSwapDiscloseTitle => 'Was der Tausch-Anbieter sieht';

  @override
  String get walletSwapDiscloseAmounts => 'Die Beträge auf beiden Seiten';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Dass dieses ZEC und das empfangene Asset zu einem Tausch gehören';

  @override
  String get walletSwapDiscloseDestination => 'Ihre Zieladresse';

  @override
  String get walletSwapDiscloseSource => 'Ihre Herkunftsadresse';

  @override
  String get walletSwapDiscloseIp =>
      'Ihre IP-Adresse (außer bei Nutzung von Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'Weitere Details dieses Tauschs';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'Die eigenen Transaktionen des Anbieters sind auf dessen Netzwerk öffentlich';

  @override
  String get walletSwapAckLabel =>
      'Ich verstehe, dass der Anbieter die oben genannten Informationen sieht.';

  @override
  String get walletSwapConfirmButton => 'Tausch starten';

  @override
  String get walletSwapBackButton => 'Zurück';

  @override
  String get walletSwapStatusPendingTitle => 'Tausch gestartet';

  @override
  String get walletSwapStatusCheckingTitle => 'Tauschstatus wird geprüft…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Ihr Wallet sendet die ZEC-Einzahlung an den Anbieter. Sind Sie kurz offline, wird sie automatisch gesendet, sobald Sie wieder online sind — das Sendefenster ist jedoch kurz, und schließt es vorher, endet der Tausch einfach, ohne dass etwas getauscht wird. Ihr ZEC bleibt Ihnen erhalten, es kann jedoch bis zu einer Stunde dauern, bis es wieder als verfügbar angezeigt wird.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'Wir warten auf den Eingang Ihrer Einzahlung. Falls Sie das Guthaben noch nicht aus Ihrem anderen Wallet gesendet haben, senden Sie es, bevor der Kurs abläuft.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Dieser Tausch wartet noch auf seine Einzahlung. Die Einzahlungsanweisungen sind auf diesem Gerät nicht mehr verfügbar — falls Sie das Guthaben bereits gesendet haben, wird es erkannt; falls nicht, lassen Sie diesen Tausch ablaufen und starten Sie einen neuen.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'Das Einzahlungsfenster endet $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'Das Einzahlungsfenster ist abgelaufen. Wurde die Einzahlung nicht rechtzeitig gesendet, endet der Tausch, und Ihr ZEC bleibt in Ihrem Wallet.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'Das Einzahlungsfenster ist abgelaufen. Falls Sie Ihre Einzahlung noch nicht gesendet haben, endet dieser Tausch einfach – holen Sie sich einen neuen Kurs, wenn Sie bereit sind.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Tausche laufen',
      one: 'Tausch läuft',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'Ihr ZEC ist auf dem Weg zum Anbieter.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'Wir warten darauf, dass Ihre Einzahlung beim Anbieter ankommt.';

  @override
  String get walletSwapInFlightRowGeneric => 'Ein Tausch läuft.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'Das Einzahlungsfenster ist abgelaufen — prüfen Sie den Status dieses Tauschs.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Dieser Tausch hat hier noch kein bestätigtes Ergebnis erreicht — öffnen Sie ihn, um nachzusehen. ZEC, das an dieses Wallet zurückkommt, erscheint nach einer Synchronisierung in Ihrem Guthaben.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Dieser Tausch hat hier noch kein bestätigtes Ergebnis erreicht — öffnen Sie ihn, um nachzusehen. ZEC, das dieser Tausch an dieses Wallet liefert, erscheint nach einer Synchronisierung in Ihrem Guthaben.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Tausch abgeschlossen.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Tausch erstattet.';

  @override
  String get walletSwapRowOutcomeFailed => 'Tausch nicht abgeschlossen.';

  @override
  String get walletSwapRemove => 'Entfernen';

  @override
  String get walletSwapRemoveTitle => 'Diesen Tausch aus der Liste entfernen?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Dies entfernt den Tausch nur aus dieser Liste — der Tausch selbst wird nicht storniert, und dieses Wallet verfolgt seine Rückerstattung dann nicht mehr. Später zurückerstattetes ZEC gehört weiterhin diesem Wallet; ein vollständiger erneuter Scan kann es finden.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Dies entfernt den Tausch nur aus dieser Liste — der Tausch selbst wird nicht storniert, und dieses Wallet verfolgt das eingehende ZEC dann nicht mehr. Später zugestelltes ZEC gehört weiterhin diesem Wallet; ein vollständiger erneuter Scan kann es finden. Wird der Tausch stattdessen erstattet, geht die Rückerstattung im gesendeten Asset zurück — außerhalb dieses Wallets.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Dies entfernt den Tausch nur aus dieser Liste — der Tausch selbst wird nicht storniert, und dieses Wallet verfolgt dann nicht mehr das ZEC, das noch von ihm eintrifft. Später eintreffendes ZEC gehört weiterhin diesem Wallet; ein vollständiger erneuter Scan kann es finden.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Dies entfernt den abgeschlossenen Tausch aus der Liste.';

  @override
  String get walletSwapRemoveCancel => 'Abbrechen';

  @override
  String get walletSwapRemoveConfirm => 'Entfernen';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Gestartet $time';
  }

  @override
  String get walletSwapViewSwap => 'Tausch anzeigen';

  @override
  String get walletSwapsInFlightError =>
      'Ihre laufenden Tausche konnten gerade nicht geladen werden.';

  @override
  String get walletSwapsInFlightRetry => 'Erneut versuchen';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Wird versucht…';

  @override
  String get walletSwapStartAnother => 'Weiteren Tausch starten';

  @override
  String get walletSwapStatusUnderTitle =>
      'Wartet auf die vollständige Einzahlung';

  @override
  String get walletSwapStatusUnderBody =>
      'Ein Teil der Einzahlung ist eingegangen. Der Rest wird noch abgeschlossen, oder der Anbieter erstattet zurück.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Ein Teil Ihrer Einzahlung ist eingegangen. Senden Sie den fehlenden Betrag vor Ablauf der Frist, oder der Anbieter erstattet den eingegangenen Betrag.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Erhalten: $received; $missing fehlen noch. Das Einzahlungsfenster endet: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Einzahlung erhalten';

  @override
  String get walletSwapStatusDetectedBody =>
      'Der Anbieter hat Ihre Einzahlung erhalten und wird den Tausch verarbeiten.';

  @override
  String get walletSwapStatusProcessingTitle => 'Ihr Tausch wird verarbeitet';

  @override
  String get walletSwapStatusProcessingBody =>
      'Der Anbieter schließt Ihren Tausch ab.';

  @override
  String get walletSwapStatusSuccessTitle => 'Tausch abgeschlossen';

  @override
  String get walletSwapStatusSuccessBody =>
      'Ihr Tausch wurde erfolgreich abgeschlossen.';

  @override
  String get walletSwapStatusRefundedTitle => 'Tausch erstattet';

  @override
  String get walletSwapStatusRefundedBody =>
      'Der Tausch wurde nicht abgeschlossen, daher hat der Anbieter das Guthaben an Ihre Rückerstattungsadresse zurückgeschickt.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'Der Tausch wurde nicht abgeschlossen, daher hat der Anbieter Ihr ZEC an dieses Wallet zurückgeschickt. Es kommt als ungeschirmtes Guthaben an und erscheint nach der nächsten Synchronisierung des Wallets in Ihrem Guthaben — das kann etwas dauern.';

  @override
  String get walletSwapStatusFailedTitle => 'Tausch fehlgeschlagen';

  @override
  String get walletSwapStatusFailedBody =>
      'Der Tausch konnte nicht abgeschlossen werden. Eingezahltes Guthaben wird auf Seiten des Anbieters verrechnet oder erstattet.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Tausch nicht gefunden';

  @override
  String get walletSwapStatusNotFoundBody =>
      'Der Anbieter hat keinen Eintrag mehr zu diesem Tausch — er ist höchstwahrscheinlich abgelaufen. Falls eine Einzahlung erfolgt ist, sollte der Anbieter sie an die Rückerstattungsadresse erstatten. Der Tausch bleibt in Ihrer Liste, und dieses Wallet verfolgt sein ZEC weiterhin, falls es noch ankommt; Sie können ihn jederzeit aus der Liste entfernen.';

  @override
  String get walletSwapStatusUnknownTitle => 'Status nicht verfügbar';

  @override
  String get walletSwapStatusUnknownBody =>
      'Der Status dieses Tauschs kann momentan nicht abgerufen werden.';

  @override
  String get walletSwapTrackingUnavailableTitle =>
      'Nachverfolgung nicht verfügbar';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'Tauschen ist deaktiviert, daher kann dies hier nicht nachverfolgt werden. Guthaben wird auf Seiten des Anbieters verrechnet oder erstattet.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'Tauschen ist hier deaktiviert, daher kann dieser Tausch momentan nicht nachverfolgt werden. Falls er erstattet wurde, kommt das ZEC zu diesem Wallet zurück — es erscheint in Ihrem Guthaben, sobald Tauschen wieder aktiviert ist und das Wallet synchronisiert.';

  @override
  String get walletSwapTrackingError =>
      'Dieser Tausch konnte nicht nachverfolgt werden.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Die Nachverfolgung für diesen Tausch konnte nicht geöffnet werden. Der Tausch selbst läuft möglicherweise trotzdem weiter — eingezahltes Guthaben wird auf Seiten des Anbieters verrechnet oder erstattet.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Geben Sie die Adresse ein, an der Sie das getauschte Asset empfangen möchten.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Diese Zieladresse ist für dieses Asset ungültig. Prüfen Sie sie und versuchen Sie es erneut.';

  @override
  String get walletSwapFaultExpired =>
      'Dieser Kurs ist abgelaufen. Holen Sie einen neuen Kurs ein, um fortzufahren.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'Der Preis des Anbieters hat Ihr Limit überschritten, daher wurde der Tausch gestoppt, bevor etwas bewegt wurde. Versuchen Sie es erneut.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'Das Slippage-Limit ist für einen sicheren Tausch zu hoch. Versuchen Sie es erneut.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'Der Tausch-Anbieter ist momentan nicht verfügbar. Versuchen Sie es gleich noch einmal.';

  @override
  String get walletSwapFaultConnection =>
      'Der Tausch-Dienst war nicht erreichbar. Bitte prüfen Sie Ihre Internetverbindung und versuchen Sie es erneut.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'Der Tausch-Anbieter hat eine unerwartete Antwort geliefert, daher wurde der Tausch gestoppt. Versuchen Sie es erneut.';

  @override
  String get walletSwapFaultSwapOff => 'Tauschen ist momentan deaktiviert.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Ihre Einzahlung konnte nicht gesendet werden, daher hat nichts Ihr Wallet verlassen. Holen Sie einen neuen Kurs ein, um es erneut zu versuchen.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Ein Tausch läuft bereits. Sie können einen neuen starten, nachdem dieser vollständig verrechnet ist oder sein Kurs abläuft — das kann eine Weile dauern.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Dieses Wallet kann noch keine Rückerstattungsadresse einrichten — das bedeutet in der Regel nur, dass die erste Synchronisierung noch nicht abgeschlossen ist. Warten Sie, bis die Synchronisierung abgeschlossen ist, und versuchen Sie es dann erneut.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Dieses Wallet kann noch keine Empfangsadresse für diesen Tausch einrichten — das bedeutet in der Regel nur, dass die erste Synchronisierung noch nicht abgeschlossen ist. Warten Sie, bis die Synchronisierung abgeschlossen ist, und versuchen Sie es dann erneut.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'Der Tausch konnte nicht rechtzeitig gestartet werden — die Verbindung war möglicherweise langsam, oder das Wallet war ausgelastet. Holen Sie einen neuen Kurs ein und versuchen Sie es erneut.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'Das Wallet ist gerade kurz ausgelastet. Versuchen Sie es erneut.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Dieser Kurs stimmt nicht mit dem überein, den Ihr Wallet ausgegeben hat, daher wurde nichts gesendet. Holen Sie einen neuen Kurs ein und versuchen Sie es erneut.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Dieser Tausch benötigt etwa $needed ZEC einschließlich der Netzwerkgebühr, aber nur $spendable ZEC ist derzeit verfügbar.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Diese App begrenzt Tausche momentan auf $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Dieser Tausch benötigt etwa $needed ZEC einschließlich der Netzwerkgebühr, aber nur $spendable ZEC ist derzeit verfügbar. Ihr Guthaben holt noch auf — möglicherweise wird bald mehr verfügbar.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'Das Wallet konnte diesen Tausch nicht sicher erfassen, daher wurde nichts bewegt. Versuchen Sie es erneut.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Diese Tauschanfrage konnte nicht verarbeitet werden. Holen Sie einen neuen Kurs ein und versuchen Sie es erneut.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Kein Tauschkurs konnte abgerufen werden. Prüfen Sie die Angaben und versuchen Sie es erneut.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Ihr Wallet ist momentan nicht bereit. Gehen Sie zurück und versuchen Sie es erneut.';

  @override
  String get walletSwapDirectionBuy => 'ZEC kaufen';

  @override
  String get walletSwapDirectionSell => 'ZEC verkaufen';

  @override
  String get walletSwapRefundLabel => 'Ihre Rückerstattungsadresse';

  @override
  String get walletSwapRefundHint =>
      'Wohin Ihre Coins zurückgehen, falls der Tausch fehlschlägt';

  @override
  String get walletSwapRefundHelper =>
      'Auf der Chain, von der Sie senden – keine Zcash-Adresse.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Ihre $chain-Rückerstattungsadresse';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Eine $chain-Adresse – dorthin gehen Ihre Coins zurück, falls der Tausch fehlschlägt. Keine Zcash-Adresse.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'Über Ihre Rückerstattungsadresse';

  @override
  String get walletSwapRefundInfoBody =>
      'Kann der Tausch nicht abgeschlossen werden, sendet der Anbieter Ihre Coins an diese Adresse auf der Chain zurück, von der Sie bezahlt haben. Geben Sie eine Adresse an, die Sie kontrollieren – das Wallet kann eine fremde Adresse nicht für Sie prüfen, überprüfen Sie sie also sorgfältig selbst.';

  @override
  String get walletSwapRefundScanTooltip =>
      'QR-Code der Rückerstattungsadresse scannen';

  @override
  String get walletSwapScanTitle => 'Adresse scannen';

  @override
  String get walletSwapScanInstruction =>
      'Richten Sie Ihre Kamera auf den QR-Code der Adresse.';

  @override
  String get walletSwapScanManualEntry => 'Manuell eingeben';

  @override
  String get walletSwapScanCancel => 'Abbrechen';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Kamera nicht verfügbar. Geben Sie die Adresse unten manuell ein.';

  @override
  String get walletSwapSourceAssetLabel => 'Asset, aus dem getauscht wird';

  @override
  String get walletSwapSourceAssetHint => 'Asset auswählen';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Zu sendender Betrag ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Zu sendender Betrag';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol auf $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Asset zum Tauschen auswählen';

  @override
  String get walletSwapPickerTitleReceive => 'Zu empfangendes Asset auswählen';

  @override
  String get walletSwapPickerStale =>
      'Asset-Liste konnte nicht aktualisiert werden – zeigt die zuletzt bekannte Liste.';

  @override
  String get walletSwapPickerEmpty =>
      'Momentan sind keine Assets zum Tauschen verfügbar. Versuchen Sie es später erneut.';

  @override
  String get walletSwapPickerSearchHint => 'Nach Name oder Chain suchen';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Keine Assets stimmen mit „$query“ überein.';
  }

  @override
  String get walletSwapPickerError =>
      'Die Asset-Liste konnte nicht geladen werden. Prüfen Sie Ihre Verbindung und versuchen Sie es erneut.';

  @override
  String get walletSwapPickerRetry => 'Erneut versuchen';

  @override
  String get walletSwapSlippageLabel => 'Slippage-Toleranz';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Benutzerdefiniert';

  @override
  String get walletSwapSlippageCustomLabel => 'Benutzerdefinierte Slippage';

  @override
  String get walletSwapSlippageMayFail =>
      'Sehr niedrig – der Tausch kann fehlschlagen, wenn sich der Preis bewegt.';

  @override
  String get walletSwapSlippageNormal => 'Eine sichere Toleranz.';

  @override
  String get walletSwapSlippageRisky =>
      'Hoch – Sie könnten spürbar weniger als den Kurswert erhalten.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Zu hoch – der Tausch wird abgelehnt. Verringern Sie sie auf 10 % oder weniger.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Sie erhalten mindestens $zec ZEC – Ihre $slippage%-Slippage-Untergrenze. Der endgültige Betrag fällt nicht darunter.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'Sie erhalten ZEC auf Ihre eigene Adresse';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Bis Sie es schirmen – ein Tipp, wozu Sie bei Ankunft aufgefordert werden – ist der erhaltene Betrag kurzzeitig öffentlich und auf der Chain sichtbar. Eine kleine Lieferung kann öffentlich bleiben, bis sich mehr ansammelt.';

  @override
  String get walletSwapRefundVerifyTitle => 'Rückerstattungsadresse prüfen';

  @override
  String get walletSwapRefundVerifyBody =>
      'Prüfen Sie sie Zeichen für Zeichen – hierhin gehen Ihre Coins zurück, falls der Tausch fehlschlägt. Das Wallet kann eine fremde Adresse nicht für Sie verifizieren.';

  @override
  String get walletSwapRefundVerifyAck =>
      'Ich habe geprüft, dass meine Rückerstattungsadresse korrekt ist.';

  @override
  String get walletSwapPayoutVerifyTitle => 'Empfangsadresse prüfen';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Prüfen Sie sie Zeichen für Zeichen – hierhin erhalten Sie $asset. Das Wallet kann eine fremde Adresse nicht für Sie verifizieren.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'Ich habe geprüft, dass meine Empfangsadresse korrekt ist.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'Tauschen ist hier deaktiviert. Bereits unterwegs befindliches ZEC erscheint nach Ihrer nächsten Synchronisierung in Ihrem Wallet.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Geben Sie den Betrag ein, den Sie tauschen möchten.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Geben Sie Ihre Rückerstattungsadresse auf der Herkunfts-Chain ein.';

  @override
  String get walletSwapDepositTitle => 'Zahlung senden';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Senden Sie genau $amount $asset auf $chain an die untenstehende Adresse.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Senden Sie genau den angegebenen Betrag. Wird weniger gesendet oder erst nach Ablauf des Zeitfensters, erstattet der Anbieter an Ihre Rückerstattungsadresse zurück.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Einzahlungsfenster: noch $time';
  }

  @override
  String get walletSwapDepositExpired =>
      'Dieses Einzahlungsfenster ist geschlossen. Senden Sie jetzt kein Guthaben – starten Sie einen neuen Tausch. Falls Sie bereits gesendet haben, sollte der Anbieter an Ihre Rückerstattungsadresse zurückerstatten.';

  @override
  String get walletSwapDepositQrLabel => 'QR-Code der Einzahlungsadresse';

  @override
  String get walletSwapDepositAddressLabel => 'Einzahlungsadresse';

  @override
  String get walletSwapDepositCopy => 'Einzahlungsadresse kopieren';

  @override
  String get walletSwapDepositCopied => 'Einzahlungsadresse kopiert';

  @override
  String get walletSwapDepositMemoRequired =>
      'Diese Einzahlung erfordert ein Memo/Tag';

  @override
  String get walletSwapDepositMemoWarning =>
      'Sie MÜSSEN dieses exakte Memo bei Ihrer Einzahlung angeben. Ohne Memo oder mit falschem Memo zu senden kann zum dauerhaften Verlust Ihres Guthabens führen.';

  @override
  String get walletSwapDepositMemoLabel => 'Einzahlungs-Memo/Tag';

  @override
  String get walletSwapDepositMemoCopy => 'Memo kopieren';

  @override
  String get walletSwapDepositMemoCopied => 'Memo kopiert';

  @override
  String get walletSwapDepositSent => 'Ich habe das Guthaben gesendet';

  @override
  String get walletSwapDepositBackTitle => 'Diesen Bildschirm verlassen?';

  @override
  String get walletSwapDepositBackBody =>
      'Dies bricht Ihren Tausch nicht ab – er läuft im Hintergrund weiter. Sie benötigen jedoch die Einzahlungsadresse zum Bezahlen, kopieren Sie sie also zuerst, falls noch nicht geschehen.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Dies bricht Ihren Tausch nicht ab – er läuft im Hintergrund weiter. Das Einzahlungsfenster ist geschlossen. Senden Sie jetzt kein Guthaben mehr an die Einzahlungsadresse. Falls Sie bereits gesendet haben, sollte der Anbieter an Ihre Rückerstattungsadresse zurückerstatten.';

  @override
  String get walletSwapDepositBackStay => 'Bleiben';

  @override
  String get walletSwapDepositBackLeave => 'Verlassen';

  @override
  String get walletReceive => 'Empfangen';

  @override
  String get walletReceiveSubtitle =>
      'Teilen Sie diese Adresse, um ZEC zu empfangen. Sie kann sicher öffentlich geteilt werden.';

  @override
  String get walletReceiveCopy => 'Adresse kopieren';

  @override
  String get walletReceiveCopied => 'Adresse kopiert';

  @override
  String get walletReceiveUnavailable => 'Ihr Wallet ist noch nicht bereit.';

  @override
  String get walletReceiveError =>
      'Ihre Adresse konnte nicht geladen werden. Bitte versuchen Sie es erneut.';

  @override
  String get walletReceivePreparing => 'Ihre Adresse wird vorbereitet…';

  @override
  String get walletReceivePreparingHint =>
      'Ihr Wallet bereitet diese Adresse auf Ihrem Gerät vor — dies kann einen Moment dauern, wenn das Wallet mit anderer Arbeit beschäftigt ist.';

  @override
  String get walletReceiveRetry => 'Erneut versuchen';

  @override
  String get walletReceiveQrLabel => 'QR-Code Ihrer Empfangsadresse';

  @override
  String get walletReceiveTypeShielded => 'Geschirmt';

  @override
  String get walletReceiveTypeTransparent => 'Öffentlich';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Teilen Sie diese öffentliche Adresse, um ZEC von einem Absender zu empfangen, der nicht an eine geschirmte Adresse zahlen kann.';

  @override
  String get walletReceiveTransparentWarning =>
      'Dies ist eine öffentliche Adresse: Sie ist auf der Chain sichtbar und verknüpft bei Wiederverwendung Ihre Zahlungen. Bevorzugen Sie Ihre geschirmte Adresse; schirmen Sie dieses Guthaben nach dem Empfang.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'QR-Code Ihrer öffentlichen Empfangsadresse';

  @override
  String get walletReceiveFreshAddress => 'Neue Adresse verwenden';

  @override
  String get walletReceiveFreshCaption =>
      'Neue Adresse — kann nicht mit Ihren anderen Adressen verknüpft werden. Zahlungen daran gehen weiterhin in dieses Wallet ein, und Ihre bisherigen Adressen funktionieren nach wie vor. Sie wird hier nicht noch einmal angezeigt — kopieren Sie sie jetzt.';

  @override
  String get walletReceiveFreshError =>
      'Neue Adresse konnte nicht erstellt werden. Versuchen Sie es erneut.';

  @override
  String get walletReceiveFreshBusy =>
      'Das Wallet ist gerade beschäftigt. Versuchen Sie es in einem Moment erneut mit der neuen Adresse.';

  @override
  String get walletReceiveShare => 'Teilen';

  @override
  String get walletReceiveRequestAmount => 'Betrag anfordern';

  @override
  String get walletReceiveRequestAmountLabel => 'Betrag (optional)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Sie wird hier nicht noch einmal angezeigt — kopieren Sie sie jetzt.';

  @override
  String get walletSecurityMenuItem => 'Sicherheit…';

  @override
  String get securityTitle => 'Sicherheit';

  @override
  String get securityUnavailableBody =>
      'Die Sicherheitseinstellungen des Wallets werden von dieser App verwaltet, nicht vom Wallet selbst.';

  @override
  String get securityCustodySectionTitle => 'Schlüsselverwahrung';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (Hardware)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (Hardware)';

  @override
  String get securityCustodyTierTee => 'Hardware-Schlüsselspeicher (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Software-Schlüsselspeicher';

  @override
  String get securityCustodyTierKeychain =>
      'Schlüsselbund (softwareverschlüsselt)';

  @override
  String get securityCustodyTierNone => 'Kein Hardware-Schlüsselspeicher';

  @override
  String get securityCustodyTierUnknown => 'Unbekannt';

  @override
  String get securityCustodyHardwareKey =>
      'Der Schlüssel, der dieses Wallet sperrt, liegt in der sicheren Hardware dieses Geräts und wird mit dem Wallet gelöscht.';

  @override
  String get securityCustodyBestEffort =>
      'Das Löschen entfernt Ihre Schlüssel nach bestem Bemühen; ein kurzes forensisches Wiederherstellungsfenster kann bestehen bleiben, bis das Gerät den Speicher zurückfordert. Nutzen Sie für volle Sicherheit zusätzlich die Funktion „Gesamten Inhalt löschen“ Ihres Geräts.';

  @override
  String get securityCustodyProbeError =>
      'Verwahrungsstatus konnte nicht gelesen werden. Gehen Sie zurück und versuchen Sie es erneut.';

  @override
  String get securityDeleteWalletButton => 'Wallet löschen';

  @override
  String get securityDeleteWalletSubtitle =>
      'Löscht dieses Wallet und seinen Schlüssel von diesem Gerät. Ihr Guthaben bleibt auf der Chain erhalten und ist über Ihre Wiederherstellungsphrase wiederherstellbar.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Löscht dieses Wallet und seinen Schlüssel von diesem Gerät. Es besitzt keine Sendeschlüssel – hier gibt es nichts zu sichern, und Sie können es jederzeit mit seinem Prüfschlüssel wieder hinzufügen.';

  @override
  String get securityDeleteDialogTitle => 'Dieses Wallet löschen?';

  @override
  String get securityDeleteDialogBody =>
      'Dies entfernt das Wallet und seinen Schlüssel von diesem Gerät. Stellen Sie sicher, dass Sie Ihre Wiederherstellungsphrase gesichert haben – sie ist der EINZIGE Weg, Ihr Guthaben wiederherzustellen.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Dies entfernt das Wallet und seinen Schlüssel von diesem Gerät. Es besitzt keine Sendeschlüssel – es muss nichts gesichert werden, und Sie können es später mit seinem Prüfschlüssel wieder hinzufügen.';

  @override
  String get securityDeleteDialogConfirm => 'Löschen';

  @override
  String get securityDeleteDialogCancel => 'Abbrechen';

  @override
  String get securityDeleteFailedSnack =>
      'Wallet konnte nicht gelöscht werden – Ihr Wallet ist unverändert. Versuchen Sie es erneut.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Schließen Sie zuerst den Serverwechsel ab – er wird innerhalb von $seconds Sekunden abgeschlossen oder beendet. Versuchen Sie dann erneut, das Wallet zu löschen.';
  }

  @override
  String get walletParkedTitle => 'Gespeichert & ausstehend';

  @override
  String get walletParkedSubtitle =>
      'Diese Zahlungen wurden noch nicht gesendet. Ihre Beträge sind weiterhin Teil Ihres Guthabens.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Diese Zahlungen wurden noch nicht gesendet. Ihre Beträge sind weiterhin Teil Ihres Guthabens — außer bei denen, die Ihr Wallet gerade sendet; deren Betrag könnte bereits reserviert sein.';

  @override
  String get walletParkedCancel => 'Abbrechen';

  @override
  String get walletParkedPausedHint =>
      'Pausiert – diese Zahlung wird nicht von selbst gesendet. Ihr Guthaben ist sicher. Senden Sie sie jetzt, oder brechen Sie sie ab.';

  @override
  String get walletParkedRetryStale =>
      'Diese Zahlung wartet nicht mehr – prüfen Sie Ihre ausstehenden Zahlungen und Ihre Aktivität.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Diese Zahlung wartet nicht mehr – Ihr Wallet sendet sie möglicherweise bereits. Prüfen Sie „Gespeichert & ausstehend“ und Ihre Aktivität.';

  @override
  String get walletReclaimExplainer =>
      'Senden über einmalige Adressen ist derzeit blockiert. Sie können es wieder freischalten – dabei wird ein kleiner Betrag zwischen Ihren eigenen Adressen verschoben und wieder zurückgeführt.';

  @override
  String get walletReclaimButton => 'Senden freischalten';

  @override
  String get walletReclaimInProgress => 'Wird freigeschaltet…';

  @override
  String get walletReclaimConfirmTitle =>
      'Senden über einmalige Adressen wieder freischalten?';

  @override
  String get walletReclaimConfirmBody =>
      'Dies verschiebt einen kleinen Betrag zwischen Ihren eigenen Adressen, um Senden über einmalige Adressen freizuschalten, und führt ihn danach zurück. Es kostet ein paar Netzwerkgebühren. Sobald dies bestätigt ist, holen Sie sich den verschobenen Betrag über „Jetzt wiederherstellen“ zurück.';

  @override
  String get walletReclaimConfirmCancel => 'Nicht jetzt';

  @override
  String get walletReclaimConfirmAction => 'Freischalten';

  @override
  String get walletReclaimStarted =>
      'Freischalten gestartet. Sobald dies bestätigt ist, senden Sie die pausierte Zahlung und holen Sie sich anschließend den verschobenen Betrag über „Jetzt wiederherstellen“ zurück.';

  @override
  String get walletReclaimNothing => 'Momentan nichts freizuschalten.';

  @override
  String get walletReclaimNotBroadcast =>
      'Es konnte nicht bestätigt werden, ob die Transaktion das Netzwerk erreicht hat. Sie könnte dennoch durchgegangen sein. Warten Sie einen Moment, bevor Sie es erneut versuchen.';

  @override
  String get walletReclaimNeedsFunds =>
      'Sie benötigen etwas geschirmtes ZEC, um Senden freizuschalten.';

  @override
  String get walletReclaimFailed =>
      'Momentan nicht freischaltbar. Ihr Guthaben ist unverändert. Versuchen Sie es erneut.';

  @override
  String get walletReclaimUnknown =>
      'Freischalten beendet. Prüfen Sie Ihre Sendevorgänge über einmalige Adressen, und holen Sie sich einen eventuell verschobenen Betrag über „Jetzt wiederherstellen“ zurück.';

  @override
  String get walletParkedError =>
      'Ausstehende Zahlungen konnten momentan nicht geladen werden.';

  @override
  String get walletParkedErrorRetry => 'Erneut versuchen';

  @override
  String get walletParkedErrorRetryInProgress => 'Wird versucht…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Diese ausstehende Zahlung abbrechen?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Dies verwirft die gespeicherte Zahlung. Sie wurde nicht gesendet, daher verlässt nichts Ihr Wallet – dies kann jedoch nicht rückgängig gemacht werden.';

  @override
  String get walletParkedCancelConfirmKeep => 'Behalten';

  @override
  String get walletParkedCancelConfirmDiscard => 'Zahlung verwerfen';

  @override
  String get walletParkedCancelDone => 'Ausstehende Zahlung abgebrochen.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Diese Zahlung ist möglicherweise bereits unterwegs – prüfen Sie Ihre Aktivität.';

  @override
  String get walletParkedCancelFailed =>
      'Momentan nicht abbrechbar. Ihre Zahlung ist unverändert. Versuchen Sie es erneut.';

  @override
  String get walletRecoverNow => 'Jetzt wiederherstellen';

  @override
  String get walletRecoverConfirmTitle =>
      'In Ihr geschirmtes Guthaben wiederherstellen?';

  @override
  String get walletRecoverConfirmBody =>
      'Dies prüft Ihre einmaligen Adressen und verschiebt alles Gefundene in Ihr privates, geschirmtes Guthaben. Es kann jederzeit gefahrlos erneut ausgeführt werden.';

  @override
  String get walletRecoverConfirmCancel => 'Nicht jetzt';

  @override
  String get walletRecoverConfirmAction => 'Wiederherstellen';

  @override
  String get walletRecoverInProgress => 'Wird wiederhergestellt…';

  @override
  String walletRecoverDone(String amount) {
    return '$amount wird in Ihr geschirmtes Guthaben wiederhergestellt.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return '$amount wird wiederhergestellt – ein Teil des Guthabens benötigt noch einen weiteren Versuch.';
  }

  @override
  String get walletRecoverRetry =>
      'Ein Teil des Guthabens benötigt noch einen weiteren Versuch – führen Sie die Wiederherstellung erneut aus.';

  @override
  String get walletRecoverTruncated =>
      'Noch nicht jede einmalige Adresse wurde geprüft – führen Sie es erneut aus, um den Rest zu prüfen.';

  @override
  String get walletRecoverNothing => 'Momentan nichts wiederherzustellen.';

  @override
  String get walletRecoverFailed =>
      'Momentan nicht wiederherstellbar. Ihr Guthaben ist unverändert. Versuchen Sie es erneut.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount gespeichert & ausstehend · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Die am $time gespeicherte Zahlung über $amount abbrechen';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount pausiert · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount wird zum Senden vorbereitet · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Ihr Wallet bereitet diese Zahlung vor — der Betrag könnte bereits reserviert sein. Ihr Guthaben ist sicher. Falls dies nicht abgeschlossen wird, kehrt sie von selbst in die Liste zurück.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Ihr Wallet bereitet diese Zahlung vor — der Betrag könnte bereits reserviert sein. Ihr Guthaben ist sicher, aber sie kann erst abgeschlossen werden, wenn Ihr Wallet wieder synchronisiert.';

  @override
  String get walletParkedSendNow => 'Jetzt senden';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Die am $time gespeicherte Zahlung über $amount wird gesendet';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Die am $time gespeicherte Zahlung über $amount jetzt senden';
  }

  @override
  String get walletParkedSendNowInProgress => 'Wird gesendet…';

  @override
  String get walletParkedAuthorizeSent => 'Ihre Zahlung wird jetzt gesendet.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Ihre Zahlung wird jetzt gesendet. Sollte sie nicht durchgehen, kann Ihr Wallet sie erst abschließen, wenn es wieder synchronisiert.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Noch nicht bereit zum Senden. Ihre Zahlung ist gespeichert und unverändert.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Noch nicht bereit zum Senden. Ihre Zahlung ist gespeichert und nicht mehr pausiert – versuchen Sie es später noch einmal mit „Jetzt senden“, oder brechen Sie sie ab.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Momentan nicht sendbar. Ihre Zahlung ist unverändert. Versuchen Sie es erneut.';

  @override
  String get walletTransparentFundsMenuItem => 'Öffentliches Guthaben…';

  @override
  String get walletTransparentFundsTitle => 'Öffentliches Guthaben';

  @override
  String get walletTransparentFundsIntro =>
      'Öffentliches Guthaben ist auf der Blockchain öffentlich sichtbar – der Betrag, die Adressen und der Verlauf der Münzen.';

  @override
  String get walletExpertToggleLabel => 'Erweitert: Öffentliches Guthaben';

  @override
  String get walletExpertToggleDescription =>
      'Experteneinstellungen anzeigen, um öffentliches Guthaben zu halten und das automatische Schirmen auszuschalten.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Experteneinstellungen anzeigen, um öffentliches Guthaben zu halten.';

  @override
  String get walletAutoShieldToggleLabel => 'Automatisch schirmen';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Sobald Ihr öffentliches Guthaben $minZec ZEC erreicht, wird es automatisch in Ihr geschirmtes Guthaben verschoben. Ist dies ausgeschaltet, bleibt öffentliches Guthaben öffentlich sichtbar, bis Sie es selbst schirmen.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Die Einstellung konnte nicht gespeichert werden. Versuchen Sie es erneut.';

  @override
  String get walletAutoShieldIncomplete =>
      'Das automatische Schirmen wurde nicht abgeschlossen – dieses Guthaben ist weiterhin öffentlich sichtbar. Sie können es jetzt schirmen.';

  @override
  String get walletSendPrivacyShielded =>
      'Geschirmte Zahlung – Betrag und Empfänger bleiben auf der Chain privat.';

  @override
  String get walletSendPrivacyTransparent =>
      'Öffentliche Zahlung – Betrag und Adressen sind auf der Blockchain sichtbar.';

  @override
  String get walletActivityPublicBadge =>
      'Öffentlich sichtbar auf der Blockchain';

  @override
  String get walletShieldWalletEnded =>
      'Die Wallet-Sitzung wurde beendet. Schließen und erneut öffnen, um es noch einmal zu versuchen.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Neues öffentliches Guthaben wird automatisch in Ihr privates Guthaben geschirmt, sobald es $minZec ZEC erreicht.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'Automatisches Schirmen ist deaktiviert — öffentliches Guthaben bleibt öffentlich sichtbar, bis Sie es schirmen.';

  @override
  String get walletMoveAutoShieldNote =>
      'Automatisches Schirmen ist aktiviert: Sobald diese Mittel eingehen, werden sie automatisch wieder geschirmt (gegen eine weitere Gebühr). Um sie öffentlich zu halten, deaktivieren Sie zuerst das automatische Schirmen unter Öffentliches Guthaben.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Nach dieser Verschiebung beträgt Ihr öffentliches Guthaben $amount ZEC — weniger als die $floor ZEC, die zum erneuten Schirmen nötig sind. Es bleibt öffentlich, bis mehr eingeht.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Sie verschieben zu Ihrer eigenen öffentlichen Adresse. Diese Verschiebung bleibt dauerhaft im öffentlichen Register sichtbar.';

  @override
  String get walletTxDetailVisibility => 'Sichtbarkeit';

  @override
  String get walletTransparentFundsAutoDenied =>
      'Automatisches Schirmen ist für diese Sitzung pausiert — es wurde nicht genehmigt. Sie können weiterhin manuell schirmen.';

  @override
  String get walletDeepScanMenuItem => 'Ältere Tauschadressen prüfen…';

  @override
  String get walletMenuSyncNotRunningHint =>
      'Die Synchronisierung läuft momentan nicht.';

  @override
  String get walletDeepScanTitle => 'Ältere Tauschadressen prüfen';

  @override
  String get walletDeepScanBody =>
      'Wenn Sie dieses Wallet wiederhergestellt haben und es früher häufig für Tausche verwendet wurde, kann das Guthaben aus den ältesten Tauschvorgängen einen zusätzlichen Schritt zum Auffinden benötigen. Diese Prüfung sucht danach — alles Gefundene erscheint in Ihrem Guthaben, sobald Ihr Wallet synchronisiert.';

  @override
  String get walletDeepScanCoverage =>
      'Ihre älteren Tauschadressen wurden bis hierher geprüft. Falls Guthaben aus einem alten Tausch noch fehlt, prüfen Sie noch ältere Adressen.';

  @override
  String get walletDeepScanCoveragePending =>
      'Der aktuelle Bereich wird noch geprüft — alles Gefundene erscheint in Ihrem Guthaben. Das kann eine Weile dauern.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Sucht nach Guthaben aus den ältesten Tauschvorgängen Ihres Wallets.';

  @override
  String get walletDeepScanCheckButton => 'Ältere Adressen prüfen';

  @override
  String get walletDeepScanCheckDeeperButton => 'Noch ältere Adressen prüfen';

  @override
  String get walletDeepScanChecking => 'Wird geprüft…';

  @override
  String get walletDeepScanClose => 'Schließen';

  @override
  String get walletDeepScanTorHint =>
      'Sie sind derzeit nicht über Tor verbunden. Für mehr Privatsphäre sollten Sie erwägen, mit der Prüfung zu warten, bis Tor aktiv ist.';

  @override
  String get walletDeepScanRescanBusy =>
      'Sie können ältere Tauschadressen prüfen, sobald der erneute Scan abgeschlossen ist.';

  @override
  String get walletDeepScanRan =>
      'Ältere Tauschadressen werden geprüft — alles Gefundene erscheint in Ihrem Guthaben.';

  @override
  String get walletDeepScanFailed =>
      'Die Prüfung konnte nicht gestartet werden. Es hat sich nichts geändert — versuchen Sie es erneut.';

  @override
  String get walletDeepScanSlow =>
      'Das dauert länger als gewöhnlich. Falls Ihre älteren Tauschadressen geprüft wurden, erscheint alles Gefundene in Ihrem Guthaben — schauen Sie in Kürze noch einmal nach.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'Der Tausch ist derzeit deaktiviert, daher kann dies nicht ausgeführt werden. Versuchen Sie es erneut, sobald der Tausch verfügbar ist.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Der letzte Bereich wird noch geprüft — das kann jedoch bis zu zwei Tagen dauern, meist deutlich weniger. Er endet von selbst; prüfen Sie später erneut.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'Die Privatsphäre dieser Verbindung kann noch nicht überprüft werden. Für mehr Privatsphäre sollten Sie erwägen, mit der Prüfung zu warten, bis Tor aktiv ist.';

  @override
  String get walletDeepScanBannerChecking =>
      'Ältere Tauschadressen werden noch geprüft — alles Gefundene erscheint in Ihrem Guthaben.';

  @override
  String get walletRescanSwapPointer =>
      'Suchen Sie Guthaben aus einem alten Tausch? Ein erneuter Scan findet das nicht — verwenden Sie stattdessen „Ältere Tauschadressen prüfen“.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Ein Wallet wiederhergestellt, das Tausche genutzt hat?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Wenn dieses Wallet eine sehr lange Tauschhistorie hatte, kann das Guthaben aus den ältesten Tauschvorgängen einen zusätzlichen Schritt zum Auffinden benötigen. Bei den meisten Wallets ist nichts weiter nötig.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Jetzt prüfen';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Schließen';

  @override
  String walletTorHostPath(String transport) {
    return 'Über den privaten Pfad Ihrer App ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Über den privaten Pfad Ihrer App ($transport); Verbindungen können vom Proxy verknüpft werden';
  }

  @override
  String get walletTorHostOtherTransport => 'einen privaten Pfad';

  @override
  String get walletTorHostDirect => 'Nicht privat (Direktverbindung Ihrer App)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Ihr gespeicherter Server verwendet eine unverschlüsselte Adresse, die der private Pfad Ihrer App nicht übertragen kann. Es wird $host verwendet.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Mehr zu $label';
  }

  @override
  String get walletSendPaste => 'Einfügen';

  @override
  String get walletSendScanQr => 'QR-Code scannen';

  @override
  String get walletSendRecipientGetsLabel => 'Empfänger erhält';

  @override
  String get walletSwapDepositCopyAmount => 'Betrag kopieren';

  @override
  String get walletSwapDepositAmountCopied => 'Betrag kopiert';

  @override
  String get walletScanOpenSettings => 'Einstellungen öffnen';

  @override
  String get walletScanOpenSettingsFailed =>
      'Einstellungen konnten nicht geöffnet werden.';

  @override
  String get walletSendLeaveTitle => 'Wird noch gesendet';

  @override
  String get walletSendLeaveBody =>
      'Deine Zahlung läuft weiter, wenn du gehst. Wie sie ausgegangen ist, siehst du in deinen Aktivitäten.';

  @override
  String get walletSendLeaveStay => 'Bleiben';

  @override
  String get walletSendLeaveConfirm => 'Verlassen';

  @override
  String get walletSheetLeaveBody =>
      'Das läuft weiter, wenn du gehst. Wie es ausgegangen ist, siehst du in deinen Aktivitäten.';

  @override
  String get walletLoadingLabel => 'Wird geladen';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'Vor erneutem Schirmen prüfen';

  @override
  String get walletShieldUnknownBody =>
      'Wir konnten dieses Schirmen nicht bestätigen. Prüfen Sie „Aktivität“, bevor Sie es erneut versuchen.';

  @override
  String get walletMoveUnknownTitle => 'Vor erneutem Verschieben prüfen';

  @override
  String get walletMoveUnknownBody =>
      'Wir konnten diese Verschiebung nicht bestätigen. Prüfen Sie „Aktivität“, bevor Sie es erneut versuchen.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
