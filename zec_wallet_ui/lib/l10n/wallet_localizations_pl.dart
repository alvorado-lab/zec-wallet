// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Polish (`pl`).
class WalletLocalizationsPl extends WalletLocalizations {
  WalletLocalizationsPl([String locale = 'pl']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Ustawienia';

  @override
  String get walletTitle => 'Portfel';

  @override
  String get walletNotSetUpTitle => 'Portfel nie został jeszcze skonfigurowany';

  @override
  String get walletNotSetUpBody =>
      'Konfiguracja portfela pojawi się w kolejnej wersji. Poprowadzi Cię ona przez zapisanie frazy odzyskiwania, zanim będzie można odebrać jakiekolwiek środki — dzięki temu nic nigdy nie jest zagrożone bez kopii zapasowej.';

  @override
  String get walletStartupFailedTitle => 'Nie udało się uruchomić portfela';

  @override
  String get walletStartupFailedBody =>
      'Coś uniemożliwiło wczytanie portfela na tym urządzeniu. Jeśli masz już portfel, jego środki nie są zagrożone — znajdują się w sieci Zcash i można je odzyskać za pomocą frazy odzyskiwania. Spróbuj ponownie; jeśli problem się powtarza, zamknij aplikację i otwórz ją ponownie.';

  @override
  String get walletBalanceLabel => 'Saldo';

  @override
  String get walletHideBalance => 'Ukryj saldo';

  @override
  String get walletShowBalance => 'Pokaż saldo';

  @override
  String get walletBalanceHiddenAmount => 'Saldo ukryte';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Dostępne do wydania';

  @override
  String get walletArrivingLabel => 'Nadchodzące';

  @override
  String get walletNotSpendableYetLabel => 'Jeszcze niedostępne';

  @override
  String get walletActivityTitle => 'Aktywność';

  @override
  String get walletActivityEmpty => 'Brak aktywności';

  @override
  String get walletActivityError => 'Nie udało się wczytać aktywności';

  @override
  String get walletActivityReceived => 'Otrzymano';

  @override
  String get walletActivitySent => 'Wysłano';

  @override
  String get walletActivityPending => 'Oczekująca';

  @override
  String get walletActivityQueued => 'W kolejce';

  @override
  String get walletActivityRetrying => 'Ponawianie';

  @override
  String get walletActivitySaved => 'Zapisana';

  @override
  String get walletActivityExpired => 'Wygasła';

  @override
  String get walletActivityFailed => 'Nieudana';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count potwierdzeń',
      one: '1 potwierdzenie',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Otrzymano $count płatności',
      many: 'Otrzymano $count płatności',
      few: 'Otrzymano $count płatności',
      one: 'Otrzymano płatność',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Pokaż szczegóły transakcji';

  @override
  String get walletTxDetailStatus => 'Status';

  @override
  String get walletTxDetailFee => 'Opłata sieciowa';

  @override
  String get walletTxDetailDate => 'Data';

  @override
  String get walletTxDetailHeight => 'Wysokość bloku';

  @override
  String get walletTxDetailMemo => 'Notatka';

  @override
  String get walletTxDetailMemoAttached => 'Dołączona';

  @override
  String get walletTxDetailTxid => 'Identyfikator transakcji';

  @override
  String get walletTxDetailCopyTxid => 'Kopiuj identyfikator transakcji';

  @override
  String get walletTxDetailCopied => 'Skopiowano identyfikator transakcji';

  @override
  String get walletTxDetailClose => 'Zamknij';

  @override
  String get walletTxFundsKept => 'Żadne środki nie opuściły Twojego portfela';

  @override
  String get walletTxExplainQueued =>
      'Zapisano na tym urządzeniu, w Zapisane i oczekujące — możesz ją tam wysłać lub anulować.';

  @override
  String get walletTxExplainPending =>
      'Wysłano do sieci Zcash — oczekuje na potwierdzenie w bloku.';

  @override
  String get walletTxExplainRetrying =>
      'Twój portfel nie zdołał jeszcze wysłać tej transakcji do sieci Zcash. Zachowuje podpisaną transakcję i ponawia próbę przy każdej synchronizacji, aż przejdzie lub wygaśnie.';

  @override
  String get walletTxExplainSaved =>
      'Twój portfel zachował tę podpisaną transakcję, ale obecnie nie wysyła jej samodzielnie.';

  @override
  String get walletTxExplainConfirmed => 'Potwierdzona w sieci Zcash.';

  @override
  String get walletTxExplainExpired =>
      'Ta transakcja wygasła, zanim sieć zdążyła ją potwierdzić, więc została anulowana. Kwota nadal należy do Ciebie i możesz nią dysponować.';

  @override
  String get walletTxExplainFailed =>
      'Sieć odrzuciła tę transakcję, więc nie doszła do skutku. Kwota nadal należy do Ciebie i możesz nią dysponować.';

  @override
  String get walletTxExplainUnknown =>
      'Aktualnego statusu tej transakcji nie można ustalić. Zostanie zaktualizowany po następnej synchronizacji.';

  @override
  String get walletMenuTooltip => 'Więcej opcji';

  @override
  String get walletRescanMenuItem => 'Ponownie skanuj historię…';

  @override
  String get walletCheckOneTimeMenuItem => 'Sprawdź adresy jednorazowe…';

  @override
  String get walletRescanTitle => 'Ponownie przeskanuj historię';

  @override
  String get walletRescanBody =>
      'Brakuje starszych środków? Przeskanuj blockchain od wcześniejszego momentu, aby odzyskać wpłaty pominięte przez późniejszą datę początkową. Twoje środki i fraza odzyskiwania nigdy nie są zagrożone.';

  @override
  String get walletRescanRangeTitle => 'Jak daleko wstecz skanować';

  @override
  String get walletRescanRangeAll =>
      'Przeskanuj całą historię — najwolniejsza opcja, ale odzyskuje wszystko.';

  @override
  String get walletRescanRangeDefault =>
      'Skanowanie od początku Twojego portfela. Nadal brakuje starszych środków? Wybierz wcześniejszą datę lub przeskanuj całą historię.';

  @override
  String get walletRescanRangeResolving =>
      'Przygotowywanie zalecanego zakresu…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Około $blocks bloków do zeskanowania.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Skanowanie od $date. Nadal brakuje starszych środków? Wybierz wcześniejszą datę lub przeskanuj całą historię.';
  }

  @override
  String get walletRescanPick => 'Wybierz datę';

  @override
  String get walletRescanChange => 'Zmień datę';

  @override
  String get walletRescanScanAll => 'Przeskanuj całą historię';

  @override
  String get walletRescanDatePick => 'Najwcześniejsza data skanowania';

  @override
  String get walletRescanWarning =>
      'Ta operacja ponownie skanuje blockchain. Niedawne daty zajmują kilka minut; skanowanie daleko wstecz może potrwać godziny. Synchronizacja działa w tle — możesz nadal korzystać z portfela.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Płatność z tego portfela wciąż się rozlicza. Portfel zwykle odmawia ponownego skanowania, dopóki się nie zakończy — możesz spróbować, ale spodziewaj się odmowy.';

  @override
  String get walletRescanConfirm => 'Rozpocznij ponowne skanowanie';

  @override
  String get walletRescanCancel => 'Anuluj';

  @override
  String get walletRescanRunning => 'Odbudowywanie…';

  @override
  String get walletRescanRebuildingAll =>
      'Odbudowywanie historii — skanowanie całego łańcucha bloków. Saldo i aktywność uzupełnią się w miarę postępu.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Odbudowywanie historii od $date — saldo i aktywność uzupełnią się w miarę postępu.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Odbudowywanie historii od początku Twojego portfela — saldo i aktywność uzupełnią się w miarę postępu.';

  @override
  String get walletCatchUpBanner =>
      'Nadrabianie zaległości — saldo i aktywność uzupełniają się w miarę synchronizacji portfela. Wszystko, co zostało otrzymane, jest bezpieczne.';

  @override
  String get walletCatchUpRescanBanner =>
      'Odbudowywanie historii po ponownym skanowaniu — saldo i aktywność uzupełnią się w miarę postępu. Wszystko, co zostało otrzymane, jest bezpieczne.';

  @override
  String get walletRescanFailedNotice =>
      'Nie udało się teraz ponownie przeskanować — Twoje środki są bezpieczne, choć saldo i historia mogą potrzebować chwili, aby nadrobić zaległości. Spróbuj ponownie za chwilę.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Płatność wciąż się rozlicza, dlatego ponowne skanowanie zostało wstrzymane, aby chronić Twoje środki. Twój portfel pozostaje bez zmian — spróbuj ponownie za jakieś dwie godziny i przez ten czas trzymaj aplikację otwartą i online.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'Ponowne skanowanie odbudowuje Twoją historię, gdy portfel się synchronizuje, a synchronizacja teraz nie działa. Twój portfel pozostaje bez zmian — spróbuj ponownie, gdy synchronizacja będzie działać.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Za mało wolnego miejsca, aby odbudować historię portfela — Twoje środki są bezpieczne, choć saldo i historia mogą potrzebować chwili, aby nadrobić zaległości. Zwolnij trochę miejsca i spróbuj ponownie.';

  @override
  String get walletRescanFailedDismiss => 'Zamknij';

  @override
  String get walletActivityRebuilding => 'Odbudowywanie historii…';

  @override
  String get walletActivityCatchingUp =>
      'Wciąż trwa nadrabianie zaległości — wszystko, co zostało otrzymane, pojawi się tutaj.';

  @override
  String get walletActivitySyncNotRunning =>
      'Twoje saldo i historia zostaną w pełni załadowane, gdy synchronizacja będzie działać.';

  @override
  String get walletActivityLoadMore => 'Wczytaj więcej';

  @override
  String get walletPendingChangeLabel => 'Oczekująca reszta';

  @override
  String get walletTransparentLabel => 'Niechronione (publiczne)';

  @override
  String get walletTransparentNote =>
      'Nieuwzględnione w „Dostępne do wydania” — obejmij te środki ochroną, aby móc je wydać. Do tego czasu pozostają publicznie widoczne w blockchainie.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Te środki są publicznie widoczne w blockchainie.';

  @override
  String walletPoolShielded(String amount) {
    return 'Chronione $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Publiczne $amount';
  }

  @override
  String get walletPoolAllShielded => 'Wszystko chronione · prywatne';

  @override
  String get walletPoolTapHint => 'Pokaż środki publiczne';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount Twojego salda znajduje się na adresie jednorazowym (możliwe do odzyskania).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount Twojego salda znajduje się na adresie jednorazowym.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Płatności na łączną kwotę $amount są zarezerwowane i wciąż są finalizowane przez adresy jednorazowe kontrolowane przez Twój portfel. Nie wysyłaj ich ponownie.',
      many:
          'Płatności na łączną kwotę $amount są zarezerwowane i wciąż są finalizowane przez adresy jednorazowe kontrolowane przez Twój portfel. Nie wysyłaj ich ponownie.',
      few:
          'Płatności na łączną kwotę $amount są zarezerwowane i wciąż są finalizowane przez adresy jednorazowe kontrolowane przez Twój portfel. Nie wysyłaj ich ponownie.',
      one:
          '$amount jest zarezerwowane na płatność, którą Twój portfel wciąż finalizuje przez adres jednorazowy, który kontroluje. Nie wysyłaj tej kwoty ponownie.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Płatności na łączną kwotę $amount są zarezerwowane w połowie drogi przez adresy jednorazowe kontrolowane przez Twój portfel. Wstrzymane, dopóki Twój portfel nie zacznie ponownie się synchronizować. Nie wysyłaj ich ponownie.',
      many:
          'Płatności na łączną kwotę $amount są zarezerwowane w połowie drogi przez adresy jednorazowe kontrolowane przez Twój portfel. Wstrzymane, dopóki Twój portfel nie zacznie ponownie się synchronizować. Nie wysyłaj ich ponownie.',
      few:
          'Płatności na łączną kwotę $amount są zarezerwowane w połowie drogi przez adresy jednorazowe kontrolowane przez Twój portfel. Wstrzymane, dopóki Twój portfel nie zacznie ponownie się synchronizować. Nie wysyłaj ich ponownie.',
      one:
          '$amount jest zarezerwowane na płatność w połowie drogi przez adres jednorazowy, który kontroluje Twój portfel. Wstrzymane, dopóki Twój portfel nie zacznie ponownie się synchronizować. Nie wysyłaj tej kwoty ponownie.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Nie udało się sprawdzić, czy płatność jest nadal realizowana. Ponawiamy próbę — do tego czasu sprawdź w aktywności, czy nie ma oczekującej płatności, zanim wyślesz ponownie.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount Twojego salda znajduje się na adresie jednorazowym (wciąż potwierdzane).';
  }

  @override
  String get walletShieldButton => 'Chroń';

  @override
  String get walletShieldSheetTitle => 'Obejmij ochroną środki publiczne';

  @override
  String get walletShieldNote =>
      'Ta operacja przenosi środki z Twojego publicznego, widocznego w blockchainie salda do prywatnego salda chronionego.';

  @override
  String get walletShieldPreparing => 'Przygotowywanie…';

  @override
  String get walletShieldAmountLabel => 'Kwota do ochrony';

  @override
  String get walletShieldFeeLabel => 'Opłata sieciowa';

  @override
  String get walletShieldNetLabel => 'Trafi do chronionych';

  @override
  String get walletShieldConfirmButton => 'Obejmij ochroną teraz';

  @override
  String get walletShieldSubmitting => 'Obejmowanie ochroną…';

  @override
  String get walletShieldNothingTitle =>
      'Nie ma jeszcze niczego do objęcia ochroną';

  @override
  String get walletShieldNothingBody =>
      'Te środki są obecnie poniżej kwoty, dla której opłaca się objęcie ochroną — opłata sieciowa przewyższyłaby korzyść. Będzie to możliwe, gdy napłynie nieco więcej środków.';

  @override
  String get walletShieldDoneTitle => 'Ochrona środków wysłana';

  @override
  String get walletShieldDoneBody =>
      'Twoje środki są przenoszone do salda chronionego. Wkrótce zostaną potwierdzone w blockchainie.';

  @override
  String get walletShieldSavedTitle => 'Zapisano — dokończymy ochronę';

  @override
  String get walletShieldSavedBody =>
      'Nie udało się teraz połączyć z siecią. Twoja ochrona została zapisana, a Twój portfel dokończy ją przy jednej z późniejszych synchronizacji. Nic nie zostało utracone.';

  @override
  String get walletShieldAlreadyTitle => 'Już wysłano';

  @override
  String get walletShieldFailedTitle => 'Nie udało się teraz objąć ochroną';

  @override
  String get walletShieldStaleBody =>
      'Portfel wciąż się synchronizuje. Spróbuj ponownie objąć środki ochroną za chwilę.';

  @override
  String get walletShieldTransientBody =>
      'Nie udało się teraz przygotować osłony. Spróbuj ponownie za chwilę.';

  @override
  String get walletShieldStorageFullBody =>
      'Za mało wolnego miejsca, aby teraz objąć środki ochroną. Zwolnij trochę miejsca i spróbuj ponownie. Twoje środki są bezpieczne.';

  @override
  String get walletShieldClose => 'Zamknij';

  @override
  String get walletShieldRetry => 'Spróbuj ponownie';

  @override
  String get walletMoveMenuItem => 'Przenieś do adresu publicznego…';

  @override
  String get walletMoveSheetTitle => 'Przenieś do adresu publicznego';

  @override
  String get walletMoveSheetSubtitle =>
      'Wyślij chronione ZEC na swój własny adres publiczny — przydatne, gdy giełda nie akceptuje wpłat z adresów chronionych.';

  @override
  String get walletMoveDestinationLabel => 'Twój adres publiczny';

  @override
  String walletMoveAvailable(String amount) {
    return 'Dostępne do przeniesienia: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Dostępne do przeniesienia: $amount ZEC — saldo wciąż nadrabia zaległości';
  }

  @override
  String get walletMoveDeshieldTitle =>
      'Ta operacja czyni Twoje środki publicznymi';

  @override
  String get walletMoveDeshieldBody =>
      'Przeniesienie na adres publiczny wyprowadza te środki z Twojego salda chronionego — kwota oraz Twój adres publiczny staną się publicznie widoczne w blockchainie Zcash.';

  @override
  String get walletMoveWalletEnded =>
      'Sesja portfela została zakończona. Zamknij i otwórz ponownie, aby spróbować jeszcze raz.';

  @override
  String get walletMoveLoading => 'Przygotowywanie…';

  @override
  String get walletMovePreparing => 'Sprawdzanie kwoty…';

  @override
  String get walletMoveSubmitting => 'Przenoszenie…';

  @override
  String get walletMoveReviewButton => 'Sprawdź';

  @override
  String get walletMoveCancel => 'Anuluj';

  @override
  String get walletMoveReviewTitle => 'Sprawdź przeniesienie';

  @override
  String get walletMoveOwnAddressNote =>
      'Przenosisz środki na swój własny adres publiczny. Możesz później ponownie objąć te środki ochroną, ale ta operacja na stałe pozostanie w publicznym rejestrze.';

  @override
  String get walletMoveConfirmButton => 'Przenieś do adresu publicznego';

  @override
  String get walletMoveBackButton => 'Wstecz';

  @override
  String get walletMoveDoneTitle => 'Przeniesiono do adresu publicznego';

  @override
  String get walletMoveDoneBody =>
      'Twoje środki są przenoszone na adres publiczny. Wkrótce zostaną potwierdzone w blockchainie.';

  @override
  String get walletMoveSavedTitle => 'Zapisano — dokończymy przeniesienie';

  @override
  String get walletMoveSavedBody =>
      'To przeniesienie zostało zapisane, a Twój portfel wyśle je przy jednej z późniejszych synchronizacji. Nic nie zostało utracone.';

  @override
  String get walletMoveAlreadyTitle => 'Już wysłano';

  @override
  String get walletMoveAlreadyBody =>
      'Te środki zostały już wysłane i są w drodze na Twój adres publiczny.';

  @override
  String get walletMoveFailedTitle => 'Nie udało się ukończyć tej operacji';

  @override
  String get walletMoveNothingTitle =>
      'Nie ma jeszcze niczego do przeniesienia';

  @override
  String get walletMoveNothingBody =>
      'Nie masz obecnie dostępnego salda chronionego do przeniesienia. Gdy środki zostaną potwierdzone, będziesz mógł przenieść je na adres publiczny.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Portfel wciąż nadrabia zaległości — wszystko, co zostało otrzymane, stanie się dostępne do przeniesienia po zakończeniu synchronizacji.';

  @override
  String get walletMoveCouldNotLoad =>
      'Nie udało się wczytać Twojego adresu publicznego. Spróbuj ponownie.';

  @override
  String get walletMoveRetry => 'Spróbuj ponownie';

  @override
  String get walletMoveClose => 'Zamknij';

  @override
  String get walletSnapshotUnavailable =>
      'Nie udało się teraz odczytać portfela. Odświeży się automatycznie.';

  @override
  String get walletBalanceStale =>
      'Nie udało się odświeżyć — wyświetlane jest ostatnie znane saldo.';

  @override
  String get walletSyncStartFailed =>
      'Nie udało się rozpocząć synchronizacji. Będziemy próbować dalej.';

  @override
  String get walletSyncRetry => 'Spróbuj ponownie';

  @override
  String get walletSyncTryNow => 'Spróbuj teraz';

  @override
  String get walletSyncIdle => 'Jeszcze nie synchronizowano';

  @override
  String get walletSyncIdleDetail =>
      'Synchronizacja rozpocznie się automatycznie.';

  @override
  String get walletSyncDisabled => 'Synchronizacja wyłączona';

  @override
  String get walletSyncDisabledDetail =>
      'Włącz synchronizację w ustawieniach tej aplikacji, aby zaktualizować saldo.';

  @override
  String get walletSyncExplainDisabled =>
      'Synchronizacja jest wyłączona w ustawieniach tej aplikacji. Twoje środki są bezpieczne. Twoje saldo i aktywność pokazują ostatni zsynchronizowany stan i nie zostaną zaktualizowane, dopóki synchronizacja nie zostanie włączona.';

  @override
  String get walletParkedSyncPausedNote =>
      'Twój portfel się nie synchronizuje, więc te płatności nie wyślą się same. Użyj przycisku Wyślij teraz, aby wysłać jedną samodzielnie.';

  @override
  String get walletSyncPausedMoneyNote =>
      'Wstrzymane, dopóki Twój portfel nie zacznie ponownie się synchronizować.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Łączenie…';

  @override
  String get walletSyncStartingDetail =>
      'Nawiązywanie połączenia z siecią Zcash i przygotowywanie do skanowania.';

  @override
  String get walletSyncConnecting => 'Łączenie…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Łączenie… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Skanowanie $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'Skanowanie…';

  @override
  String get walletSyncSpendableReady => 'Środki są gotowe do wydania.';

  @override
  String get walletSyncCatchingUp =>
      'Nadrabianie zaległości względem sieci — dogłębna początkowa synchronizacja może chwilę potrwać. Możesz nadal korzystać z aplikacji, aż się zakończy';

  @override
  String walletSyncScanRemaining(String count) {
    return 'Pozostało $count bloków';
  }

  @override
  String get walletSyncUpToDate => 'Aktualny stan';

  @override
  String get walletSyncOffline => 'Offline';

  @override
  String get walletSyncOfflineDetail =>
      'Płatności w kolejce pozostają zapisane w Zapisane i oczekujące.';

  @override
  String get walletSyncUnknown => 'Synchronizowanie…';

  @override
  String get walletSyncStalled => 'Synchronizacja wstrzymana';

  @override
  String get walletStallEndpoint =>
      'Nie można teraz połączyć się z siecią Zcash. Będziemy próbować dalej automatycznie — sprawdź swoje połączenie z internetem albo serwer może być tymczasowo niedostępny.';

  @override
  String get walletStallTor =>
      'Prywatna ścieżka w Twojej aplikacji jest niedostępna, więc portfel się nie łączy. Sprawdź ustawienia sieci w swojej aplikacji lub wyłącz prywatną ścieżkę. Synchronizacja wznowi się, gdy ścieżka wróci.';

  @override
  String get walletStallStorage =>
      'Pamięć urządzenia jest pełna. Zwolnij trochę miejsca, a synchronizacja zostanie wznowiona.';

  @override
  String get walletStallReorg =>
      'Nastąpiła reorganizacja łańcucha bloków; ponowne sprawdzanie ostatnich bloków.';

  @override
  String get walletStallInternal =>
      'Lokalny problem zatrzymał synchronizację. Jeśli będzie się powtarzać, przywróć portfel z frazy odzyskiwania.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Ten serwer wysłał dane, które nie mogą być poprawne, więc synchronizacja została zatrzymana. To nie jest problem z połączeniem — przełącz się na inny serwer. Jeśli każdy serwer jest odrzucany, ponownie skanuj historię: portfel może przechowywać błędny wpis z wcześniejszego serwera.';

  @override
  String get walletStallBirthdayInFuture =>
      'Ten portfel jest ustawiony tak, by zaczynać od bloku, którego ten serwer jeszcze nie osiągnął. Sprawdź blok początkowy ustawiony dla tego portfela lub spróbuj innego serwera.';

  @override
  String get walletStallStorageUnavailable =>
      'Synchronizacja wstrzymana na tym urządzeniu. Ponawianie próby.';

  @override
  String get walletStallUnknown =>
      'Synchronizacja zatrzymała się z nieznanego powodu.';

  @override
  String get walletSyncBadgeHint => 'Pokaż szczegóły synchronizacji';

  @override
  String get walletSyncSheetClose => 'Zamknij';

  @override
  String get walletSyncSheetProgress => 'Postęp';

  @override
  String get walletSyncSheetBlocksLeft => 'Pozostałe bloki';

  @override
  String get walletSyncSheetSyncedTo => 'Zsynchronizowano do bloku';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'W tyle o co najmniej $blocks bloku',
      many: 'W tyle o co najmniej $blocks bloków',
      few: 'W tyle o co najmniej $blocks bloki',
      one: 'W tyle o co najmniej 1 blok',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'Synchronizacja jeszcze się nie rozpoczęła — uruchomi się automatycznie. Nie jest wymagane żadne działanie.';

  @override
  String get walletSyncExplainStartFailed =>
      'Synchronizacja nie mogła się rozpocząć. Twoje środki są bezpieczne — portfel po prostu nie sprawdza teraz nowej aktywności. Spróbuj ponownie poniżej albo otwórz aplikację ponownie.';

  @override
  String get walletSyncExplainStarting =>
      'Portfel nawiązuje kontakt z siecią Zcash i przygotowuje się do skanowania. Zwykle trwa to kilka sekund.';

  @override
  String get walletSyncExplainConnecting =>
      'Nawiązywanie połączenia z siecią Zcash.';

  @override
  String get walletSyncExplainScanning =>
      'Portfel sprawdza bloki blockchaina w poszukiwaniu Twoich środków. Saldo i aktywność aktualizują się w miarę znajdowania nowych transakcji — możesz nadal korzystać z aplikacji, aż proces się zakończy.';

  @override
  String get walletSyncExplainUpToDate =>
      'W pełni zsynchronizowano z siecią Zcash. Saldo i aktywność są aktualne.';

  @override
  String get walletSyncExplainStalled =>
      'Synchronizacja napotkała problem i została wstrzymana. Zostanie automatycznie wznowiona.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Nie można połączyć się z siecią Zcash — to normalne, jeśli jesteś offline, albo serwer może być tymczasowo niedostępny. Twoje środki są bezpieczne — saldo pokazuje ostatni zsynchronizowany stan, a płatności w kolejce pozostają zapisane w Zapisane i oczekujące. Połączenie ponawia próby samodzielnie.';

  @override
  String get walletSyncExplainOffline =>
      'Brak połączenia z siecią. Twoje środki są bezpieczne — saldo pokazuje ostatni zsynchronizowany stan, a płatności w kolejce pozostają zapisane w Zapisane i oczekujące.';

  @override
  String get walletSyncExplainUnknown =>
      'Portfel się synchronizuje. Saldo i aktywność aktualizują się w miarę postępu.';

  @override
  String get walletTorOff => 'Tor wyłączony';

  @override
  String get walletTorBootstrapping => 'Uruchamianie prywatnej ścieżki…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return 'Uruchamianie $transport…';
  }

  @override
  String get walletTorActive => 'Tor aktywny';

  @override
  String get walletTorActiveUnverified =>
      'Tor aktywny (niezweryfikowane środowisko)';

  @override
  String get walletTorActiveUnattested =>
      'Prywatna ścieżka w użyciu (prywatność niezweryfikowana)';

  @override
  String get walletTorFellBack =>
      'Tor niedostępny — używane połączenie bezpośrednie';

  @override
  String get walletTorUnavailable =>
      'Prywatna ścieżka niedostępna — brak połączenia';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport niedostępny — brak połączenia';
  }

  @override
  String get walletTorUnanswered =>
      'Prywatna ścieżka połączona — nic nie wraca';

  @override
  String get walletTorUnansweredUnattested =>
      'Prywatna ścieżka połączona — nic nie wraca (prywatność niezweryfikowana)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport połączony — nic nie wraca';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Nieprywatne (bezpośrednie połączenie Twojej aplikacji) — nic nie wraca';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Połączono przez $transport — nic nie wraca; proxy może powiązać połączenia';
  }

  @override
  String get walletTorUnknown =>
      'Status Tor nieznany — traktuj jako niechronione';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Saldo (stan na blok $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Saldo · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Saldo (stan na blok $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Połączenie';

  @override
  String get walletSyncSheetServer => 'Serwer';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Serwer, $host, otwiera wybór serwera';
  }

  @override
  String get walletSyncServerSheetTitle => 'Serwer synchronizacji';

  @override
  String get walletSyncServerInUse => 'W użyciu';

  @override
  String get walletSyncServerAppDefault => 'Domyślny aplikacji';

  @override
  String get walletSyncServerCustom => 'Własny serwer…';

  @override
  String get walletSyncServerCustomHint => 'https://host:port';

  @override
  String get walletSyncServerCheck => 'Sprawdź serwer';

  @override
  String get walletSyncServerChecking => 'Sprawdzanie…';

  @override
  String get walletSyncServerUse => 'Użyj tego serwera';

  @override
  String get walletSyncServerSwitching => 'Przełączanie…';

  @override
  String get walletSyncServerContinue => 'Dalej';

  @override
  String get walletSyncServerCancel => 'Anuluj';

  @override
  String get walletSyncServerTrustTitle => 'Zaufać temu serwerowi?';

  @override
  String get walletSyncServerTrustNotice =>
      'Ufasz temu serwerowi, że poda Twoje saldo i historię oraz przekaże Twoje płatności. Zobaczy Twój adres IP, chyba że Tor jest włączony, mniej więcej kiedy utworzono portfel, publiczne adresy sprawdzane przez portfel, transakcje, które wyszukuje, oraz wysyłane przez Ciebie transakcje.';

  @override
  String get walletSyncServerKeyLabel => 'Klucz dostępu (opcjonalnie)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Nagłówek klucza';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Wpisz nagłówek, którego oczekuje Twój serwer';

  @override
  String get walletSyncServerKeyInvalid =>
      'Nie można użyć tego klucza lub nagłówka';

  @override
  String get walletSyncServerKeySaved => 'Klucz zapisany';

  @override
  String get walletSyncServerKeyShow => 'Pokaż';

  @override
  String get walletSyncServerKeyHide => 'Ukryj';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Twój klucz identyfikuje Cię wobec tego serwera. Może powiązać Twoje płatności z Twoim portfelem, nawet przez Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Przełączenie uruchamia ponownie trwającą synchronizację. Saldo i historia zostają. Środki mogą być wyświetlane jako nadchodzące, dopóki skanowanie nowego serwera nie nadrobi zaległości.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Przełączenie ponownie łączy z nowym serwerem. Saldo i historia zostają.';

  @override
  String get walletSyncServerUnreachable =>
      'Nie udało się połączyć z tym serwerem. Sprawdź adres — a jeśli jest poprawny, to albo ten serwer nie odpowiada, albo Twoja aplikacja nie może go teraz osiągnąć. Spróbuj ponownie lub wybierz inny serwer.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Nie udało się połączyć z tym serwerem. Portfel nie jest w stanie rozróżnić, czy ten serwer nie odpowiada, czy Twoja aplikacja nie może go teraz osiągnąć. Wybierz inny serwer lub spróbuj później.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Ten serwer działa w innej sieci Zcash.';

  @override
  String get walletSyncServerInvalidUrl =>
      'To nie wygląda na adres serwera. Użyj formatu https://host:port.';

  @override
  String get walletSyncServerNotOffered =>
      'Ta aplikacja nie oferuje tego serwera.';

  @override
  String get walletSyncServerBusy =>
      'Portfel jest teraz zajęty. Spróbuj ponownie za chwilę.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'Wybrany serwer nie jest już oferowany przez tę aplikację. Używany jest $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'Nie udało się odczytać zapisanego wyboru serwera. Używany jest $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Nie udało się przełączyć — nadal używany jest $host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Ruch portfela łączy się bezpośrednio z serwerem. Serwer może zobaczyć Twój adres IP.';

  @override
  String get walletTransportExplainTor =>
      'Ruch portfela jest kierowany przez sieć Tor, co ukrywa Twój adres IP przed serwerem.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Prywatna ścieżka Twojej aplikacji się uruchamia. Ruch portfela czeka na jej gotowość przed połączeniem.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport się uruchamia. Ruch portfela czeka na gotowość przed połączeniem.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Nie udało się połączyć z Tor, więc ruch przełączył się na połączenie bezpośrednie. Serwer może zobaczyć Twój adres IP.';

  @override
  String get walletTransportExplainUnavailable =>
      'Prywatna ścieżka w Twojej aplikacji jest niedostępna, więc portfel się nie łączy. Wyłącz prywatną ścieżkę lub sprawdź ustawienia sieci w swojej aplikacji.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport jest niedostępny, więc portfel się nie łączy. Wyłącz go lub sprawdź ustawienia sieci w swojej aplikacji.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'Prywatna ścieżka przyjęła połączenie, ale od minuty nic nie wraca. To może być ścieżka albo serwer portfela — portfel nie jest w stanie tego rozróżnić. Próbuje dalej; jeśli to nie ustąpi, spróbuj innego serwera lub sprawdź ustawienia sieci w swojej aplikacji.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport przyjął połączenie, ale od minuty nic nie wraca. To może być ścieżka albo serwer portfela — portfel nie jest w stanie tego rozróżnić. Próbuje dalej; jeśli to nie ustąpi, spróbuj innego serwera lub sprawdź ustawienia sieci w swojej aplikacji.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Ruch portfela łączy się bezpośrednio z serwerem. Serwer może zobaczyć Twój adres IP. Połączenie zostało przyjęte, ale od minuty nic nie wraca. To może być ścieżka albo serwer portfela — portfel nie jest w stanie tego rozróżnić. Próbuje dalej; jeśli to nie ustąpi, spróbuj innego serwera lub sprawdź ustawienia sieci w swojej aplikacji.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'Prywatności tego połączenia nie można zweryfikować — traktuj je jako nieprywatne. Połączenie zostało przyjęte, ale od minuty nic nie wraca. To może być ścieżka albo serwer portfela — portfel nie jest w stanie tego rozróżnić. Próbuje dalej; jeśli to nie ustąpi, spróbuj innego serwera lub sprawdź ustawienia sieci w swojej aplikacji.';

  @override
  String get walletTransportExplainUnverified =>
      'Prywatności tego połączenia nie można zweryfikować — traktuj je jako nieprywatne.';

  @override
  String get walletTransportExplainHostProxy =>
      'Ruch portfela jest kierowany przez transport prywatności tej aplikacji, co ukrywa Twój adres IP przed serwerem.';

  @override
  String get walletOnboardingWelcomeTitle => 'Skonfiguruj swój portfel';

  @override
  String get walletOnboardingWelcomeBody =>
      'Utwórz nowy portfel, aby otrzymywać i przechowywać ZEC. Wygenerujemy frazę odzyskiwania i przeprowadzimy Cię przez proces jej zabezpieczenia, zanim będą mogły napłynąć jakiekolwiek środki — dzięki temu nic nigdy nie jest zagrożone bez kopii zapasowej.';

  @override
  String get walletCreateButton => 'Utwórz nowy portfel';

  @override
  String get walletRestoreButton => 'Przywróć z frazy odzyskiwania';

  @override
  String get walletWatchOnlyButton => 'Podglądaj portfel (tylko do podglądu)';

  @override
  String get walletWatchOnlyTitle => 'Podglądaj portfel';

  @override
  String get walletWatchOnlyBody =>
      'Wklej klucz podglądu, aby podglądać portfel bez jego kluczy wydatkowania. Zobaczysz jego saldo i historię, ale nie będzie można wysyłać środków. Wybierz przybliżoną datę początkową portfela, abyśmy wiedzieli, jak daleko wstecz szukać.';

  @override
  String get walletWatchOnlyKeyLabel => 'Klucz podglądu';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => 'Zeskanuj kod QR klucza podglądu';

  @override
  String get walletWatchOnlyScanTitle => 'Skanuj klucz podglądu';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Skieruj aparat na kod QR klucza podglądu.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Aparat niedostępny. Wklej klucz ręcznie zamiast tego.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Wklej zamiast tego';

  @override
  String get walletWatchOnlyScanHint =>
      'Lub dotknij przycisku skanowania, aby odczytać kod QR klucza podglądu.';

  @override
  String get walletWatchOnlyScanFilled => 'Zeskanowano klucz podglądu.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Data początkowa portfela';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Skanowanie od $date — środki otrzymane wcześniej się nie pojawią. Starszy portfel? Wybierz wcześniejszą datę.';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'Wybierz datę początkową portfela';

  @override
  String get walletWatchOnlyBirthdayChange => 'Zmień datę';

  @override
  String get walletWatchOnlySubmit => 'Podglądaj ten portfel';

  @override
  String get walletWatchOnlyBack => 'Wstecz';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'To nie wygląda na prawidłowy klucz podglądu. Sprawdź go i spróbuj ponownie.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Ten klucz podglądu jest przeznaczony dla innej sieci. Nie można go tutaj użyć.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'Na tym urządzeniu już istnieje portfel. Wróć i otwórz go zamiast tego.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Ta data początkowa jest zbyt późna. Wybierz wcześniejszą datę.';

  @override
  String get walletRestoreTitle => 'Przywróć swój portfel';

  @override
  String get walletRestoreBody =>
      'Wprowadź frazę odzyskiwania, aby przywrócić portfel — wpisz lub wklej słowa po kolei, oddzielone spacjami. Obsługiwane są tylko standardowe frazy: jeśli Twój portfel używał dodatkowego hasła („25. słowa”), ta aplikacja nie może go jeszcze przywrócić — zobaczysz pusty portfel, a nie błąd.';

  @override
  String get walletRestorePhraseHint =>
      'słowo pierwsze  słowo drugie  słowo trzecie  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count słów',
      one: '1 słowo',
      zero: 'Brak słów',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'frazy odzyskiwania mają 12, 15, 18, 21 lub 24 słowa';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count słów nie jest słowami odzyskiwania — popraw zaznaczone',
      one: '1 słowo nie jest słowem odzyskiwania — popraw zaznaczone',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'słowo $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'słowo $index: nie jest słowem odzyskiwania';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Usuń słowo $index';
  }

  @override
  String get walletRestoreSubmit => 'Przywróć portfel';

  @override
  String get walletRestoreBack => 'Wstecz';

  @override
  String get walletRestoreBirthdayTitle => 'Jak daleko wstecz skanować';

  @override
  String get walletRestoreBirthdayNone =>
      'Przeskanujemy całą historię — wolniej, ale nic nie zostanie pominięte.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Skanowanie od $date — środki otrzymane wcześniej się nie pojawią. Starszy portfel? Wybierz wcześniejszą datę lub przeskanuj całą historię.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Wybierz datę';

  @override
  String get walletRestoreBirthdayChange => 'Zmień datę';

  @override
  String get walletRestoreBirthdayClear => 'Przeskanuj całą historię';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'Słowo $index nie jest słowem odzyskiwania. Sprawdź frazę pod kątem literówek i spróbuj ponownie.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Ta fraza odzyskiwania jest nieprawidłowa. Sprawdź słowa i ich kolejność, a następnie spróbuj ponownie.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Ta fraza nie pasuje do portfela na tym urządzeniu. Sprawdź ją dokładnie i spróbuj ponownie.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'Na tym urządzeniu istnieje już portfel. Wróć, aby go otworzyć.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Ta data jest zbyt niedawna. Wybierz wcześniejszą datę lub przeskanuj wszystko.';

  @override
  String get walletGeneratingLabel => 'Tworzenie portfela…';

  @override
  String get walletOpeningLabel => 'Otwieranie portfela…';

  @override
  String get walletBackupTitle => 'Zabezpiecz swoją frazę odzyskiwania';

  @override
  String get walletBackupBody =>
      'Te słowa są JEDYNYM sposobem na odzyskanie portfela i środków. Zapisz je w odpowiedniej kolejności i przechowuj w bezpiecznym, prywatnym miejscu. Nigdy nie udostępniaj ich ani nie przechowuj online — każdy, kto pozna te słowa, może przejąć Twoje środki.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Zrzuty ekranu są wyłączone na tym ekranie.';

  @override
  String get walletBackupSecureNoteOther =>
      'Upewnij się, że nikt nie może zobaczyć Twojego ekranu.';

  @override
  String get walletBackupReveal => 'Pokaż frazę odzyskiwania';

  @override
  String get walletBackupRevealing => 'Przygotowywanie frazy odzyskiwania…';

  @override
  String get walletBackupRevealFailed =>
      'Nie udało się teraz wyświetlić frazy odzyskiwania. Upewnij się, że urządzenie jest odblokowane, i spróbuj ponownie.';

  @override
  String get walletBackupRetryReveal => 'Spróbuj ponownie';

  @override
  String get walletBackupReauthFailed =>
      'Nie udało się zweryfikować Twojej tożsamości. Spróbuj ponownie.';

  @override
  String get walletBackupConfirmCheckbox =>
      'Fraza odzyskiwania została zapisana i jest przechowywana w bezpiecznym miejscu.';

  @override
  String get walletBackupContinue => 'Kontynuuj';

  @override
  String get walletBackupSaveFailed =>
      'Nie udało się zapisać potwierdzenia. Spróbuj ponownie.';

  @override
  String get walletBackupStartOver => 'Zacznij od nowa';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Zacząć od nowa bez tego portfela?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Spowoduje to usunięcie tego portfela z urządzenia i powrót do początku. Zanim konfiguracja zostanie ukończona, nie można nic wpłacić przez tę aplikację.\n\nJeśli ten portfel kiedykolwiek zawierał środki — lub został przywrócony z frazy odzyskiwania — tylko ta fraza może go przywrócić.';

  @override
  String get walletBackupStartOverConfirm => 'Usuń i zacznij od nowa';

  @override
  String get walletBackupStartOverKeep => 'Zachowaj ten portfel';

  @override
  String get walletBackupSectionTitle => 'Fraza odzyskiwania';

  @override
  String get walletBackupTileTitle => 'Zabezpiecz swoją frazę odzyskiwania';

  @override
  String get walletBackupTileSubtitle =>
      'Pokaż słowa, które pozwalają odzyskać portfel i środki.';

  @override
  String get walletBackupScreenTitle => 'Fraza odzyskiwania';

  @override
  String get walletBackupDone => 'Gotowe';

  @override
  String get walletBackupManagedTitle => 'Brak własnej frazy odzyskiwania';

  @override
  String get walletBackupManagedBody =>
      'Ten portfel został skonfigurowany przy użyciu Twojego konta z aplikacji, która go zainstalowała, dlatego nie ma własnej frazy odzyskiwania. Twoje środki są odzyskiwane razem z tym kontem — użyj jego kopii zapasowej, aby je zabezpieczyć.';

  @override
  String get walletExportViewingKeyTitle => 'Eksportuj klucz podglądu';

  @override
  String get walletExportViewingKeyTileTitle => 'Eksportuj klucz podglądu';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Udostępnij kopię portfela tylko do podglądu — może zobaczyć Twoją historię, ale nie może wydawać środków.';

  @override
  String get walletExportViewingKeyWarning =>
      'Ten klucz pozwala każdemu, kto go posiada, zobaczyć wszystko, co ten portfel kiedykolwiek otrzymał i wysłał — a także wszystko, co otrzyma i wyśle w przyszłości. Nie pozwala wydawać Twoich środków ani odzyskać Twojego portfela. Udostępniaj go tylko komuś, komu ufasz na tyle, by pokazać mu pełną historię — na przykład księgowemu lub własnemu drugiemu urządzeniu. Jedynym sposobem na cofnięcie dostępu w przyszłości jest przeniesienie środków do nowego portfela.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Ten klucz pozwala każdemu, kto go posiada, zobaczyć wszystko, co ten portfel kiedykolwiek otrzymał i wysłał — a także wszystko, co otrzyma i wyśle w przyszłości. Nie pozwala wydawać Twoich środków ani odzyskać Twojego portfela. Udostępniaj go tylko komuś, komu ufasz na tyle, by pokazać mu pełną historię — na przykład księgowemu lub własnemu drugiemu urządzeniu. Po udostępnieniu nie ma możliwości cofnięcia dostępu.';

  @override
  String get walletExportViewingKeyReveal => 'Pokaż klucz podglądu';

  @override
  String get walletExportViewingKeyRetry => 'Spróbuj ponownie';

  @override
  String get walletExportViewingKeyRevealing =>
      'Przygotowywanie Twojego klucza podglądu…';

  @override
  String get walletExportViewingKeyFailed =>
      'Nie udało się teraz pokazać Twojego klucza podglądu. Spróbuj ponownie za chwilę.';

  @override
  String get walletExportViewingKeyQrLabel => 'Kod QR klucza podglądu';

  @override
  String get walletExportViewingKeyCopy => 'Skopiuj klucz podglądu';

  @override
  String get walletExportViewingKeyCopied => 'Skopiowano klucz podglądu';

  @override
  String get walletExportViewingKeyDone => 'Gotowe';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Zrzuty ekranu są wyłączone na tym ekranie.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Upewnij się, że nikt nie może zobaczyć Twojego ekranu.';

  @override
  String get walletWatchOnlySectionTitle => 'O tym portfelu tylko do podglądu';

  @override
  String get walletWatchOnlyAboutBody =>
      'To portfel tylko do podglądu. Został skonfigurowany na podstawie klucza podglądu, dlatego widzi Twoje saldo i historię, ale nie posiada kluczy wydatkowania — nie ma tu niczego do wykonania kopii zapasowej, a on sam nie może wysyłać środków.';

  @override
  String get walletWatchOnlyBadge => 'Tylko podgląd';

  @override
  String get walletOnboardingFailedTitle =>
      'Konfiguracja portfela nie została ukończona';

  @override
  String get walletOnboardingRetry => 'Spróbuj ponownie';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'Bezpieczna pamięć telefonu nie odpowiada. Odblokuj urządzenie i spróbuj ponownie. Jeśli problem się powtarza, uruchom telefon ponownie.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Ten portfel jest otwarty w innym oknie lub aplikacji, albo wciąż kończy poprzednią operację. Zamknij inne okno, które go używa — lub odczekaj chwilę — a następnie spróbuj ponownie.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'Bezpieczny klucz tego portfela nie jest już dostępny, więc nie można go otworzyć na tym urządzeniu. Twoje środki są bezpieczne — przywróć portfel z frazy odzyskiwania, aby je odzyskać.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Przywróć z frazy odzyskiwania';

  @override
  String get walletOnboardingRecoverConfirmTitle => 'Przywrócić ten portfel?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Zanim przejdziesz dalej, upewnij się, że masz swoją frazę odzyskiwania — będzie potrzebna na następnym ekranie do odzyskania środków. Twoje środki są bezpieczne w blockchainie i kontrolowane przez tę frazę. Ta operacja usuwa z urządzenia nieczytelne dane portfela, aby można je było odbudować.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Anuluj';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Za mało wolnego miejsca, aby skonfigurować portfel. Zwolnij trochę miejsca i spróbuj ponownie.';

  @override
  String get walletOnboardingFailedNoVault =>
      'To urządzenie nie ma bezpiecznego magazynu kluczy, więc portfel nie może tutaj chronić Twojej frazy odzyskiwania.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Nie udało się połączyć z siecią podczas konfiguracji. Sprawdź połączenie i spróbuj ponownie.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'Konfiguracja portfela nie została ukończona. Spróbuj ponownie, aby ją dokończyć — nic nie zostało utracone.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Coś poszło nie tak podczas konfiguracji portfela. Spróbuj ponownie.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'Konfiguracja portfela w tej aplikacji jest nieprawidłowa, więc portfel nie może się uruchomić. Ponowna próba nie pomoże — zgłoś to twórcy aplikacji. Twoje środki nie są zagrożone.';

  @override
  String get walletSendButton => 'Wyślij';

  @override
  String get walletSendSyncNotRunning =>
      'Synchronizacja nie działa — Twoje saldo dostępne do wydania nie może się zaktualizować';

  @override
  String get walletSendWaitingForFunds =>
      'Synchronizacja wciąż trwa — będziesz mógł wysłać, gdy będziesz mieć saldo dostępne do wydania';

  @override
  String get walletSendNoSpendableYet => 'Brak jeszcze dostępnego salda';

  @override
  String get walletSendSyncUnavailable =>
      'Będziesz mógł wysłać środki, gdy synchronizacja zostanie wznowiona';

  @override
  String get walletSendTitle => 'Wyślij';

  @override
  String get walletSendUnavailable =>
      'Portfel nie jest teraz gotowy. Wróć i spróbuj ponownie.';

  @override
  String get walletSendWatchOnly =>
      'To jest portfel tylko do podglądu. Może wyświetlać saldo i odbierać płatności, ale nie ma kluczy wydatkowania — więc nie może wysyłać.';

  @override
  String get walletSendExpiredTitle => 'To żądanie płatności wygasło';

  @override
  String get walletSendExpiredBody =>
      'Ekran wysyłania otwierał się dłużej niż pięć sekund, więc aplikacji przekazano, że nic nie zostało wysłane. Ta odpowiedź jest ostateczna: tego żądania nie da się opłacić stąd. Aby zapłacić, zacznij od nowa w aplikacji.';

  @override
  String get walletSendFaultWatchOnly =>
      'To jest portfel tylko do podglądu — nie ma kluczy wydatkowania, więc nie może wysyłać.';

  @override
  String walletSendAvailable(String amount) {
    return 'Dostępne do wysłania: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Dostępne do wysłania: $amount ZEC — saldo wciąż nadrabia zaległości';
  }

  @override
  String get walletSendRecipientLabel => 'Adres odbiorcy';

  @override
  String get walletSendRecipientHint =>
      'Adres Zcash (zaczyna się od u, z lub t)';

  @override
  String get walletSendRecipientLocked => 'Nie można zmienić odbiorcy tutaj';

  @override
  String get walletSendAmountLabel => 'Kwota (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Notatka (opcjonalnie)';

  @override
  String get walletSendMemoHint =>
      'Dostarczana tylko do odbiorców chronionych (prywatnych)';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Notatki wymagają odbiorcy chronionego. Ten adres publiczny nie może jej otrzymać.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Ta płatność zawiera już odniesienie od aplikacji, więc nie może mieć także pisemnej notatki.';

  @override
  String get walletSendMachineMemoTitle => 'Aplikacja dołącza odniesienie';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Podaje, że służy to do: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Pozostanie przy transakcji i nie będzie można go później usunąć. Portfel nie może sprawdzić jego zawartości.';

  @override
  String get walletSendRecipientShielded => 'Chroniony · prywatny';

  @override
  String get walletSendRecipientTransparent => 'Publiczny';

  @override
  String get walletSendRecipientInvalid =>
      'To nie wygląda na prawidłowy adres Zcash.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Ten adres jest przeznaczony dla innej sieci Zcash.';

  @override
  String get walletSendReviewButton => 'Sprawdź płatność';

  @override
  String get walletSendQueueButton => 'Dodaj do kolejki na później';

  @override
  String get walletSendQueueHint =>
      'Płatność w kolejce czeka w Zapisane i oczekujące, gdzie możesz ją wysłać lub anulować. Opłata sieciowa zostanie obliczona w momencie wysłania.';

  @override
  String get walletSendPreparing => 'Przygotowywanie płatności…';

  @override
  String get walletSendSubmitting => 'Wysyłanie…';

  @override
  String get walletSendQueuing => 'Dodawanie do kolejki…';

  @override
  String get walletSendReviewTitle => 'Potwierdź płatność';

  @override
  String get walletSendTotalLabel => 'Razem';

  @override
  String get walletSendFeeLabel => 'Opłata sieciowa';

  @override
  String get walletSendChangeLabel => 'Zwrócona reszta';

  @override
  String get walletSendDeshieldTitle => 'Ta płatność nie jest prywatna';

  @override
  String get walletSendDeshieldBody =>
      'Wysyła na adres publiczny, więc kwota i odbiorca będą publicznie widoczni w blockchainie Zcash.';

  @override
  String get walletSendPublicAckLabel =>
      'Rozumiem, że ta płatność będzie publiczna.';

  @override
  String get walletSendConfirmButton => 'Wyślij teraz';

  @override
  String get walletSendBackButton => 'Wstecz';

  @override
  String get walletSendSelfSendNote =>
      'Wysyłasz do własnego portfela. Opłata sieciowa nadal obowiązuje.';

  @override
  String get walletSendLargeConfirmTitle => 'Wysłać dużą kwotę?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'To niemal całe Twoje saldo. Wysłanej płatności nie można cofnąć.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'To duża płatność. Wysłanej płatności nie można cofnąć.';

  @override
  String get walletSendLargeConfirmBoth =>
      'To duża płatność — niemal całe Twoje saldo. Wysłanej płatności nie można cofnąć.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Wyślij $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Wróć';

  @override
  String get walletSendSentTitle => 'Płatność wysłana';

  @override
  String get walletSendSentBody =>
      'Twoja płatność została rozgłoszona w sieci.';

  @override
  String get walletSendSavedTitle => 'Zapisano — dokończymy wysyłanie';

  @override
  String get walletSendSavedBody =>
      'Twoja płatność nie mogła teraz zostać wysłana, więc została zapisana, a Twój portfel wyśle ją przy jednej z późniejszych synchronizacji. Nic nie zostało utracone.';

  @override
  String get walletSendKeptTitle => 'Zapisana';

  @override
  String get walletSendKeptBody =>
      'Twój portfel zachował tę transakcję, ale nie zobowiązał się do wysłania jej samodzielnie. Sprawdź jej stan w sekcji Aktywność.';

  @override
  String get walletSendPartialBody =>
      'Część Twojej płatności została wysłana; Twój portfel dokończy resztę przy jednej z późniejszych synchronizacji. Nic nie zostało utracone.';

  @override
  String get walletSendInMotionTitle => 'Płatność w trakcie realizacji';

  @override
  String get walletSendInMotionBody =>
      'Twoja płatność się rozpoczęła i przechodzi przez adres jednorazowy kontrolowany przez Twój portfel. Nie wysyłaj jej ponownie. Jeśli nie zostanie dokończona, możesz odzyskać środki z ekranu portfela.';

  @override
  String get walletSendAlreadyTitle => 'Już wysłano';

  @override
  String get walletSendAlreadyBody =>
      'Ta płatność została już wysłana — nie zostanie wysłana ponownie.';

  @override
  String get walletSendFailedTitle => 'Nie udało się zrealizować płatności';

  @override
  String get walletSendFailedBody =>
      'Coś poszło nie tak podczas realizacji tej płatności i nic nie zostało wysłane. Możesz spróbować ponownie.';

  @override
  String get walletSendTryAgain => 'Spróbuj ponownie';

  @override
  String get walletSendDone => 'Gotowe';

  @override
  String get walletSendAnother => 'Wyślij kolejną';

  @override
  String get walletSendQueuedTitle => 'Dodano do kolejki wysyłania';

  @override
  String get walletSendQueuedBody =>
      'Ta płatność jest zapisana. Znajdziesz ją w sekcji „Zapisane i oczekujące”, gdzie możesz ją wysłać teraz lub anulować.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Za mało dostępnego salda — masz $available ZEC, a potrzeba $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Sieć Zcash została zaktualizowana i ta aplikacja wymaga aktualizacji, zanim będzie mogła wysyłać. Twoje środki są bezpieczne.';

  @override
  String get walletSyncUpToDateLimited =>
      'Zsynchronizowano tak daleko, jak ta wersja potrafi odczytać';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Sieć Zcash została zaktualizowana. Ta wersja przeskanowała wszystko, co potrafi odczytać, ale nowsze bloki mogą zawierać środki, których jeszcze nie może pokazać, a notatki do ostatnich płatności są niedostępne. Zaktualizuj aplikację, aby zobaczyć wszystko.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Aktualne, ale ten serwer nie obsługuje wszystkich puli';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Ten serwer odrzuca, wstrzymuje lub błędnie raportuje jedną z chronionych puli Zcash. Środków otrzymanych w tej puli nie można wydać przez ten serwer, a wyświetlane saldo to wartość minimalna. Przełącz się na inny serwer, aby ich użyć — to nie jest problem z połączeniem.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: ten serwer odmawia jej obsługi';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: ten serwer wstrzymuje jej część';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: ten serwer podaje o niej błędne dane';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: nie wiadomo, czy ten serwer ją obsługuje';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Aktualne względem tego serwera, ale serwer jest w tyle za siecią';

  @override
  String get walletSyncExplainEndpointBehind =>
      'Łańcuch tego serwera kończy się na bloku, który sieć minęła jeszcze przed zbudowaniem tej wersji aplikacji, więc saldo jest aktualne tylko do tego bloku. Nowe płatności do Ciebie mogą jeszcze nie być widoczne, a płatność wysłana stąd może nie dotrzeć. Przełącz się na inny serwer, aby nadrobić zaległości — to nie jest problem z połączeniem.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Oczekiwanie na aktualizację aplikacji — Twoje środki są bezpieczne i nic nie zostało wysłane.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Oczekiwanie na serwer, który podaje wersję sieci — przełącz serwer. Twoje środki są bezpieczne i nic nie zostało wysłane.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Oczekiwanie na serwer, który podaje wersję sieci. Jeśli data i godzina na tym urządzeniu są błędne, najpierw je popraw — a potem przełącz serwer. Twoje środki są bezpieczne i nic nie zostało wysłane.';

  @override
  String get walletSyncUnverified =>
      'Zsynchronizowano, ale ten serwer nie podaje wersji sieci';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'Wysyłanie działa jeszcze przez około $hours godziny — potem przełącz serwer.',
      many:
          'Wysyłanie działa jeszcze przez około $hours godzin — potem przełącz serwer.',
      few:
          'Wysyłanie działa jeszcze przez około $hours godziny — potem przełącz serwer.',
      one:
          'Wysyłanie działa jeszcze przez około $hours godzinę — potem przełącz serwer.',
      zero:
          'Wysyłanie działa jeszcze przez mniej niż godzinę — potem przełącz serwer.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'Wysyłanie działa jeszcze przez około $blocks bloków — potem przełącz serwer.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Ten serwer od $blocks bloków nie podaje wersji sieci, więc aplikacja nie może potwierdzić, że wysyłanie jest bezpieczne. Przełącz się na inny serwer.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Ten serwer od doby nie podaje wersji sieci, więc aplikacja nie może potwierdzić, że wysyłanie jest bezpieczne. Jeśli data i godzina na tym urządzeniu są błędne, najpierw je popraw — a potem przełącz się na serwer, który podaje wersję sieci.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Ten serwer nigdy nie podał wersji sieci, więc aplikacja nie może potwierdzić, że wysyłanie jest bezpieczne. Przełącz się na inny serwer.';

  @override
  String get walletSyncExplainUnverified =>
      'Ten serwer nie mówi, w której wersji sieci Zcash działa, więc aplikacja nie może potwierdzić, że podpisana przez nią płatność zostanie przyjęta. Twoje saldo jest aktualne. Przełącz się na inny serwer — to nie jest problem z połączeniem.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Ten serwer nie mówi, w której wersji sieci Zcash działa, więc aplikacja nie może potwierdzić, że podpisana przez nią płatność zostanie przyjęta. Wciąż też dostarczał bloki, które ten portfel musiał potem wycofać, więc Twoje saldo może nie być aktualne. Przełącz się na inny serwer — to nie jest problem z połączeniem.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Ten serwer wciąż też dostarcza bloki, które ten portfel musi potem wycofać — zmień serwer.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Saldo wciąż nadrabia zaległości — w miarę synchronizacji portfela może stać się dostępne więcej środków.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC wciąż napływa i będzie dostępne do wydania, gdy portfel nadrobi zaległości.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Wprowadź kwotę do wysłania.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Wprowadź kwotę jako liczbę, na przykład 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'ZEC ma co najwyżej 8 miejsc po przecinku.';

  @override
  String get walletSendFaultAmountNotPositive =>
      'Wprowadź kwotę większą niż zero.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Ta kwota przekracza całkowitą podaż ZEC.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Ta aplikacja obecnie ogranicza wysyłkę do $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'To nie wygląda na prawidłowy adres Zcash dla tej sieci. Sprawdź go i spróbuj ponownie.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Ten odbiorca nie może otrzymać notatki. Usuń notatkę lub wyślij na adres chroniony (prywatny).';

  @override
  String get walletSendFaultMemoTooLong =>
      'Twoja notatka jest za długa. Skróć ją i spróbuj ponownie.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Tej notatki nie można wysłać. Usuń ją i spróbuj ponownie.';

  @override
  String get walletSendFaultMemoConflict =>
      'Nie udało się wysłać tej płatności — aplikacja dołączyła do niej dwie notatki. Nic nie zostało wysłane.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Ten adres jest przeznaczony dla innej sieci.';

  @override
  String get walletSendFaultUriInvalid =>
      'Nie udało się utworzyć tej płatności. Sprawdź adres i kwotę.';

  @override
  String get walletSendFaultNotSynced =>
      'Portfel nie jest jeszcze wystarczająco zsynchronizowany. Poczekaj, aż synchronizacja nadrobi zaległości, lub dodaj tę płatność do kolejki na później.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Portfel nie jest jeszcze wystarczająco zsynchronizowany. Poczekaj, aż synchronizacja nadrobi zaległości.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Portfel nie jest jeszcze wystarczająco zsynchronizowany, a synchronizacja teraz nie działa. Sprawdź stan synchronizacji na ekranie portfela.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Kwoty wygasły podczas przeglądania. Sprawdź płatność ponownie.';

  @override
  String get walletSendFaultQueueFull =>
      'Zbyt wiele płatności czeka na wysłanie. Poczekaj, aż zostaną wysłane, a następnie spróbuj ponownie.';

  @override
  String get walletSendFaultWalletBusy =>
      'Portfel jest teraz zajęty. Spróbuj ponownie za chwilę.';

  @override
  String get walletSendFaultStorageFull =>
      'Za mało wolnego miejsca, aby ukończyć tę wysyłkę. Zwolnij trochę miejsca i spróbuj ponownie.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Zbyt wiele adresów jednorazowych jest obecnie w użyciu. Część z nich może się zwolnić, gdy transakcje zostaną potwierdzone, ale może to nie ustąpić samoistnie. Twoje środki są bezpieczne.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Nie udało się przygotować tej płatności. Sprawdź szczegóły i spróbuj ponownie.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Nie udało się teraz przygotować tej płatności. Spróbuj ponownie za chwilę.';

  @override
  String get walletSwapButton => 'Wymiana';

  @override
  String get walletSwapTitle => 'Wymień ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'Portfel nie jest teraz gotowy. Wróć i spróbuj ponownie.';

  @override
  String get walletSwapUnavailableOff => 'Wymiana jest obecnie niedostępna.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'To jest portfel tylko do podglądu — nie może wymieniać.';

  @override
  String get walletSwapDone => 'Gotowe';

  @override
  String get walletSwapBackToWallet => 'Powrót do portfela';

  @override
  String walletSwapAvailable(String amount) {
    return 'Dostępne do wymiany: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Dostępne do wymiany: $amount ZEC — saldo wciąż nadrabia zaległości';
  }

  @override
  String get walletSwapAssetLabel => 'Otrzymywany aktyw';

  @override
  String get walletSwapAmountLabel => 'Kwota do wymiany (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Adres docelowy';

  @override
  String get walletSwapDestinationHint =>
      'Twój adres odbiorczy w sieci docelowej';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Twój adres odbiorczy w sieci $chain';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Adres w sieci $chain — tam trafi wymieniony aktyw. Sprawdź dokładnie, czy sieć jest właściwa.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Zeskanuj kod QR adresu docelowego';

  @override
  String get walletSwapTargetAssetHint => 'Wybierz aktyw do otrzymania';

  @override
  String get walletSwapQuoteButton => 'Uzyskaj wycenę';

  @override
  String get walletSwapQuoting => 'Uzyskiwanie wyceny…';

  @override
  String get walletSwapExecuting => 'Rozpoczynanie wymiany…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Wciąż trwa — wymiana się rozpoczyna. Może to potrwać do minuty.';

  @override
  String get walletSwapReviewTitle => 'Potwierdź wymianę';

  @override
  String get walletSwapYouSendLabel => 'Wysyłasz';

  @override
  String get walletSwapYouReceiveLabel => 'Otrzymasz co najmniej';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Opłata sieciowa';

  @override
  String get walletSwapNetworkFeeValue => 'Doliczana przy wysyłce wpłaty';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Wycena ważna jeszcze przez około $time — potwierdź, zanim wygaśnie.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Wycena ważna jeszcze przez mniej niż minutę — potwierdź, zanim wygaśnie.';

  @override
  String get walletSwapQuoteExpired =>
      'Ta wycena wygasła. Wróć i uzyskaj nową — jej kurs nie jest już gwarantowany, a wysłanie teraz grozi zwrotem środków.';

  @override
  String get walletCountdownUnderMinute => 'mniej niż minuta';

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
    return '$hours godz. $minutes min';
  }

  @override
  String get walletSwapDeshieldTitle => 'Ta wymiana nie jest prywatna';

  @override
  String get walletSwapDeshieldBody =>
      'Wymiana wychodząca zdejmuje ochronę z Twojego ZEC — wpłata jest publiczną transakcją, a strona dostawcy jest publiczna w jego sieci.';

  @override
  String get walletSwapDiscloseTitle => 'Co zobaczy dostawca wymiany';

  @override
  String get walletSwapDiscloseAmounts => 'Kwoty po obu stronach';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Że to ZEC i otrzymywany aktyw stanowią jedną wymianę';

  @override
  String get walletSwapDiscloseDestination => 'Twój adres docelowy';

  @override
  String get walletSwapDiscloseSource => 'Twój adres źródłowy';

  @override
  String get walletSwapDiscloseIp =>
      'Twój adres IP (chyba że korzystasz z Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'Inne szczegóły tej wymiany';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'Własne transakcje dostawcy są publiczne w jego sieci';

  @override
  String get walletSwapAckLabel =>
      'Rozumiem, że dostawca zobaczy powyższe informacje.';

  @override
  String get walletSwapConfirmButton => 'Rozpocznij wymianę';

  @override
  String get walletSwapBackButton => 'Wstecz';

  @override
  String get walletSwapStatusPendingTitle => 'Wymiana rozpoczęta';

  @override
  String get walletSwapStatusCheckingTitle => 'Sprawdzanie statusu wymiany…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Twój portfel wysyła wpłatę ZEC do dostawcy. Jeśli jesteś chwilowo offline, zostanie ona wysłana automatycznie, gdy tylko wrócisz do sieci — ale okno na wysłanie jest krótkie, a jeśli zamknie się wcześniej, wymiana po prostu się kończy i nic nie zostaje wymienione. Twój ZEC pozostaje Twój, choć pokazanie go jako dostępnego do wydania może zająć do godziny.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'Oczekiwanie na Twoją wpłatę. Jeśli środki z innego portfela nie zostały jeszcze wysłane, wyślij je, zanim wycena wygaśnie.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Ta wymiana wciąż czeka na wpłatę. Instrukcje wpłaty nie są już dostępne na tym urządzeniu — jeśli środki zostały już wysłane, zostaną wykryte; jeśli nie, pozwól tej wymianie wygasnąć i rozpocznij nową.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'Okno wpłaty kończy się $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'Okno wpłaty minęło. Jeśli wpłata nie została wysłana na czas, wymiana się kończy, a Twój ZEC pozostaje w portfelu.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'Okno wpłaty minęło. Jeśli nie wysłałeś jeszcze swojej wpłaty, ta wymiana po prostu się kończy — pobierz nową wycenę, gdy będziesz gotowy.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Wymiany w toku',
      one: 'Wymiana w toku',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'Twój ZEC jest w drodze do dostawcy.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'Oczekiwanie na dotarcie Twojej wpłaty do dostawcy.';

  @override
  String get walletSwapInFlightRowGeneric => 'Wymiana jest w toku.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'Okno wpłaty minęło — sprawdź status tej wymiany.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Ta wymiana nie osiągnęła tu jeszcze potwierdzonego wyniku — otwórz ją, aby sprawdzić. Każdy ZEC wracający do tego portfela pojawi się w Twoim saldzie po synchronizacji.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Ta wymiana nie osiągnęła tu jeszcze potwierdzonego wyniku — otwórz ją, aby sprawdzić. Każdy ZEC dostarczony przez tę wymianę do tego portfela pojawi się w Twoim saldzie po synchronizacji.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Wymiana zakończona.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Wymiana zwrócona.';

  @override
  String get walletSwapRowOutcomeFailed => 'Wymiana nieukończona.';

  @override
  String get walletSwapRemove => 'Usuń';

  @override
  String get walletSwapRemoveTitle => 'Usunąć tę wymianę z listy?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'To tylko usuwa wymianę z tej listy — nie anuluje wymiany, a ten portfel przestanie śledzić jej zwrot. ZEC zwrócony później nadal należy do tego portfela; pełne ponowne skanowanie może go znaleźć.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'To tylko usuwa wymianę z tej listy — nie anuluje wymiany, a ten portfel przestanie śledzić przychodzący ZEC. ZEC dostarczony później nadal należy do tego portfela; pełne ponowne skanowanie może go znaleźć. Jeśli wymiana zostanie zamiast tego zwrócona, zwrot wraca w aktywie, który wysłałeś, poza tym portfelem.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'To tylko usuwa wymianę z tej listy — nie anuluje wymiany, a ten portfel przestanie śledzić ZEC wciąż z niej napływający. ZEC, który napłynie później, nadal należy do tego portfela; pełne ponowne skanowanie może go znaleźć.';

  @override
  String get walletSwapRemoveBodyDone => 'To usuwa zakończoną wymianę z listy.';

  @override
  String get walletSwapRemoveCancel => 'Anuluj';

  @override
  String get walletSwapRemoveConfirm => 'Usuń';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Rozpoczęto $time';
  }

  @override
  String get walletSwapViewSwap => 'Pokaż wymianę';

  @override
  String get walletSwapsInFlightError =>
      'Nie udało się teraz wczytać wymian w toku.';

  @override
  String get walletSwapsInFlightRetry => 'Spróbuj ponownie';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Próbowanie…';

  @override
  String get walletSwapStartAnother => 'Rozpocznij kolejną wymianę';

  @override
  String get walletSwapStatusUnderTitle => 'Oczekiwanie na pełną wpłatę';

  @override
  String get walletSwapStatusUnderBody =>
      'Część wpłaty dotarła. Reszta jest w trakcie realizacji albo dostawca zwróci środki.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Część Twojej wpłaty dotarła. Wyślij brakującą kwotę przed terminem, w przeciwnym razie dostawca zwróci to, co dotarło.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Otrzymano $received; wciąż brakuje $missing. Okno wpłaty kończy się: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Wpłata otrzymana';

  @override
  String get walletSwapStatusDetectedBody =>
      'Dostawca otrzymał Twoją wpłatę i przetworzy wymianę.';

  @override
  String get walletSwapStatusProcessingTitle => 'Przetwarzanie wymiany';

  @override
  String get walletSwapStatusProcessingBody =>
      'Dostawca finalizuje Twoją wymianę.';

  @override
  String get walletSwapStatusSuccessTitle => 'Wymiana zakończona';

  @override
  String get walletSwapStatusSuccessBody =>
      'Twoja wymiana zakończyła się pomyślnie.';

  @override
  String get walletSwapStatusRefundedTitle => 'Wymiana zwrócona';

  @override
  String get walletSwapStatusRefundedBody =>
      'Wymiana nie została ukończona, więc dostawca odesłał środki na Twój adres zwrotu.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'Wymiana nie została ukończona, więc dostawca odesłał Twój ZEC do tego portfela. Środki dotrą jako niechronione i pojawią się w Twoim saldzie po najbliższej synchronizacji portfela — może to chwilę potrwać.';

  @override
  String get walletSwapStatusFailedTitle => 'Wymiana nieudana';

  @override
  String get walletSwapStatusFailedBody =>
      'Nie udało się ukończyć wymiany. Wszelkie wpłacone środki zostaną rozliczone lub zwrócone po stronie dostawcy.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Nie znaleziono wymiany';

  @override
  String get walletSwapStatusNotFoundBody =>
      'Dostawca nie ma już zapisu tej wymiany — najprawdopodobniej wygasła. Jeśli wpłata została dokonana, dostawca powinien zwrócić ją na adres zwrotu. Wymiana pozostaje na Twojej liście, a ten portfel nadal śledzi jej ZEC na wypadek, gdyby jednak dotarł; możesz usunąć ją z listy w dowolnym momencie.';

  @override
  String get walletSwapStatusUnknownTitle => 'Status niedostępny';

  @override
  String get walletSwapStatusUnknownBody =>
      'Nie możemy teraz odczytać statusu tej wymiany.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'Śledzenie niedostępne';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'Wymiana jest wyłączona, więc nie możemy jej tutaj śledzić. Wszelkie środki zostaną rozliczone lub zwrócone po stronie dostawcy.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'Wymiana jest tutaj wyłączona, więc nie można jej teraz śledzić. Jeśli została zwrócona, ZEC wraca do tego portfela — pojawi się w Twoim saldzie po ponownym włączeniu wymiany i zsynchronizowaniu portfela.';

  @override
  String get walletSwapTrackingError => 'Nie udało się śledzić tej wymiany.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Nie udało się otworzyć śledzenia tej wymiany. Sama wymiana może nadal być w toku — wszelkie wpłacone środki zostaną rozliczone lub zwrócone po stronie dostawcy.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Wprowadź adres, na który chcesz otrzymać wymieniony aktyw.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Ten adres docelowy jest nieprawidłowy dla tego aktywu. Sprawdź go i spróbuj ponownie.';

  @override
  String get walletSwapFaultExpired =>
      'Ta wycena wygasła. Uzyskaj nową wycenę, aby kontynuować.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'Cena dostawcy wykroczyła poza Twój limit, więc wymiana została zatrzymana, zanim cokolwiek zostało przesłane. Spróbuj ponownie.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'Limit poślizgu cenowego jest zbyt wysoki dla bezpiecznej wymiany. Spróbuj ponownie.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'Dostawca wymiany jest obecnie niedostępny. Spróbuj ponownie za chwilę.';

  @override
  String get walletSwapFaultConnection =>
      'Nie udało się połączyć z usługą wymiany. Sprawdź połączenie z internetem i spróbuj ponownie.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'Dostawca wymiany zwrócił nieoczekiwaną odpowiedź, więc wymiana została zatrzymana. Spróbuj ponownie.';

  @override
  String get walletSwapFaultSwapOff => 'Wymiana jest teraz wyłączona.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Nie udało się wysłać wpłaty, więc nic nie opuściło Twojego portfela. Uzyskaj nową wycenę, aby spróbować ponownie.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Wymiana jest już w toku. Możesz rozpocząć nową, gdy ta zostanie w pełni rozliczona lub jej wycena wygaśnie — może to potrwać jakiś czas.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Ten portfel nie może jeszcze ustawić adresu zwrotu — zwykle oznacza to po prostu, że pierwsza synchronizacja się nie zakończyła. Poczekaj na zakończenie synchronizacji, a następnie spróbuj ponownie.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Ten portfel nie może jeszcze ustawić adresu odbiorczego dla tej wymiany — zwykle oznacza to po prostu, że pierwsza synchronizacja się nie zakończyła. Poczekaj na zakończenie synchronizacji, a następnie spróbuj ponownie.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'Wymiana nie mogła rozpocząć się na czas — połączenie mogło być wolne albo portfel był zajęty. Uzyskaj nową wycenę i spróbuj ponownie.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'Portfel jest chwilowo zajęty. Spróbuj ponownie.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Ta wycena nie zgadza się z tą, którą wystawił Twój portfel, więc nic nie zostało wysłane. Uzyskaj nową wycenę i spróbuj ponownie.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Ta wymiana wymaga około $needed ZEC wraz z opłatą sieciową, ale obecnie dostępne jest tylko $spendable ZEC.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Ta aplikacja obecnie ogranicza wymianę do $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Ta wymiana wymaga około $needed ZEC wraz z opłatą sieciową, ale obecnie dostępne jest tylko $spendable ZEC. Twoje saldo wciąż nadrabia zaległości — wkrótce może stać się dostępne więcej środków.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'Portfel nie mógł bezpiecznie zapisać tej wymiany, więc nic nie zostało przesłane. Spróbuj ponownie.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Nie udało się przetworzyć tego żądania wymiany. Uzyskaj nową wycenę i spróbuj ponownie.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Nie udało się uzyskać wyceny wymiany. Sprawdź szczegóły i spróbuj ponownie.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Portfel nie jest teraz gotowy. Wróć i spróbuj ponownie.';

  @override
  String get walletSwapDirectionBuy => 'Kup ZEC';

  @override
  String get walletSwapDirectionSell => 'Sprzedaj ZEC';

  @override
  String get walletSwapRefundLabel => 'Twój adres zwrotu';

  @override
  String get walletSwapRefundHint =>
      'Gdzie wrócą Twoje środki, jeśli wymiana się nie powiedzie';

  @override
  String get walletSwapRefundHelper =>
      'W sieci, z której wysyłasz — nie adres Zcash.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Twój adres zwrotu w sieci $chain';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Adres w sieci $chain — tam wrócą Twoje środki, jeśli wymiana się nie powiedzie. Nie adres Zcash.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'O adresie zwrotu';

  @override
  String get walletSwapRefundInfoBody =>
      'Jeśli wymiana nie może zostać ukończona, dostawca odeśle Twoje środki na ten adres w sieci, z której zapłaciłeś. Podaj adres, który kontrolujesz — portfel nie może zweryfikować za Ciebie obcego adresu, więc sprawdź go dokładnie.';

  @override
  String get walletSwapRefundScanTooltip => 'Zeskanuj kod QR adresu zwrotu';

  @override
  String get walletSwapScanTitle => 'Skanuj adres';

  @override
  String get walletSwapScanInstruction => 'Skieruj aparat na kod QR adresu.';

  @override
  String get walletSwapScanManualEntry => 'Wprowadź ręcznie';

  @override
  String get walletSwapScanCancel => 'Anuluj';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Aparat niedostępny. Wprowadź adres ręcznie poniżej.';

  @override
  String get walletSwapSourceAssetLabel => 'Aktyw źródłowy wymiany';

  @override
  String get walletSwapSourceAssetHint => 'Wybierz aktyw';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Kwota do wysłania ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Kwota do wysłania';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol w sieci $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Wybierz aktyw źródłowy wymiany';

  @override
  String get walletSwapPickerTitleReceive => 'Wybierz aktyw do otrzymania';

  @override
  String get walletSwapPickerStale =>
      'Nie udało się odświeżyć listy aktywów — wyświetlana jest ostatnia znana lista.';

  @override
  String get walletSwapPickerEmpty =>
      'Obecnie brak dostępnych aktywów do wymiany. Spróbuj ponownie później.';

  @override
  String get walletSwapPickerSearchHint => 'Szukaj po nazwie lub sieci';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Żaden aktyw nie pasuje do „$query”.';
  }

  @override
  String get walletSwapPickerError =>
      'Nie udało się wczytać listy aktywów. Sprawdź połączenie i spróbuj ponownie.';

  @override
  String get walletSwapPickerRetry => 'Spróbuj ponownie';

  @override
  String get walletSwapSlippageLabel => 'Tolerancja poślizgu cenowego';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Niestandardowa';

  @override
  String get walletSwapSlippageCustomLabel => 'Niestandardowy poślizg cenowy';

  @override
  String get walletSwapSlippageMayFail =>
      'Bardzo niska — wymiana może się nie powieść, jeśli cena się zmieni.';

  @override
  String get walletSwapSlippageNormal => 'Bezpieczna tolerancja.';

  @override
  String get walletSwapSlippageRisky =>
      'Wysoka — możesz otrzymać zauważalnie mniej niż w wycenie.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Zbyt wysoka — wymiana zostanie odrzucona. Obniż ją do 10% lub mniej.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Otrzymasz co najmniej $zec ZEC — to Twój próg poślizgu cenowego wynoszący $slippage%. Ostateczna kwota nie spadnie poniżej tego progu.';
  }

  @override
  String get walletSwapIntoZecShieldTitle => 'Otrzymujesz ZEC na własny adres';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Dopóki nie obejmiesz jej ochroną — jedno dotknięcie, o które przypomnimy po dotarciu środków — otrzymana kwota jest przez chwilę publiczna i widoczna w blockchainie. Niewielka wpłata może pozostać publiczna, dopóki się nie skumuluje.';

  @override
  String get walletSwapRefundVerifyTitle => 'Zweryfikuj adres zwrotu';

  @override
  String get walletSwapRefundVerifyBody =>
      'Sprawdź go znak po znaku — tam wrócą Twoje środki, jeśli wymiana się nie powiedzie. Portfel nie może zweryfikować za Ciebie obcego adresu.';

  @override
  String get walletSwapRefundVerifyAck =>
      'Potwierdzam poprawność mojego adresu zwrotu.';

  @override
  String get walletSwapPayoutVerifyTitle => 'Zweryfikuj adres odbioru';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Sprawdź go znak po znaku — na ten adres otrzymasz $asset. Portfel nie może zweryfikować za Ciebie obcego adresu.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'Potwierdzam poprawność mojego adresu odbioru.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'Wymiana jest tutaj wyłączona. Wszelkie ZEC już w drodze pojawi się w portfelu po następnej synchronizacji.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Wprowadź kwotę, którą chcesz wymienić.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Wprowadź adres zwrotu w sieci źródłowej.';

  @override
  String get walletSwapDepositTitle => 'Wyślij płatność';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Wyślij dokładnie $amount $asset w sieci $chain na adres poniżej.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Wyślij dokładną kwotę. Wysłanie mniejszej kwoty lub po zamknięciu okna czasowego oznacza, że dostawca zwróci środki na Twój adres zwrotu.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Okno wpłaty: pozostało $time';
  }

  @override
  String get walletSwapDepositExpired =>
      'To okno wpłaty zostało zamknięte. Nie wysyłaj teraz środków — rozpocznij nową wymianę. Jeśli już je wysłałeś, dostawca powinien zwrócić je na Twój adres zwrotu.';

  @override
  String get walletSwapDepositQrLabel => 'Kod QR adresu wpłaty';

  @override
  String get walletSwapDepositAddressLabel => 'Adres wpłaty';

  @override
  String get walletSwapDepositCopy => 'Kopiuj adres wpłaty';

  @override
  String get walletSwapDepositCopied => 'Skopiowano adres wpłaty';

  @override
  String get walletSwapDepositMemoRequired => 'Ta wpłata wymaga notatki / tagu';

  @override
  String get walletSwapDepositMemoWarning =>
      'MUSISZ dołączyć dokładnie tę notatkę do wpłaty. Wysłanie bez niej — lub z błędną notatką — może spowodować trwałą utratę środków.';

  @override
  String get walletSwapDepositMemoLabel => 'Notatka / tag wpłaty';

  @override
  String get walletSwapDepositMemoCopy => 'Kopiuj notatkę';

  @override
  String get walletSwapDepositMemoCopied => 'Skopiowano notatkę';

  @override
  String get walletSwapDepositSent => 'Wysłano środki';

  @override
  String get walletSwapDepositBackTitle => 'Opuścić ten ekran?';

  @override
  String get walletSwapDepositBackBody =>
      'To nie anuluje wymiany — będzie ona kontynuowana w tle. Do zapłaty potrzebny będzie jednak adres wpłaty, więc skopiuj go najpierw, jeśli jeszcze tego nie zrobiłeś.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'To nie anuluje wymiany — będzie ona kontynuowana w tle. Okno wpłaty zostało zamknięte. Nie wysyłaj teraz środków na adres wpłaty. Jeśli już je wysłałeś, dostawca powinien zwrócić je na Twój adres zwrotu.';

  @override
  String get walletSwapDepositBackStay => 'Zostań';

  @override
  String get walletSwapDepositBackLeave => 'Opuść';

  @override
  String get walletReceive => 'Odbierz';

  @override
  String get walletReceiveSubtitle =>
      'Udostępnij ten adres, aby otrzymać ZEC. Można go bezpiecznie udostępniać publicznie.';

  @override
  String get walletReceiveCopy => 'Kopiuj adres';

  @override
  String get walletReceiveCopied => 'Skopiowano adres';

  @override
  String get walletReceiveUnavailable => 'Portfel nie jest jeszcze gotowy.';

  @override
  String get walletReceiveError =>
      'Nie udało się wczytać Twojego adresu. Spróbuj ponownie.';

  @override
  String get walletReceivePreparing => 'Przygotowywanie adresu…';

  @override
  String get walletReceivePreparingHint =>
      'Twój portfel przygotowuje ten adres na Twoim urządzeniu — może to chwilę potrwać, jeśli portfel jest zajęty inną pracą.';

  @override
  String get walletReceiveRetry => 'Spróbuj ponownie';

  @override
  String get walletReceiveQrLabel => 'Kod QR Twojego adresu odbiorczego';

  @override
  String get walletReceiveTypeShielded => 'Chroniony';

  @override
  String get walletReceiveTypeTransparent => 'Publiczny';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Udostępnij ten adres publiczny, aby otrzymać ZEC od nadawcy, który nie może zapłacić na adres chroniony.';

  @override
  String get walletReceiveTransparentWarning =>
      'To adres publiczny: jest widoczny w blockchainie i łączy ze sobą Twoje płatności w przypadku ponownego użycia. Preferuj adres chroniony; po otrzymaniu środków obejmij je ochroną.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'Kod QR Twojego publicznego adresu odbiorczego';

  @override
  String get walletReceiveFreshAddress => 'Użyj nowego adresu';

  @override
  String get walletReceiveFreshCaption =>
      'Nowy adres — nie można go połączyć z Twoimi innymi adresami. Płatności na niego nadal trafiają do tego portfela, a Twoje poprzednie adresy nadal działają. Nie zostanie tu ponownie wyświetlony — skopiuj go teraz.';

  @override
  String get walletReceiveFreshError =>
      'Nie udało się utworzyć nowego adresu. Spróbuj ponownie.';

  @override
  String get walletReceiveFreshBusy =>
      'Portfel jest teraz zajęty. Spróbuj ponownie za chwilę z nowym adresem.';

  @override
  String get walletReceiveShare => 'Udostępnij';

  @override
  String get walletReceiveRequestAmount => 'Poproś o kwotę';

  @override
  String get walletReceiveRequestAmountLabel => 'Kwota (opcjonalnie)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Nie zostanie tu ponownie wyświetlony — skopiuj go teraz.';

  @override
  String get walletSecurityMenuItem => 'Bezpieczeństwo…';

  @override
  String get securityTitle => 'Bezpieczeństwo';

  @override
  String get securityUnavailableBody =>
      'Ustawieniami bezpieczeństwa portfela zarządza ta aplikacja, a nie sam portfel.';

  @override
  String get securityCustodySectionTitle => 'Przechowywanie kluczy';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (sprzętowe)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (sprzętowe)';

  @override
  String get securityCustodyTierTee => 'Sprzętowy magazyn kluczy (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Programowy magazyn kluczy';

  @override
  String get securityCustodyTierKeychain =>
      'Pęk kluczy (szyfrowanie programowe)';

  @override
  String get securityCustodyTierNone => 'Brak sprzętowego magazynu kluczy';

  @override
  String get securityCustodyTierUnknown => 'Nieznany';

  @override
  String get securityCustodyHardwareKey =>
      'Klucz, który blokuje ten portfel, jest przechowywany w bezpiecznym sprzęcie tego urządzenia i jest usuwany razem z portfelem.';

  @override
  String get securityCustodyBestEffort =>
      'Usunięcie kasuje klucze najlepiej, jak to możliwe; do czasu odzyskania pamięci przez urządzenie może pozostać krótkie okno umożliwiające odzyskanie danych metodami kryminalistycznymi. Dla pełnej pewności użyj dodatkowo funkcji urządzenia „Usuń całą zawartość”.';

  @override
  String get securityCustodyProbeError =>
      'Nie udało się odczytać statusu przechowywania kluczy. Cofnij się i spróbuj ponownie.';

  @override
  String get securityDeleteWalletButton => 'Usuń portfel';

  @override
  String get securityDeleteWalletSubtitle =>
      'Usuń ten portfel i jego klucz z tego urządzenia. Twoje środki pozostają w blockchainie i można je przywrócić z frazy odzyskiwania.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Usuń ten portfel i jego klucz z tego urządzenia. Nie posiada kluczy wydatkowania, więc nie ma tu niczego do wykonania kopii zapasowej — możesz dodać go ponownie w dowolnym momencie za pomocą klucza podglądu.';

  @override
  String get securityDeleteDialogTitle => 'Usunąć ten portfel?';

  @override
  String get securityDeleteDialogBody =>
      'Ta operacja usuwa portfel i jego klucz z tego urządzenia. Upewnij się, że masz kopię zapasową frazy odzyskiwania — to JEDYNY sposób na przywrócenie środków.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Ta operacja usuwa portfel i jego klucz z tego urządzenia. Nie posiada kluczy wydatkowania, więc nic nie wymaga kopii zapasowej — możesz dodać go ponownie później za pomocą klucza podglądu.';

  @override
  String get securityDeleteDialogConfirm => 'Usuń';

  @override
  String get securityDeleteDialogCancel => 'Anuluj';

  @override
  String get securityDeleteFailedSnack =>
      'Nie udało się usunąć portfela — pozostaje on bez zmian. Spróbuj ponownie.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Najpierw dokończ zmianę serwera — zakończy się lub zatrzyma w ciągu $seconds sekund. Następnie spróbuj ponownie usunąć portfel.';
  }

  @override
  String get walletParkedTitle => 'Zapisane i oczekujące';

  @override
  String get walletParkedSubtitle =>
      'Te płatności nie zostały jeszcze wysłane. Ich kwoty nadal są częścią Twojego salda.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Te płatności nie zostały jeszcze wysłane. Ich kwoty nadal są częścią Twojego salda — z wyjątkiem tych, które Twój portfel właśnie wysyła, których kwota mogła już zostać zarezerwowana.';

  @override
  String get walletParkedCancel => 'Anuluj';

  @override
  String get walletParkedPausedHint =>
      'Wstrzymane — ta płatność nie wyśle się sama. Twoje środki są bezpieczne. Wyślij ją teraz lub anuluj.';

  @override
  String get walletParkedRetryStale =>
      'Ta płatność już nie oczekuje. Sprawdź swoje oczekujące płatności i aktywność.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Ta płatność już nie oczekuje — Twój portfel może już ją wysyłać. Sprawdź Zapisane i oczekujące oraz aktywność.';

  @override
  String get walletReclaimExplainer =>
      'Wysyłanie za pomocą adresów jednorazowych jest zablokowane. Możesz je wznowić — to przenosi niewielką kwotę między Twoimi własnymi adresami i ją zwraca.';

  @override
  String get walletReclaimButton => 'Wznów wysyłanie';

  @override
  String get walletReclaimInProgress => 'Wznawianie…';

  @override
  String get walletReclaimConfirmTitle =>
      'Wznowić wysyłanie za pomocą adresów jednorazowych?';

  @override
  String get walletReclaimConfirmBody =>
      'To przenosi niewielką kwotę między Twoimi własnymi adresami, aby zwolnić wysyłanie za pomocą adresów jednorazowych, a następnie ją zwraca. Kosztuje to parę opłat sieciowych. Gdy się potwierdzi, odzyskaj przeniesioną kwotę za pomocą przycisku „Odzyskaj teraz”.';

  @override
  String get walletReclaimConfirmCancel => 'Nie teraz';

  @override
  String get walletReclaimConfirmAction => 'Wznów';

  @override
  String get walletReclaimStarted =>
      'Wznawianie rozpoczęte. Gdy się potwierdzi, wyślij wstrzymaną wysyłkę, a następnie odzyskaj przeniesioną kwotę za pomocą przycisku „Odzyskaj teraz”.';

  @override
  String get walletReclaimNothing => 'Obecnie nie ma nic do wznowienia.';

  @override
  String get walletReclaimNotBroadcast =>
      'Nie udało się potwierdzić, że dotarło to do sieci. Może się to jednak udać — poczekaj chwilę, zanim spróbujesz ponownie.';

  @override
  String get walletReclaimNeedsFunds =>
      'Potrzebujesz nieco chronionych ZEC, aby wznowić wysyłanie.';

  @override
  String get walletReclaimFailed =>
      'Nie udało się teraz wznowić wysyłania. Twoje środki pozostają bez zmian. Spróbuj ponownie.';

  @override
  String get walletReclaimUnknown =>
      'Wznawianie zakończone. Sprawdź swoje wysyłki za pomocą adresów jednorazowych i odzyskaj ewentualną przeniesioną kwotę za pomocą przycisku „Odzyskaj teraz”.';

  @override
  String get walletParkedError =>
      'Nie udało się teraz wczytać oczekujących płatności.';

  @override
  String get walletParkedErrorRetry => 'Spróbuj ponownie';

  @override
  String get walletParkedErrorRetryInProgress => 'Próbowanie…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Anulować tę oczekującą płatność?';

  @override
  String get walletParkedCancelConfirmBody =>
      'To odrzuca zapisaną płatność. Nie została ona wysłana, więc nic nie opuszcza Twojego portfela — ale tej operacji nie można cofnąć.';

  @override
  String get walletParkedCancelConfirmKeep => 'Zachowaj';

  @override
  String get walletParkedCancelConfirmDiscard => 'Odrzuć płatność';

  @override
  String get walletParkedCancelDone => 'Oczekująca płatność została anulowana.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Ta płatność może być już w drodze — sprawdź swoją aktywność.';

  @override
  String get walletParkedCancelFailed =>
      'Nie udało się teraz anulować. Płatność pozostaje bez zmian. Spróbuj ponownie.';

  @override
  String get walletRecoverNow => 'Odzyskaj teraz';

  @override
  String get walletRecoverConfirmTitle => 'Odzyskać do salda chronionego?';

  @override
  String get walletRecoverConfirmBody =>
      'Ta operacja sprawdza Twoje adresy jednorazowe i przenosi wszystko, co znajdzie, do prywatnego salda chronionego. Można ją bezpiecznie uruchamiać ponownie w dowolnym momencie.';

  @override
  String get walletRecoverConfirmCancel => 'Nie teraz';

  @override
  String get walletRecoverConfirmAction => 'Odzyskaj';

  @override
  String get walletRecoverInProgress => 'Odzyskiwanie…';

  @override
  String walletRecoverDone(String amount) {
    return 'Odzyskiwanie $amount do salda chronionego.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'Odzyskiwanie $amount — niektóre środki wymagają jeszcze kolejnej próby.';
  }

  @override
  String get walletRecoverRetry =>
      'Niektóre środki wymagają kolejnej próby — uruchom odzyskiwanie ponownie.';

  @override
  String get walletRecoverTruncated =>
      'Nie wszystkie adresy jednorazowe zostały jeszcze sprawdzone — uruchom ponownie, aby sprawdzić resztę.';

  @override
  String get walletRecoverNothing => 'Obecnie nie ma nic do odzyskania.';

  @override
  String get walletRecoverFailed =>
      'Nie udało się teraz odzyskać środków. Pozostają one bez zmian. Spróbuj ponownie.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount zapisane i oczekujące · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Anuluj płatność na kwotę $amount, zapisaną $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount wstrzymane · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount przygotowywane do wysłania · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Twój portfel przygotowuje tę płatność — jej kwota mogła już zostać zarezerwowana. Twoje środki są bezpieczne. Jeśli się nie zakończy, sama wróci na listę.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Twój portfel przygotowuje tę płatność — jej kwota mogła już zostać zarezerwowana. Twoje środki są bezpieczne, ale będzie mogła się zakończyć dopiero gdy Twój portfel znów zacznie się synchronizować.';

  @override
  String get walletParkedSendNow => 'Wyślij teraz';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Wysyłanie płatności na kwotę $amount, zapisanej $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Wyślij teraz płatność na kwotę $amount, zapisaną $time';
  }

  @override
  String get walletParkedSendNowInProgress => 'Wysyłanie…';

  @override
  String get walletParkedAuthorizeSent => 'Twoja płatność jest teraz wysyłana.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Twoja płatność jest teraz wysyłana. Jeśli się nie powiedzie, Twój portfel będzie mógł ją dokończyć dopiero, gdy znów zacznie się synchronizować.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Płatność nie jest jeszcze gotowa do wysłania. Jest zapisana i pozostaje bez zmian.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Płatność nie jest jeszcze gotowa do wysłania. Jest zapisana i nie jest już wstrzymana — spróbuj później ponownie użyć przycisku „Wyślij teraz” lub ją anuluj.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Nie udało się teraz wysłać. Płatność pozostaje bez zmian. Spróbuj ponownie.';

  @override
  String get walletTransparentFundsMenuItem => 'Środki publiczne…';

  @override
  String get walletTransparentFundsTitle => 'Środki publiczne';

  @override
  String get walletTransparentFundsIntro =>
      'Środki publiczne są publicznie widoczne w blockchainie — kwota, adresy i historia środków.';

  @override
  String get walletExpertToggleLabel => 'Zaawansowane: środki publiczne';

  @override
  String get walletExpertToggleDescription =>
      'Pokaż zaawansowane opcje przechowywania środków publicznych i wyłączania automatycznego obejmowania ochroną.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Pokaż zaawansowane opcje przechowywania środków publicznych.';

  @override
  String get walletAutoShieldToggleLabel => 'Automatycznie obejmuj ochroną';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Gdy Twoje saldo publiczne osiągnie $minZec ZEC, jest ono automatycznie przenoszone do Twojego salda chronionego. Gdy ta opcja jest wyłączona, środki publiczne pozostają publicznie widoczne, dopóki sam ich nie obejmiesz ochroną.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Nie udało się zapisać ustawienia. Spróbuj ponownie.';

  @override
  String get walletAutoShieldIncomplete =>
      'Automatyczne obejmowanie ochroną nie zostało ukończone — te środki są nadal publicznie widoczne. Możesz teraz objąć je ochroną.';

  @override
  String get walletSendPrivacyShielded =>
      'Płatność chroniona — kwota i odbiorca pozostają prywatne w blockchainie.';

  @override
  String get walletSendPrivacyTransparent =>
      'Płatność jawna — kwota i adresy są widoczne w blockchainie.';

  @override
  String get walletActivityPublicBadge => 'Publicznie widoczne w blockchainie';

  @override
  String get walletShieldWalletEnded =>
      'Sesja portfela została zakończona. Zamknij i otwórz ponownie, aby spróbować jeszcze raz.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Nowe środki publiczne są automatycznie obejmowane ochroną i trafiają do Twojego prywatnego salda, gdy osiągną $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'Automatyczne obejmowanie ochroną jest wyłączone — środki publiczne pozostają publicznie widoczne, dopóki ich nie obejmiesz ochroną.';

  @override
  String get walletMoveAutoShieldNote =>
      'Automatyczne obejmowanie ochroną jest włączone: gdy te środki dotrą, zostaną ponownie automatycznie objęte ochroną (wiąże się to z dodatkową opłatą sieciową). Aby zachować je jako publiczne, najpierw wyłącz automatyczne obejmowanie ochroną w sekcji Środki publiczne.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Po tej operacji Twoje saldo publiczne wyniesie $amount ZEC — mniej niż $floor ZEC potrzebne, by ponownie objąć je ochroną. Pozostanie publiczne, dopóki nie dotrą kolejne środki.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Przenosisz środki na swój własny adres publiczny. Ta operacja na stałe pozostanie w publicznym rejestrze.';

  @override
  String get walletTxDetailVisibility => 'Widoczność';

  @override
  String get walletTransparentFundsAutoDenied =>
      'Automatyczne obejmowanie ochroną jest wstrzymane na czas tej sesji — nie zostało zatwierdzone. Nadal możesz objąć środki ochroną ręcznie.';

  @override
  String get walletDeepScanMenuItem => 'Sprawdź starsze adresy wymiany…';

  @override
  String get walletMenuSyncNotRunningHint => 'Synchronizacja teraz nie działa.';

  @override
  String get walletDeepScanTitle => 'Sprawdź starsze adresy wymiany';

  @override
  String get walletDeepScanBody =>
      'Jeśli przywróciłeś ten portfel i kiedyś często korzystał z wymiany, środki z jego najstarszych wymian mogą wymagać dodatkowego kroku, aby je znaleźć. To sprawdzenie ich szuka — wszystko, co zostanie znalezione, pojawi się w Twoim saldzie w miarę synchronizacji portfela.';

  @override
  String get walletDeepScanCoverage =>
      'Twoje starsze adresy wymiany zostały sprawdzone do tego miejsca. Jeśli środki z dawnej wymiany nadal nie zostały znalezione, sprawdź jeszcze głębiej.';

  @override
  String get walletDeepScanCoveragePending =>
      'Nadal trwa sprawdzanie bieżącego zakresu — wszystko, co zostanie znalezione, pojawi się w Twoim saldzie. Może to potrwać chwilę.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Sprawdza obecność środków z najstarszych wymian Twojego portfela.';

  @override
  String get walletDeepScanCheckButton => 'Sprawdź starsze adresy';

  @override
  String get walletDeepScanCheckDeeperButton =>
      'Sprawdź jeszcze starsze adresy';

  @override
  String get walletDeepScanChecking => 'Sprawdzanie…';

  @override
  String get walletDeepScanClose => 'Zamknij';

  @override
  String get walletDeepScanTorHint =>
      'Nie jesteś teraz połączony przez Tor. Aby zwiększyć prywatność, rozważ poczekanie, aż Tor będzie aktywny, zanim rozpoczniesz sprawdzanie.';

  @override
  String get walletDeepScanRescanBusy =>
      'Będziesz mógł sprawdzić starsze adresy wymiany, gdy ponowne skanowanie się zakończy.';

  @override
  String get walletDeepScanRan =>
      'Starsze adresy wymiany są sprawdzane — wszystko, co zostanie znalezione, pojawi się w Twoim saldzie.';

  @override
  String get walletDeepScanFailed =>
      'Nie udało się rozpocząć sprawdzania. Nic się nie zmieniło — spróbuj ponownie.';

  @override
  String get walletDeepScanSlow =>
      'To trwa dłużej niż zwykle. Jeśli Twoje starsze adresy wymiany zostały sprawdzone, wszystko, co zostanie znalezione, pojawi się w Twoim saldzie — sprawdź ponownie za chwilę.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'Wymiana jest teraz wyłączona, więc nie można tego uruchomić. Spróbuj ponownie, gdy wymiana będzie dostępna.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Nadal trwa sprawdzanie ostatniego zakresu — może to potrwać do dwóch dni, ale zwykle znacznie krócej. Kończy się samoczynnie — sprawdź ponownie później.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'Nie można jeszcze potwierdzić prywatności Twojego połączenia. Aby zwiększyć prywatność, sprawdź, gdy Tor będzie aktywny.';

  @override
  String get walletDeepScanBannerChecking =>
      'Nadal trwa sprawdzanie starszych adresów wymiany — wszystko, co zostanie znalezione, pojawi się w Twoim saldzie.';

  @override
  String get walletRescanSwapPointer =>
      'Szukasz środków z dawnej wymiany? Ponowne skanowanie ich nie znajdzie — użyj zamiast tego opcji „Sprawdź starsze adresy wymiany”.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Przywróciłeś portfel, który korzystał z wymiany?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Jeśli ten portfel miał bardzo długą historię wymian, środki z jego najstarszych wymian mogą wymagać dodatkowego kroku, aby je znaleźć. Większość portfeli niczego nie wymaga.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Sprawdź teraz';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Zamknij';

  @override
  String walletTorHostPath(String transport) {
    return 'Przez prywatną ścieżkę Twojej aplikacji ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Przez prywatną ścieżkę Twojej aplikacji ($transport); proxy może powiązać połączenia';
  }

  @override
  String get walletTorHostOtherTransport => 'prywatna ścieżka';

  @override
  String get walletTorHostDirect =>
      'Nieprywatne (bezpośrednie połączenie Twojej aplikacji)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Zapisany serwer używa nieszyfrowanego adresu, którego prywatna ścieżka Twojej aplikacji nie może przenieść. Używany jest $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Więcej o: $label';
  }

  @override
  String get walletSendPaste => 'Wklej';

  @override
  String get walletSendScanQr => 'Zeskanuj kod QR';

  @override
  String get walletSendRecipientGetsLabel => 'Odbiorca otrzyma';

  @override
  String get walletSwapDepositCopyAmount => 'Kopiuj kwotę';

  @override
  String get walletSwapDepositAmountCopied => 'Skopiowano kwotę';

  @override
  String get walletScanOpenSettings => 'Otwórz ustawienia';

  @override
  String get walletScanOpenSettingsFailed => 'Nie udało się otworzyć ustawień.';

  @override
  String get walletSendLeaveTitle => 'Wysyłanie trwa';

  @override
  String get walletSendLeaveBody =>
      'Płatność będzie kontynuowana, jeśli wyjdziesz. Jej wynik zobaczysz w aktywności.';

  @override
  String get walletSendLeaveStay => 'Zostań';

  @override
  String get walletSendLeaveConfirm => 'Wyjdź';

  @override
  String get walletSheetLeaveBody =>
      'To będzie kontynuowane, jeśli wyjdziesz. Wynik zobaczysz w aktywności.';

  @override
  String get walletLoadingLabel => 'Ładowanie';

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
      'Sprawdź, zanim ponownie obejmiesz ochroną';

  @override
  String get walletShieldUnknownBody =>
      'Nie udało się potwierdzić objęcia ochroną. Sprawdź sekcję Aktywność, zanim spróbujesz ponownie.';

  @override
  String get walletMoveUnknownTitle => 'Sprawdź, zanim ponownie przeniesiesz';

  @override
  String get walletMoveUnknownBody =>
      'Nie udało się potwierdzić tego przeniesienia. Sprawdź sekcję Aktywność, zanim spróbujesz ponownie.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
