// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Russian (`ru`).
class WalletLocalizationsRu extends WalletLocalizations {
  WalletLocalizationsRu([String locale = 'ru']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Настройки';

  @override
  String get walletTitle => 'Кошелёк';

  @override
  String get walletNotSetUpTitle => 'Кошелёк ещё не настроен';

  @override
  String get walletNotSetUpBody =>
      'Настройка кошелька появится в одной из следующих версий. Она проведёт вас через запись фразы восстановления до того, как станет возможно получать средства, — поэтому без резервной копии деньги никогда не окажутся под угрозой.';

  @override
  String get walletStartupFailedTitle => 'Не удалось запустить кошелёк';

  @override
  String get walletStartupFailedBody =>
      'Что-то помешало кошельку загрузиться на этом устройстве. Если у вас уже есть кошелёк, его средства не пострадали — они находятся в сети Zcash и могут быть восстановлены с помощью фразы восстановления. Повторите попытку; если это повторяется, закройте приложение и откройте его снова.';

  @override
  String get walletBalanceLabel => 'Баланс';

  @override
  String get walletHideBalance => 'Скрыть баланс';

  @override
  String get walletShowBalance => 'Показать баланс';

  @override
  String get walletBalanceHiddenAmount => 'Баланс скрыт';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Доступно сейчас';

  @override
  String get walletArrivingLabel => 'Поступает';

  @override
  String get walletNotSpendableYetLabel => 'Пока нельзя потратить';

  @override
  String get walletActivityTitle => 'Активность';

  @override
  String get walletActivityEmpty => 'Активности пока нет';

  @override
  String get walletActivityError => 'Не удалось загрузить активность';

  @override
  String get walletActivityReceived => 'Получено';

  @override
  String get walletActivitySent => 'Отправлено';

  @override
  String get walletActivityPending => 'В обработке';

  @override
  String get walletActivityQueued => 'В очереди';

  @override
  String get walletActivityRetrying => 'Повторная отправка';

  @override
  String get walletActivitySaved => 'Сохранено';

  @override
  String get walletActivityExpired => 'Истекло';

  @override
  String get walletActivityFailed => 'Отклонено';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count подтверждений',
      many: '$count подтверждений',
      few: '$count подтверждения',
      one: '$count подтверждение',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count платежей получено',
      many: '$count платежей получено',
      few: '$count платежа получено',
      one: 'Платёж получен',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Показать детали транзакции';

  @override
  String get walletTxDetailStatus => 'Статус';

  @override
  String get walletTxDetailFee => 'Комиссия сети';

  @override
  String get walletTxDetailDate => 'Дата';

  @override
  String get walletTxDetailHeight => 'Высота блока';

  @override
  String get walletTxDetailMemo => 'Заметка';

  @override
  String get walletTxDetailMemoAttached => 'Прикреплена';

  @override
  String get walletTxDetailTxid => 'ID транзакции';

  @override
  String get walletTxDetailCopyTxid => 'Скопировать ID транзакции';

  @override
  String get walletTxDetailCopied => 'ID транзакции скопирован';

  @override
  String get walletTxDetailClose => 'Закрыть';

  @override
  String get walletTxFundsKept => 'Средства не покидали ваш кошелёк';

  @override
  String get walletTxExplainQueued =>
      'Сохранено на этом устройстве, в разделе «Сохранено и ожидает» — там его можно отправить или отменить.';

  @override
  String get walletTxExplainPending =>
      'Отправлено в сеть Zcash — ожидает подтверждения в блоке.';

  @override
  String get walletTxExplainRetrying =>
      'Кошелёк пока не смог отправить это в сеть Zcash. Он хранит подписанную транзакцию и повторяет попытку при каждой синхронизации, пока она не пройдёт или не истечёт.';

  @override
  String get walletTxExplainSaved =>
      'Кошелёк сохранил эту подписанную транзакцию, но сейчас не отправляет её сам.';

  @override
  String get walletTxExplainConfirmed => 'Подтверждено в сети Zcash.';

  @override
  String get walletTxExplainExpired =>
      'Срок действия этой транзакции истёк до подтверждения сетью, поэтому она была отменена. Сумма по-прежнему доступна вам для расходования.';

  @override
  String get walletTxExplainFailed =>
      'Сеть отклонила эту транзакцию, поэтому она не прошла. Сумма по-прежнему доступна вам для расходования.';

  @override
  String get walletTxExplainUnknown =>
      'Текущий статус этой транзакции не удаётся определить. Он обновится после следующей синхронизации.';

  @override
  String get walletMenuTooltip => 'Ещё';

  @override
  String get walletRescanMenuItem => 'Пересканировать историю…';

  @override
  String get walletCheckOneTimeMenuItem => 'Проверить одноразовые адреса…';

  @override
  String get walletRescanTitle => 'Пересканирование истории';

  @override
  String get walletRescanBody =>
      'Не хватает старых средств? Просканируйте блокчейн заново с более ранней даты, чтобы найти поступления, пропущенные из-за более поздней начальной даты. Ваши средства и фраза восстановления никогда не подвергаются риску.';

  @override
  String get walletRescanRangeTitle => 'Насколько далеко сканировать';

  @override
  String get walletRescanRangeAll =>
      'Просканировать всю историю — самый медленный вариант, но находит всё.';

  @override
  String get walletRescanRangeDefault =>
      'Сканирование с момента создания вашего кошелька. Всё ещё не хватает старых средств? Выберите более раннюю дату или отсканируйте всю историю.';

  @override
  String get walletRescanRangeResolving =>
      'Подготовка рекомендуемого диапазона…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Осталось просканировать около $blocks блоков.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Сканирование с $date. Всё ещё не хватает старых средств? Выберите более раннюю дату или отсканируйте всю историю.';
  }

  @override
  String get walletRescanPick => 'Выбрать дату';

  @override
  String get walletRescanChange => 'Изменить дату';

  @override
  String get walletRescanScanAll => 'Отсканировать всю историю';

  @override
  String get walletRescanDatePick => 'Самая ранняя дата для сканирования';

  @override
  String get walletRescanWarning =>
      'Это заново просканирует блокчейн. Недавние даты занимают минуты; сканирование далёкого прошлого может занять часы. Синхронизация выполняется в фоне — вы можете продолжать пользоваться кошельком.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Платёж из этого кошелька всё ещё подтверждается. Кошелёк обычно отказывает в пересканировании, пока он не завершится — вы можете попробовать, но, скорее всего, получите отказ.';

  @override
  String get walletRescanConfirm => 'Начать пересканирование';

  @override
  String get walletRescanCancel => 'Отмена';

  @override
  String get walletRescanRunning => 'Пересборка…';

  @override
  String get walletRescanRebuildingAll =>
      'Пересборка истории — сканируется вся цепочка блоков. Баланс и активность будут заполняться по мере продвижения.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Пересборка истории с $date — баланс и активность будут заполняться по мере продвижения.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Пересборка истории с момента создания вашего кошелька — баланс и активность будут заполняться по мере продвижения.';

  @override
  String get walletCatchUpBanner =>
      'Наверстывание — баланс и активность заполняются по мере синхронизации кошелька. Всё, что вы получили, в безопасности.';

  @override
  String get walletCatchUpRescanBanner =>
      'Пересборка истории после пересканирования — баланс и активность будут заполняться по мере продвижения. Всё, что вы получили, в безопасности.';

  @override
  String get walletRescanFailedNotice =>
      'Не удалось выполнить пересканирование — ваши средства в безопасности, хотя балансу и истории может понадобиться немного времени, чтобы наверстать упущенное. Повторите попытку через некоторое время.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Платёж всё ещё подтверждается, поэтому пересканирование приостановлено для защиты ваших средств. Кошелёк не изменился — повторите попытку через пару часов и держите приложение открытым и в сети.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'Пересканирование пересобирает вашу историю по мере синхронизации кошелька, а синхронизация сейчас не выполняется. Кошелёк не изменился — повторите попытку, как только синхронизация будет выполняться.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Недостаточно свободного места для пересборки истории кошелька — ваши средства в безопасности, хотя балансу и истории может понадобиться немного времени, чтобы наверстать упущенное. Освободите немного места и повторите попытку.';

  @override
  String get walletRescanFailedDismiss => 'Закрыть';

  @override
  String get walletActivityRebuilding => 'Пересборка истории…';

  @override
  String get walletActivityCatchingUp =>
      'Наверстывание продолжается — всё, что вы получили, появится здесь.';

  @override
  String get walletActivitySyncNotRunning =>
      'Ваши баланс и история полностью загрузятся, как только синхронизация будет выполняться.';

  @override
  String get walletActivityLoadMore => 'Загрузить ещё';

  @override
  String get walletPendingChangeLabel => 'Ожидается сдача';

  @override
  String get walletTransparentLabel => 'Незащищённый (публичный)';

  @override
  String get walletTransparentNote =>
      'Не входят в «Доступно сейчас» — защитите эти средства, чтобы их потратить. До тех пор они по-прежнему публично видны в блокчейне.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Эти средства публично видны в блокчейне.';

  @override
  String walletPoolShielded(String amount) {
    return 'Защищено $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Публично $amount';
  }

  @override
  String get walletPoolAllShielded => 'Всё защищено · приватно';

  @override
  String get walletPoolTapHint => 'Показать публичные средства';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount вашего баланса находится на одноразовом адресе (можно восстановить).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount вашего баланса находится на одноразовом адресе.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Платежи на общую сумму $amount зарезервированы и всё ещё завершаются через одноразовые адреса, которые контролирует ваш кошелёк. Не отправляйте их повторно.',
      many:
          'Платежи на общую сумму $amount зарезервированы и всё ещё завершаются через одноразовые адреса, которые контролирует ваш кошелёк. Не отправляйте их повторно.',
      few:
          'Платежи на общую сумму $amount зарезервированы и всё ещё завершаются через одноразовые адреса, которые контролирует ваш кошелёк. Не отправляйте их повторно.',
      one:
          '$amount зарезервировано за платежом, который ваш кошелёк ещё завершает через контролируемый им одноразовый адрес. Не отправляйте его повторно.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Платежи на общую сумму $amount зарезервированы и завершаются на полпути через одноразовые адреса, которые контролирует ваш кошелёк. Приостановлено, пока ваш кошелёк снова не синхронизируется. Не отправляйте их повторно.',
      many:
          'Платежи на общую сумму $amount зарезервированы и завершаются на полпути через одноразовые адреса, которые контролирует ваш кошелёк. Приостановлено, пока ваш кошелёк снова не синхронизируется. Не отправляйте их повторно.',
      few:
          'Платежи на общую сумму $amount зарезервированы и завершаются на полпути через одноразовые адреса, которые контролирует ваш кошелёк. Приостановлено, пока ваш кошелёк снова не синхронизируется. Не отправляйте их повторно.',
      one:
          '$amount зарезервировано за платежом, который ваш кошелёк завершает на полпути через контролируемый им одноразовый адрес. Приостановлено, пока ваш кошелёк снова не синхронизируется. Не отправляйте его повторно.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Не удалось проверить, завершается ли ещё платёж. Повторяем попытку — а пока проверьте, нет ли ожидающего платежа в вашей активности, прежде чем отправлять снова.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount вашего баланса находится на одноразовом адресе (ещё подтверждается).';
  }

  @override
  String get walletShieldButton => 'Защитить';

  @override
  String get walletShieldSheetTitle => 'Защита публичных средств';

  @override
  String get walletShieldNote =>
      'Это переместит средства с вашего публичного, видимого в блокчейне баланса в приватный защищённый баланс.';

  @override
  String get walletShieldPreparing => 'Подготовка…';

  @override
  String get walletShieldAmountLabel => 'К защите';

  @override
  String get walletShieldFeeLabel => 'Комиссия сети';

  @override
  String get walletShieldNetLabel => 'Будет защищено';

  @override
  String get walletShieldConfirmButton => 'Защитить сейчас';

  @override
  String get walletShieldSubmitting => 'Защита…';

  @override
  String get walletShieldNothingTitle => 'Пока нечего защищать';

  @override
  String get walletShieldNothingBody =>
      'Эта сумма пока меньше той, которую имеет смысл защищать, — комиссия сети превысит выгоду. Защита станет возможна, когда поступит немного больше средств.';

  @override
  String get walletShieldDoneTitle => 'Защита отправлена';

  @override
  String get walletShieldDoneBody =>
      'Ваши средства перемещаются в защищённый баланс. Скоро это подтвердится в блокчейне.';

  @override
  String get walletShieldSavedTitle => 'Сохранено — мы завершим защиту';

  @override
  String get walletShieldSavedBody =>
      'Сейчас не удалось связаться с сетью. Ваша защита сохранена, и кошелёк завершит её при одной из более поздних синхронизаций. Ничего не потеряно.';

  @override
  String get walletShieldAlreadyTitle => 'Уже отправлено';

  @override
  String get walletShieldFailedTitle => 'Не удалось защитить средства';

  @override
  String get walletShieldStaleBody =>
      'Кошелёк ещё синхронизируется. Повторите попытку защиты через некоторое время.';

  @override
  String get walletShieldTransientBody =>
      'Не удалось подготовить экранирование прямо сейчас. Повторите попытку через минуту.';

  @override
  String get walletShieldStorageFullBody =>
      'Недостаточно свободного места для защиты сейчас. Освободите немного места и повторите попытку. Ваши средства в безопасности.';

  @override
  String get walletShieldClose => 'Закрыть';

  @override
  String get walletShieldRetry => 'Повторить';

  @override
  String get walletMoveMenuItem => 'Перевести в публичный…';

  @override
  String get walletMoveSheetTitle => 'Перевод в публичный адрес';

  @override
  String get walletMoveSheetSubtitle =>
      'Отправьте защищённые ZEC на свой собственный публичный адрес — это пригодится, если биржа не принимает защищённые депозиты.';

  @override
  String get walletMoveDestinationLabel => 'Ваш публичный адрес';

  @override
  String walletMoveAvailable(String amount) {
    return 'Доступно для перевода: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Доступно для перевода: $amount ZEC — баланс всё ещё наверстывает упущенное';
  }

  @override
  String get walletMoveDeshieldTitle =>
      'Этот перевод сделает ваши средства публичными';

  @override
  String get walletMoveDeshieldBody =>
      'Перевод на публичный адрес выводит эти средства из защищённого баланса — сумма и ваш публичный адрес становятся публично видимыми в блокчейне Zcash.';

  @override
  String get walletMoveWalletEnded =>
      'Сеанс работы с кошельком завершён. Закройте и снова откройте кошелёк, чтобы повторить попытку.';

  @override
  String get walletMoveLoading => 'Подготовка…';

  @override
  String get walletMovePreparing => 'Проверка суммы…';

  @override
  String get walletMoveSubmitting => 'Перевод…';

  @override
  String get walletMoveReviewButton => 'Проверить';

  @override
  String get walletMoveCancel => 'Отмена';

  @override
  String get walletMoveReviewTitle => 'Проверка перевода';

  @override
  String get walletMoveOwnAddressNote =>
      'Вы переводите средства на собственный публичный адрес. Позже вы сможете снова защитить эти средства, но запись об этом переводе навсегда останется в публичном реестре.';

  @override
  String get walletMoveConfirmButton => 'Перевести в публичный адрес';

  @override
  String get walletMoveBackButton => 'Назад';

  @override
  String get walletMoveDoneTitle => 'Переведено в публичный адрес';

  @override
  String get walletMoveDoneBody =>
      'Ваши средства перемещаются на публичный адрес. Скоро это подтвердится в блокчейне.';

  @override
  String get walletMoveSavedTitle => 'Сохранено — мы завершим перевод';

  @override
  String get walletMoveSavedBody =>
      'Этот перевод сохранён, и кошелёк отправит его при одной из более поздних синхронизаций. Ничего не потеряно.';

  @override
  String get walletMoveAlreadyTitle => 'Уже отправлено';

  @override
  String get walletMoveAlreadyBody =>
      'Эти средства уже были отправлены и находятся на пути к вашему публичному адресу.';

  @override
  String get walletMoveFailedTitle => 'Не удалось выполнить перевод';

  @override
  String get walletMoveNothingTitle => 'Пока нечего переводить';

  @override
  String get walletMoveNothingBody =>
      'У вас пока нет доступного защищённого баланса для перевода. Как только средства подтвердятся, вы сможете перевести их на публичный адрес.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Кошелёк всё ещё наверстывает упущенное — всё, что вы получили, станет доступно для перевода после завершения синхронизации.';

  @override
  String get walletMoveCouldNotLoad =>
      'Не удалось загрузить ваш публичный адрес. Повторите попытку.';

  @override
  String get walletMoveRetry => 'Повторить';

  @override
  String get walletMoveClose => 'Закрыть';

  @override
  String get walletSnapshotUnavailable =>
      'Не удалось прочитать данные кошелька прямо сейчас. Они обновятся автоматически.';

  @override
  String get walletBalanceStale =>
      'Не удалось обновить — показан последний известный баланс.';

  @override
  String get walletSyncStartFailed =>
      'Не удалось начать синхронизацию. Мы продолжим попытки.';

  @override
  String get walletSyncRetry => 'Повторить';

  @override
  String get walletSyncTryNow => 'Попробовать сейчас';

  @override
  String get walletSyncIdle => 'Синхронизация ещё не началась';

  @override
  String get walletSyncIdleDetail => 'Синхронизация начинается автоматически.';

  @override
  String get walletSyncDisabled => 'Синхронизация отключена';

  @override
  String get walletSyncDisabledDetail =>
      'Включите синхронизацию в настройках этого приложения, чтобы обновить баланс.';

  @override
  String get walletSyncExplainDisabled =>
      'Синхронизация отключена в настройках этого приложения. Ваши средства в безопасности. Баланс и активность показывают последнее синхронизированное состояние и не будут обновляться, пока синхронизация не будет включена.';

  @override
  String get walletParkedSyncPausedNote =>
      'Ваш кошелёк не синхронизируется, поэтому эти платежи не отправятся сами по себе. Используйте «Отправить сейчас», чтобы отправить платёж самостоятельно.';

  @override
  String get walletSyncPausedMoneyNote =>
      'Приостановлено, пока ваш кошелёк снова не синхронизируется.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Подключение…';

  @override
  String get walletSyncStartingDetail =>
      'Подключение к сети Zcash и подготовка к сканированию.';

  @override
  String get walletSyncConnecting => 'Подключение…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Подключение… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Сканирование $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'Сканирование…';

  @override
  String get walletSyncSpendableReady => 'Средства готовы к расходованию.';

  @override
  String get walletSyncCatchingUp =>
      'Синхронизация с сетью — глубокая начальная синхронизация может занять время. Вы можете продолжать пользоваться приложением, пока она завершается';

  @override
  String walletSyncScanRemaining(String count) {
    return 'Осталось блоков: $count';
  }

  @override
  String get walletSyncUpToDate => 'Синхронизировано';

  @override
  String get walletSyncOffline => 'Не в сети';

  @override
  String get walletSyncOfflineDetail =>
      'Отправки в очереди остаются сохранёнными в разделе «Сохранено и ожидает».';

  @override
  String get walletSyncUnknown => 'Синхронизация…';

  @override
  String get walletSyncStalled => 'Синхронизация приостановлена';

  @override
  String get walletStallEndpoint =>
      'Не удаётся связаться с сетью Zcash. Мы продолжим попытки автоматически — проверьте подключение к интернету, либо сервер может быть временно недоступен.';

  @override
  String get walletStallTor =>
      'Приватный путь вашего приложения недоступен, поэтому кошелёк не подключается. Проверьте сетевые настройки приложения или отключите приватный путь. Синхронизация возобновится, как только путь вернётся.';

  @override
  String get walletStallStorage =>
      'Хранилище устройства заполнено. Освободите место, и синхронизация возобновится.';

  @override
  String get walletStallReorg =>
      'Произошла реорганизация цепочки; повторная проверка последних блоков.';

  @override
  String get walletStallInternal =>
      'Локальная проблема остановила синхронизацию. Если это повторяется, восстановите кошелёк из фразы восстановления.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Этот сервер прислал данные, которые не могут быть верными, поэтому синхронизация остановлена. Это не проблема соединения — переключитесь на другой сервер. Если отклоняется каждый сервер, пересканируйте историю: кошелёк может хранить ошибочную запись от предыдущего сервера.';

  @override
  String get walletStallBirthdayInFuture =>
      'Этот кошелёк настроен начинать с блока, которого этот сервер ещё не достиг. Проверьте начальный блок, заданный для этого кошелька, или попробуйте другой сервер.';

  @override
  String get walletStallStorageUnavailable =>
      'Синхронизация на этом устройстве приостановлена. Повторная попытка.';

  @override
  String get walletStallUnknown =>
      'Синхронизация остановлена по неизвестной причине.';

  @override
  String get walletSyncBadgeHint => 'Показать сведения о синхронизации';

  @override
  String get walletSyncSheetClose => 'Закрыть';

  @override
  String get walletSyncSheetProgress => 'Прогресс';

  @override
  String get walletSyncSheetBlocksLeft => 'Осталось блоков';

  @override
  String get walletSyncSheetSyncedTo => 'Синхронизировано до блока';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Отстаёт не менее чем на $blocks блока',
      many: 'Отстаёт не менее чем на $blocks блоков',
      few: 'Отстаёт не менее чем на $blocks блока',
      one: 'Отстаёт не менее чем на $blocks блок',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'Синхронизация ещё не началась — она запускается автоматически. Никаких действий не требуется.';

  @override
  String get walletSyncExplainStartFailed =>
      'Синхронизация не смогла начаться. Ваши средства в безопасности — кошелёк просто не проверяет новую активность. Повторите попытку ниже или откройте приложение заново.';

  @override
  String get walletSyncExplainStarting =>
      'Кошелёк подключается к сети Zcash и готовится к сканированию. Обычно это занимает несколько секунд.';

  @override
  String get walletSyncExplainConnecting =>
      'Установка соединения с сетью Zcash.';

  @override
  String get walletSyncExplainScanning =>
      'Кошелёк проверяет блоки блокчейна на наличие ваших средств. Баланс и активность обновляются по мере обнаружения новых транзакций — вы можете продолжать пользоваться приложением, пока это происходит.';

  @override
  String get walletSyncExplainUpToDate =>
      'Полностью синхронизировано с сетью Zcash. Баланс и активность актуальны.';

  @override
  String get walletSyncExplainStalled =>
      'Синхронизация столкнулась с проблемой и приостановлена. Повтор произойдёт автоматически.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Не удаётся связаться с сетью Zcash — это нормально, если вы не в сети, либо сервер может быть временно недоступен. Ваши средства в безопасности: баланс показывает последнее синхронизированное состояние, а отправки в очереди остаются сохранёнными в разделе «Сохранено и ожидает». Подключение повторяется автоматически.';

  @override
  String get walletSyncExplainOffline =>
      'Нет подключения к сети. Ваши средства в безопасности — баланс показывает последнее синхронизированное состояние, а отправки в очереди остаются сохранёнными в разделе «Сохранено и ожидает».';

  @override
  String get walletSyncExplainUnknown =>
      'Кошелёк синхронизируется. Баланс и активность обновляются по мере продвижения.';

  @override
  String get walletTorOff => 'Tor выключен';

  @override
  String get walletTorBootstrapping => 'Приватный путь запускается…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport запускается…';
  }

  @override
  String get walletTorActive => 'Tor активен';

  @override
  String get walletTorActiveUnverified =>
      'Tor активен (среда выполнения не проверена)';

  @override
  String get walletTorActiveUnattested =>
      'Используется приватный путь (приватность не подтверждена)';

  @override
  String get walletTorFellBack =>
      'Tor недоступен — используется прямое подключение';

  @override
  String get walletTorUnavailable =>
      'Приватный путь недоступен — нет подключения';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport недоступен — нет подключения';
  }

  @override
  String get walletTorUnanswered => 'Приватный путь подключён — ответа нет';

  @override
  String get walletTorUnansweredUnattested =>
      'Приватный путь подключён — ответа нет (приватность не подтверждена)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport подключён — ответа нет';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Не приватно (прямое подключение вашего приложения) — ответа нет';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Подключено через $transport — ответа нет; прокси может связать соединения между собой';
  }

  @override
  String get walletTorUnknown =>
      'Статус Tor неизвестен — считайте соединение незащищённым';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Баланс (по состоянию на блок $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Баланс · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Баланс (по состоянию на блок $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Подключение';

  @override
  String get walletSyncSheetServer => 'Сервер';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Сервер, $host, открывает выбор сервера';
  }

  @override
  String get walletSyncServerSheetTitle => 'Сервер синхронизации';

  @override
  String get walletSyncServerInUse => 'Используется';

  @override
  String get walletSyncServerAppDefault => 'По умолчанию в приложении';

  @override
  String get walletSyncServerCustom => 'Свой сервер…';

  @override
  String get walletSyncServerCustomHint => 'https://хост:порт';

  @override
  String get walletSyncServerCheck => 'Проверить сервер';

  @override
  String get walletSyncServerChecking => 'Проверка…';

  @override
  String get walletSyncServerUse => 'Использовать этот сервер';

  @override
  String get walletSyncServerSwitching => 'Переключение…';

  @override
  String get walletSyncServerContinue => 'Продолжить';

  @override
  String get walletSyncServerCancel => 'Отмена';

  @override
  String get walletSyncServerTrustTitle => 'Доверять этому серверу?';

  @override
  String get walletSyncServerTrustNotice =>
      'Вы доверяете этому серверу сообщать ваш баланс и историю и передавать ваши платежи. Он увидит ваш IP-адрес, если Tor не включён, примерно когда был создан кошелёк, публичные адреса, которые проверяет кошелёк, транзакции, которые он запрашивает, и отправляемые вами транзакции.';

  @override
  String get walletSyncServerKeyLabel => 'Ключ доступа (необязательно)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Заголовок ключа';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Введите заголовок, который ожидает ваш сервер';

  @override
  String get walletSyncServerKeyInvalid =>
      'Этот ключ или заголовок нельзя использовать';

  @override
  String get walletSyncServerKeySaved => 'Ключ сохранён';

  @override
  String get walletSyncServerKeyShow => 'Показать';

  @override
  String get walletSyncServerKeyHide => 'Скрыть';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Ваш ключ идентифицирует вас для этого сервера. Он может связать ваши платежи с вашим кошельком, даже через Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Переключение перезапустит текущую синхронизацию. Баланс и история сохранятся. Средства могут отображаться как поступающие, пока сканирование нового сервера не догонит.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Переключение заново подключится к новому серверу. Баланс и история сохранятся.';

  @override
  String get walletSyncServerUnreachable =>
      'Не удалось связаться с этим сервером. Проверьте адрес — и если он верный, то либо этот сервер не отвечает, либо ваше приложение сейчас не может до него добраться. Попробуйте снова или выберите другой сервер.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Не удалось связаться с этим сервером. Кошелёк не может определить, этот сервер не отвечает или ваше приложение сейчас не может до него добраться. Выберите другой сервер или попробуйте позже.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Этот сервер находится в другой сети Zcash.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Это не похоже на адрес сервера. Используйте https://хост:порт.';

  @override
  String get walletSyncServerNotOffered =>
      'Это приложение не предлагает этот сервер.';

  @override
  String get walletSyncServerBusy =>
      'Кошелёк сейчас занят. Попробуйте чуть позже.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'Выбранный сервер больше не предлагается этим приложением. Используется $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'Не удалось прочитать сохранённый выбор сервера. Используется $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Не удалось переключиться — по-прежнему используется $host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Трафик кошелька подключается напрямую к серверу. Сервер может видеть ваш IP-адрес.';

  @override
  String get walletTransportExplainTor =>
      'Трафик кошелька маршрутизируется через сеть Tor, которая скрывает ваш IP-адрес от сервера.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Приватный путь вашего приложения запускается. Трафик кошелька ожидает его перед подключением.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport запускается. Трафик кошелька ожидает его перед подключением.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Не удалось подключиться к Tor, поэтому трафик перешёл на прямое подключение. Сервер может видеть ваш IP-адрес.';

  @override
  String get walletTransportExplainUnavailable =>
      'Приватный путь вашего приложения недоступен, поэтому кошелёк не подключается. Отключите приватный путь или проверьте сетевые настройки приложения.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport недоступен, поэтому кошелёк не подключается. Отключите его или проверьте сетевые настройки приложения.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'Приватный путь принял соединение, но уже минуту по нему ничего не приходит. Дело может быть в пути или в сервере кошелька — кошелёк не может определить, в чём именно. Он продолжает попытки; если это не пройдёт, попробуйте другой сервер или проверьте сетевые настройки приложения.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport принял соединение, но уже минуту по нему ничего не приходит. Дело может быть в пути или в сервере кошелька — кошелёк не может определить, в чём именно. Он продолжает попытки; если это не пройдёт, попробуйте другой сервер или проверьте сетевые настройки приложения.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Трафик кошелька подключается напрямую к серверу. Сервер может видеть ваш IP-адрес. Соединение принято, но уже минуту по нему ничего не приходит. Дело может быть в пути или в сервере кошелька — кошелёк не может определить, в чём именно. Он продолжает попытки; если это не пройдёт, попробуйте другой сервер или проверьте сетевые настройки приложения.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'Конфиденциальность этого соединения проверить не удаётся — считайте его неприватным. Соединение принято, но уже минуту по нему ничего не приходит. Дело может быть в пути или в сервере кошелька — кошелёк не может определить, в чём именно. Он продолжает попытки; если это не пройдёт, попробуйте другой сервер или проверьте сетевые настройки приложения.';

  @override
  String get walletTransportExplainUnverified =>
      'Конфиденциальность этого соединения проверить не удаётся — считайте его неприватным.';

  @override
  String get walletTransportExplainHostProxy =>
      'Трафик кошелька маршрутизируется через приватный транспорт этого приложения, который скрывает ваш IP-адрес от сервера.';

  @override
  String get walletOnboardingWelcomeTitle => 'Настройка кошелька';

  @override
  String get walletOnboardingWelcomeBody =>
      'Создайте новый кошелёк для получения и хранения ZEC. Мы сгенерируем фразу восстановления и проведём вас через её резервное сохранение до того, как смогут поступить какие-либо средства, — поэтому без резервной копии деньги никогда не окажутся под угрозой.';

  @override
  String get walletCreateButton => 'Создать новый кошелёк';

  @override
  String get walletRestoreButton => 'Восстановить из фразы восстановления';

  @override
  String get walletWatchOnlyButton =>
      'Наблюдать за кошельком (только просмотр)';

  @override
  String get walletWatchOnlyTitle => 'Наблюдение за кошельком';

  @override
  String get walletWatchOnlyBody =>
      'Вставьте ключ просмотра, чтобы наблюдать за кошельком без его ключей расходования. Вы увидите баланс и историю, но не сможете отправлять средства. Выберите примерную дату начала кошелька, чтобы мы знали, как далеко назад нужно искать.';

  @override
  String get walletWatchOnlyKeyLabel => 'Ключ просмотра';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip =>
      'Отсканировать QR-код ключа просмотра';

  @override
  String get walletWatchOnlyScanTitle => 'Сканирование ключа просмотра';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Наведите камеру на QR-код ключа просмотра.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Камера недоступна. Вставьте ключ вручную вместо этого.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Вставить вместо этого';

  @override
  String get walletWatchOnlyScanHint =>
      'Или нажмите кнопку сканирования, чтобы прочитать QR-код ключа просмотра.';

  @override
  String get walletWatchOnlyScanFilled => 'Ключ просмотра отсканирован.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Дата начала кошелька';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Сканирование с $date — средства, полученные до этой даты, не появятся. Кошелёк старше? Выберите более раннюю дату.';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'Выберите дату начала кошелька';

  @override
  String get walletWatchOnlyBirthdayChange => 'Изменить дату';

  @override
  String get walletWatchOnlySubmit => 'Наблюдать за этим кошельком';

  @override
  String get walletWatchOnlyBack => 'Назад';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Это не похоже на действительный ключ просмотра. Проверьте его и попробуйте снова.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Этот ключ просмотра предназначен для другой сети. Его нельзя использовать здесь.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'На этом устройстве уже есть кошелёк. Вернитесь назад и откройте его.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Эта дата начала слишком поздняя. Выберите более раннюю дату.';

  @override
  String get walletRestoreTitle => 'Восстановление кошелька';

  @override
  String get walletRestoreBody =>
      'Введите фразу восстановления, чтобы восстановить кошелёк, — впишите или вставьте слова по порядку через пробел. Поддерживаются только стандартные фразы: если ваш кошелёк использовал дополнительную парольную фразу («25-е слово»), это приложение пока не может её восстановить — вы увидите пустой кошелёк, а не ошибку.';

  @override
  String get walletRestorePhraseHint => 'слово один  слово два  слово три  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count слов',
      many: '$count слов',
      few: '$count слова',
      one: '$count слово',
      zero: 'Слов пока нет',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'фразы восстановления состоят из 12, 15, 18, 21 или 24 слов';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count слов не входят в список слов восстановления — исправьте выделенные',
      many:
          '$count слов не входят в список слов восстановления — исправьте выделенные',
      few:
          '$count слова не входят в список слов восстановления — исправьте выделенные',
      one:
          '$count слово не входит в список слов восстановления — исправьте выделенное',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'слово $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'слово $index: не входит в список слов восстановления';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Удалить слово $index';
  }

  @override
  String get walletRestoreSubmit => 'Восстановить кошелёк';

  @override
  String get walletRestoreBack => 'Назад';

  @override
  String get walletRestoreBirthdayTitle => 'Насколько далеко сканировать';

  @override
  String get walletRestoreBirthdayNone =>
      'Мы просканируем всю историю — медленнее, но ничего не будет пропущено.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Сканирование с $date — средства, полученные до этой даты, не появятся. Кошелёк старше? Выберите более раннюю дату или отсканируйте всю историю.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Выбрать дату';

  @override
  String get walletRestoreBirthdayChange => 'Изменить дату';

  @override
  String get walletRestoreBirthdayClear => 'Отсканировать всю историю';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'Слово $index не входит в список слов восстановления. Проверьте фразу на опечатки и повторите попытку.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Эта фраза восстановления недействительна. Проверьте слова и их порядок, затем повторите попытку.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Эта фраза не соответствует кошельку на этом устройстве. Перепроверьте её и повторите попытку.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'На этом устройстве уже есть кошелёк. Вернитесь назад, чтобы открыть его.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Эта дата слишком поздняя. Выберите более раннюю дату или отсканируйте всё.';

  @override
  String get walletGeneratingLabel => 'Создание кошелька…';

  @override
  String get walletOpeningLabel => 'Открытие кошелька…';

  @override
  String get walletBackupTitle => 'Сохранение фразы восстановления';

  @override
  String get walletBackupBody =>
      'Эти слова — ЕДИНСТВЕННЫЙ способ восстановить ваш кошелёк и средства. Запишите их по порядку и храните в надёжном месте, недоступном для посторонних. Никогда не сообщайте их никому и не храните в интернете — тот, кто узнает эти слова, сможет забрать ваши средства.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Снимки экрана на этом экране отключены.';

  @override
  String get walletBackupSecureNoteOther =>
      'Убедитесь, что никто не видит ваш экран.';

  @override
  String get walletBackupReveal => 'Показать фразу восстановления';

  @override
  String get walletBackupRevealing => 'Подготовка фразы восстановления…';

  @override
  String get walletBackupRevealFailed =>
      'Не удалось показать фразу восстановления. Убедитесь, что устройство разблокировано, и повторите попытку.';

  @override
  String get walletBackupRetryReveal => 'Повторить';

  @override
  String get walletBackupReauthFailed =>
      'Не удалось подтвердить, что это вы. Повторите попытку.';

  @override
  String get walletBackupConfirmCheckbox =>
      'Фраза восстановления записана и надёжно сохранена.';

  @override
  String get walletBackupContinue => 'Продолжить';

  @override
  String get walletBackupSaveFailed =>
      'Не удалось сохранить подтверждение. Повторите попытку.';

  @override
  String get walletBackupStartOver => 'Начать заново';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Начать заново без этого кошелька?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Это удалит этот кошелёк с устройства и вернёт вас к началу. Пока настройка не завершена, внести средства через это приложение невозможно.\n\nЕсли в этом кошельке когда-либо были средства — или он был восстановлен из фразы восстановления — восстановить его сможет только эта фраза.';

  @override
  String get walletBackupStartOverConfirm => 'Удалить и начать заново';

  @override
  String get walletBackupStartOverKeep => 'Сохранить этот кошелёк';

  @override
  String get walletBackupSectionTitle => 'Фраза восстановления';

  @override
  String get walletBackupTileTitle => 'Сохранить фразу восстановления';

  @override
  String get walletBackupTileSubtitle =>
      'Показать слова, с помощью которых можно восстановить кошелёк и средства.';

  @override
  String get walletBackupScreenTitle => 'Фраза восстановления';

  @override
  String get walletBackupDone => 'Готово';

  @override
  String get walletBackupManagedTitle => 'Нет отдельной фразы восстановления';

  @override
  String get walletBackupManagedBody =>
      'Этот кошелёк был настроен на основе учётной записи из приложения, которое его установило, поэтому у него нет собственной фразы восстановления. Ваши средства восстанавливаются вместе с этой учётной записью — используйте её резервную копию, чтобы сохранить их в безопасности.';

  @override
  String get walletExportViewingKeyTitle => 'Экспорт ключа просмотра';

  @override
  String get walletExportViewingKeyTileTitle => 'Экспорт ключа просмотра';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Поделитесь копией кошелька только для просмотра — она может видеть вашу историю, но не может тратить средства.';

  @override
  String get walletExportViewingKeyWarning =>
      'Этот ключ позволяет любому, кто им обладает, видеть всё, что этот кошелёк когда-либо получал и отправлял, — а также всё, что он получит и отправит в будущем. Он не может тратить ваши средства и не может восстановить ваш кошелёк. Делитесь им только с тем, кому вы доверяете видеть всю вашу историю, например с бухгалтером или вашим собственным вторым устройством. Единственный способ впоследствии закрыть доступ — перевести ваши средства в новый кошелёк.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Этот ключ позволяет любому, кто им обладает, видеть всё, что этот кошелёк когда-либо получал и отправлял, — а также всё, что он получит и отправит в будущем. Он не может тратить ваши средства и не может восстановить ваш кошелёк. Делитесь им только с тем, кому вы доверяете видеть всю вашу историю, например с бухгалтером или вашим собственным вторым устройством. Однажды поделившись им, закрыть к нему доступ уже нельзя.';

  @override
  String get walletExportViewingKeyReveal => 'Показать ключ просмотра';

  @override
  String get walletExportViewingKeyRetry => 'Повторить';

  @override
  String get walletExportViewingKeyRevealing =>
      'Подготовка вашего ключа просмотра…';

  @override
  String get walletExportViewingKeyFailed =>
      'Не удалось показать ваш ключ просмотра прямо сейчас. Повторите попытку через некоторое время.';

  @override
  String get walletExportViewingKeyQrLabel => 'QR-код ключа просмотра';

  @override
  String get walletExportViewingKeyCopy => 'Копировать ключ просмотра';

  @override
  String get walletExportViewingKeyCopied => 'Ключ просмотра скопирован';

  @override
  String get walletExportViewingKeyDone => 'Готово';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Снимки экрана на этом экране отключены.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Убедитесь, что никто не видит ваш экран.';

  @override
  String get walletWatchOnlySectionTitle =>
      'Об этом кошельке только для просмотра';

  @override
  String get walletWatchOnlyAboutBody =>
      'Это кошелёк только для просмотра. Он был настроен на основе ключа просмотра, поэтому видит ваш баланс и историю, но не содержит ключей расходования — здесь нечего резервировать, и он не может отправлять средства.';

  @override
  String get walletWatchOnlyBadge => 'Только просмотр';

  @override
  String get walletOnboardingFailedTitle =>
      'Не удалось завершить настройку кошелька';

  @override
  String get walletOnboardingRetry => 'Повторить';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'Защищённое хранилище телефона не отвечает. Разблокируйте устройство и повторите попытку. Если это повторяется, перезагрузите телефон.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Этот кошелёк открыт в другом окне или приложении либо ещё завершает предыдущую операцию. Закройте другое окно, использующее его, — или подождите немного, — затем повторите попытку.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'Защищённый ключ этого кошелька больше недоступен, поэтому открыть его на этом устройстве нельзя. Ваши средства в безопасности — восстановите кошелёк из фразы восстановления, чтобы вернуть к ним доступ.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Восстановить из фразы восстановления';

  @override
  String get walletOnboardingRecoverConfirmTitle =>
      'Восстановить этот кошелёк?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Прежде чем продолжить, убедитесь, что у вас есть фраза восстановления, — она понадобится на следующем экране для восстановления средств. Ваши средства в безопасности в блокчейне и контролируются этой фразой. Это действие удалит с устройства нечитаемые данные кошелька, чтобы его можно было пересоздать.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Отмена';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Недостаточно свободного места для настройки кошелька. Освободите немного места и повторите попытку.';

  @override
  String get walletOnboardingFailedNoVault =>
      'На этом устройстве нет защищённого хранилища ключей, поэтому кошелёк не может защитить здесь вашу фразу восстановления.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Не удалось подключиться к сети во время настройки. Проверьте подключение и повторите попытку.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'Настройка кошелька не была завершена. Повторите попытку, чтобы завершить её, — ничего не потеряно.';

  @override
  String get walletOnboardingFailedUnknown =>
      'При настройке кошелька что-то пошло не так. Повторите попытку.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'Настройка кошелька в этом приложении задана неверно, поэтому кошелёк не может запуститься. Повторная попытка не поможет — пожалуйста, сообщите об этом разработчику приложения. Ваши средства не пострадали.';

  @override
  String get walletSendButton => 'Отправить';

  @override
  String get walletSendSyncNotRunning =>
      'Синхронизация не выполняется — ваш доступный для расходования баланс не сможет обновиться';

  @override
  String get walletSendWaitingForFunds =>
      'Синхронизация ещё идёт — вы сможете отправлять, когда у вас появится доступный для расходования баланс';

  @override
  String get walletSendNoSpendableYet =>
      'Пока нет доступного для расходования баланса';

  @override
  String get walletSendSyncUnavailable =>
      'Вы сможете отправлять средства, когда синхронизация возобновится';

  @override
  String get walletSendTitle => 'Отправить';

  @override
  String get walletSendUnavailable =>
      'Кошелёк пока не готов. Вернитесь назад и повторите попытку.';

  @override
  String get walletSendWatchOnly =>
      'Это кошелёк только для просмотра. Он может показывать баланс и получать платежи, но не хранит ключи расходования — поэтому не может отправлять.';

  @override
  String get walletSendExpiredTitle =>
      'Срок действия этого запроса на оплату истёк';

  @override
  String get walletSendExpiredBody =>
      'Экран отправки открывался дольше пяти секунд, поэтому приложению сообщили, что ничего не отправлено. Этот ответ окончательный: оплатить этот запрос отсюда нельзя. Чтобы заплатить, начните заново из приложения.';

  @override
  String get walletSendFaultWatchOnly =>
      'Это кошелёк только для просмотра — он не хранит ключи расходования, поэтому не может отправлять.';

  @override
  String walletSendAvailable(String amount) {
    return 'Доступно для отправки: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Доступно для отправки: $amount ZEC — баланс всё ещё наверстывает упущенное';
  }

  @override
  String get walletSendRecipientLabel => 'Адрес получателя';

  @override
  String get walletSendRecipientHint => 'Адрес Zcash (начинается с u, z или t)';

  @override
  String get walletSendRecipientLocked => 'Получателя нельзя изменить здесь';

  @override
  String get walletSendAmountLabel => 'Сумма (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Заметка (необязательно)';

  @override
  String get walletSendMemoHint =>
      'Доставляется только защищённым (приватным) получателям';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Заметки требуют защищённого получателя. Этот публичный адрес не может её получить.';

  @override
  String get walletSendMemoMachineDisabled =>
      'К этому платежу уже прикреплена ссылка приложения, поэтому написать заметку нельзя.';

  @override
  String get walletSendMachineMemoTitle => 'Приложение прикрепляет ссылку';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Указано, что это для: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Она останется с транзакцией, и её нельзя будет удалить. Кошелёк не может проверить, что в ней.';

  @override
  String get walletSendRecipientShielded => 'Защищённый · приватный';

  @override
  String get walletSendRecipientTransparent => 'Публичный';

  @override
  String get walletSendRecipientInvalid =>
      'Это не похоже на действительный адрес Zcash.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Этот адрес предназначен для другой сети Zcash.';

  @override
  String get walletSendReviewButton => 'Проверить платёж';

  @override
  String get walletSendQueueButton => 'Поставить в очередь';

  @override
  String get walletSendQueueHint =>
      'Платёж в очереди ожидает в разделе «Сохранено и ожидает», где его можно отправить или отменить. Комиссия сети определяется в момент отправки.';

  @override
  String get walletSendPreparing => 'Подготовка платежа…';

  @override
  String get walletSendSubmitting => 'Отправка…';

  @override
  String get walletSendQueuing => 'Постановка в очередь…';

  @override
  String get walletSendReviewTitle => 'Подтверждение платежа';

  @override
  String get walletSendTotalLabel => 'Итого';

  @override
  String get walletSendFeeLabel => 'Комиссия сети';

  @override
  String get walletSendChangeLabel => 'Возвращённая сдача';

  @override
  String get walletSendDeshieldTitle => 'Этот платёж не приватен';

  @override
  String get walletSendDeshieldBody =>
      'Он отправляется на публичный адрес, поэтому сумма и получатель будут публично видны в блокчейне Zcash.';

  @override
  String get walletSendPublicAckLabel =>
      'Я понимаю, что этот платёж будет публичным.';

  @override
  String get walletSendConfirmButton => 'Отправить сейчас';

  @override
  String get walletSendBackButton => 'Назад';

  @override
  String get walletSendSelfSendNote =>
      'Вы отправляете средства на свой собственный кошелёк. Комиссия сети всё равно взимается.';

  @override
  String get walletSendLargeConfirmTitle => 'Отправить крупную сумму?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Это почти весь ваш баланс. Отправленный платёж нельзя отменить.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Это крупный платёж. Отправленный платёж нельзя отменить.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Это крупный платёж — почти весь ваш баланс. Отправленный платёж нельзя отменить.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Отправить $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Вернуться назад';

  @override
  String get walletSendSentTitle => 'Платёж отправлен';

  @override
  String get walletSendSentBody => 'Ваш платёж передан в сеть.';

  @override
  String get walletSendSavedTitle => 'Сохранено — мы завершим отправку';

  @override
  String get walletSendSavedBody =>
      'Ваш платёж не удалось отправить прямо сейчас, поэтому он сохранён — кошелёк отправит его при одной из более поздних синхронизаций. Ничего не потеряно.';

  @override
  String get walletSendKeptTitle => 'Сохранено';

  @override
  String get walletSendKeptBody =>
      'Кошелёк сохранил эту транзакцию, но не обещал отправить её сам. Её состояние можно посмотреть в разделе «Активность».';

  @override
  String get walletSendPartialBody =>
      'Часть вашего платежа отправлена; кошелёк завершит остальное при одной из более поздних синхронизаций. Ничего не потеряно.';

  @override
  String get walletSendInMotionTitle => 'Платёж выполняется';

  @override
  String get walletSendInMotionBody =>
      'Ваш платёж начал выполняться и проходит через одноразовый адрес, который контролирует ваш кошелёк. Не отправляйте его повторно. Если он не завершится, вы сможете восстановить средства на экране кошелька.';

  @override
  String get walletSendAlreadyTitle => 'Уже отправлено';

  @override
  String get walletSendAlreadyBody =>
      'Этот платёж уже был отправлен — повторно он отправлен не будет.';

  @override
  String get walletSendFailedTitle => 'Не удалось выполнить платёж';

  @override
  String get walletSendFailedBody =>
      'При выполнении платежа что-то пошло не так, ничего не было отправлено. Вы можете повторить попытку.';

  @override
  String get walletSendTryAgain => 'Повторить';

  @override
  String get walletSendDone => 'Готово';

  @override
  String get walletSendAnother => 'Отправить ещё';

  @override
  String get walletSendQueuedTitle => 'Отправка поставлена в очередь';

  @override
  String get walletSendQueuedBody =>
      'Этот платёж сохранён. Вы найдёте его в разделе «Сохранено и ожидает», где можно отправить его сейчас или отменить.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Недостаточно доступного баланса — у вас $available ZEC, а требуется $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Сеть Zcash была обновлена, и этому приложению требуется обновление, прежде чем оно сможет отправлять. Ваши средства в безопасности.';

  @override
  String get walletSyncUpToDateLimited =>
      'Синхронизировано настолько, насколько эта версия может прочитать';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Сеть Zcash была обновлена. Эта версия просканировала всё, что может прочитать, но более новые блоки могут содержать средства, которые она пока не может показать, а заметки к недавним платежам недоступны. Обновите приложение, чтобы увидеть всё.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Синхронизировано, но этот сервер обслуживает не все пулы';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Этот сервер отказывает, скрывает или неверно сообщает данные одного из защищённых пулов Zcash. Средства, полученные в этом пуле, нельзя потратить через него, а показанный баланс — это нижняя граница. Переключитесь на другой сервер, чтобы использовать их — это не проблема соединения.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: этот сервер отказывается его обслуживать';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: этот сервер скрывает его часть';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: этот сервер сообщает о нём неверные данные';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: неизвестно, обслуживает ли его этот сервер';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Синхронизировано с этим сервером, но сервер отстаёт от сети';

  @override
  String get walletSyncExplainEndpointBehind =>
      'Цепочка этого сервера заканчивается на блоке, который сеть прошла ещё до сборки этой версии приложения, поэтому баланс актуален только до этого блока. Новые входящие платежи могут пока не отображаться, а платёж, отправленный отсюда, может не дойти. Переключитесь на другой сервер, чтобы догнать сеть — это не проблема соединения.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Ожидание обновления приложения — ваши средства в безопасности, ничего не отправлено.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Ожидание сервера, который сообщает версию сети — переключите сервер. Ваши средства в безопасности, ничего не отправлено.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Ожидание сервера, который сообщает версию сети. Если дата и время на этом устройстве неверны, сначала исправьте их — а затем переключите сервер. Ваши средства в безопасности, ничего не отправлено.';

  @override
  String get walletSyncUnverified =>
      'Синхронизировано, но этот сервер не сообщает версию сети';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'Отправка ещё работает около $hours часа — затем переключите сервер.',
      many:
          'Отправка ещё работает около $hours часов — затем переключите сервер.',
      few:
          'Отправка ещё работает около $hours часов — затем переключите сервер.',
      one:
          'Отправка ещё работает около $hours часа — затем переключите сервер.',
      zero: 'Отправка ещё работает менее часа — затем переключите сервер.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'Отправка ещё работает около $blocks блоков — затем переключите сервер.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Этот сервер не сообщает версию сети уже $blocks блоков, поэтому приложение не может подтвердить, что отправлять безопасно. Переключитесь на другой сервер.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Этот сервер не сообщает версию сети уже сутки, поэтому приложение не может подтвердить, что отправлять безопасно. Если дата и время на этом устройстве неверны, сначала исправьте их — а затем переключитесь на сервер, который сообщает версию сети.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Этот сервер ни разу не сообщил версию сети, поэтому приложение не может подтвердить, что отправлять безопасно. Переключитесь на другой сервер.';

  @override
  String get walletSyncExplainUnverified =>
      'Этот сервер не сообщает, на какой версии сети Zcash он работает, поэтому приложение не может подтвердить, что подписанный им платёж будет принят. Ваш баланс актуален. Переключитесь на другой сервер — это не проблема соединения.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Этот сервер не сообщает, на какой версии сети Zcash он работает, поэтому приложение не может подтвердить, что подписанный им платёж будет принят. Он также продолжал отдавать блоки, которые этому кошельку затем пришлось откатить, поэтому ваш баланс может быть неактуален. Переключитесь на другой сервер — это не проблема соединения.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Этот сервер также продолжает отдавать блоки, которые кошельку затем приходится откатывать — переключите сервер.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Баланс всё ещё наверстывает упущенное — по мере синхронизации кошелька может стать доступно больше.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC ещё поступает и станет доступно для расходования, когда кошелёк догонит сеть.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Введите сумму для отправки.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Введите сумму в виде числа, например 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'У ZEC не более 8 знаков после запятой.';

  @override
  String get walletSendFaultAmountNotPositive => 'Введите сумму больше нуля.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Эта сумма превышает весь объём выпущенных ZEC.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Это приложение сейчас ограничивает отправку суммой $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Это не похоже на действительный адрес Zcash для этой сети. Проверьте его и повторите попытку.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Этот получатель не может получить заметку. Удалите заметку или отправьте на защищённый (приватный) адрес.';

  @override
  String get walletSendFaultMemoTooLong =>
      'Заметка слишком длинная. Сократите её и повторите попытку.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Эту заметку нельзя отправить. Удалите её и повторите попытку.';

  @override
  String get walletSendFaultMemoConflict =>
      'Не удалось отправить этот платёж — приложение прикрепило к нему две заметки. Ничего не отправлено.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Этот адрес предназначен для другой сети.';

  @override
  String get walletSendFaultUriInvalid =>
      'Не удалось сформировать платёж. Проверьте адрес и сумму.';

  @override
  String get walletSendFaultNotSynced =>
      'Кошелёк ещё не синхронизирован в достаточной мере. Дождитесь завершения синхронизации или отложите отправку на потом.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Кошелёк ещё не синхронизирован в достаточной мере. Дождитесь завершения синхронизации.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Кошелёк ещё не синхронизирован в достаточной мере, а синхронизация сейчас не выполняется. Проверьте статус синхронизации на экране кошелька.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Пока вы проверяли платёж, суммы устарели. Пожалуйста, проверьте платёж заново.';

  @override
  String get walletSendFaultQueueFull =>
      'Слишком много отправок ожидает своей очереди. Дождитесь их отправки, затем повторите попытку.';

  @override
  String get walletSendFaultWalletBusy =>
      'Кошелёк сейчас занят. Повторите попытку через некоторое время.';

  @override
  String get walletSendFaultStorageFull =>
      'Недостаточно свободного места для завершения этой отправки. Освободите немного места и повторите попытку.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Сейчас используется слишком много одноразовых адресов. Часть из них может освободиться по мере подтверждения переводов, но это может не решиться само собой. Ваши средства в безопасности.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Не удалось подготовить платёж. Проверьте данные и повторите попытку.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Не удалось подготовить этот платёж прямо сейчас. Повторите попытку через минуту.';

  @override
  String get walletSwapButton => 'Обмен';

  @override
  String get walletSwapTitle => 'Обмен ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'Кошелёк пока не готов. Вернитесь назад и повторите попытку.';

  @override
  String get walletSwapUnavailableOff => 'Обмен сейчас недоступен.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Это кошелёк только для просмотра — он не может обменивать.';

  @override
  String get walletSwapDone => 'Готово';

  @override
  String get walletSwapBackToWallet => 'Назад к кошельку';

  @override
  String walletSwapAvailable(String amount) {
    return 'Доступно для обмена: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Доступно для обмена: $amount ZEC — баланс всё ещё наверстывает упущенное';
  }

  @override
  String get walletSwapAssetLabel => 'Получаемый актив';

  @override
  String get walletSwapAmountLabel => 'Сумма для обмена (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Адрес назначения';

  @override
  String get walletSwapDestinationHint =>
      'Ваш адрес для получения в целевой сети';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Ваш адрес получения в сети $chain';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Адрес сети $chain — куда будет отправлен полученный после обмена актив. Внимательно проверьте, что сеть указана верно.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Отсканировать QR-код адреса назначения';

  @override
  String get walletSwapTargetAssetHint => 'Выберите актив для получения';

  @override
  String get walletSwapQuoteButton => 'Получить курс';

  @override
  String get walletSwapQuoting => 'Получение курса…';

  @override
  String get walletSwapExecuting => 'Запуск обмена…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Всё ещё выполняется — обмен запускается. Это может занять до минуты.';

  @override
  String get walletSwapReviewTitle => 'Подтверждение обмена';

  @override
  String get walletSwapYouSendLabel => 'Вы отправите';

  @override
  String get walletSwapYouReceiveLabel => 'Вы получите не менее';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Комиссия сети';

  @override
  String get walletSwapNetworkFeeValue => 'Добавляется при отправке депозита';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Курс действителен ещё примерно $time — подтвердите обмен до истечения срока.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Курс действителен ещё менее минуты — подтвердите обмен до истечения срока.';

  @override
  String get walletSwapQuoteExpired =>
      'Срок действия этого курса истёк. Вернитесь назад и получите новый — прежний курс больше не гарантирован, и отправка сейчас рискует привести к возврату средств.';

  @override
  String get walletCountdownUnderMinute => 'менее минуты';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes мин';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds с';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '$hours ч $minutes мин';
  }

  @override
  String get walletSwapDeshieldTitle => 'Этот обмен не приватен';

  @override
  String get walletSwapDeshieldBody =>
      'Обмен ZEC на другой актив снимает защиту с ваших ZEC — депозит представляет собой публичную транзакцию, а сторона поставщика публична в его сети.';

  @override
  String get walletSwapDiscloseTitle => 'Что увидит поставщик обмена';

  @override
  String get walletSwapDiscloseAmounts => 'Суммы с обеих сторон';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Что эти ZEC и получаемый вами актив — часть одного обмена';

  @override
  String get walletSwapDiscloseDestination => 'Ваш адрес назначения';

  @override
  String get walletSwapDiscloseSource => 'Ваш исходный адрес';

  @override
  String get walletSwapDiscloseIp =>
      'Ваш IP-адрес (если вы не используете Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'Другие сведения об этом обмене';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'Собственные транзакции поставщика публичны в его сети';

  @override
  String get walletSwapAckLabel =>
      'Я понимаю, что поставщик увидит информацию, указанную выше.';

  @override
  String get walletSwapConfirmButton => 'Начать обмен';

  @override
  String get walletSwapBackButton => 'Назад';

  @override
  String get walletSwapStatusPendingTitle => 'Обмен начат';

  @override
  String get walletSwapStatusCheckingTitle => 'Проверка статуса обмена…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Ваш кошелёк отправляет депозит ZEC поставщику. Если вы ненадолго окажетесь офлайн, перевод отправится автоматически, как только соединение восстановится — но окно для отправки короткое, и если оно закроется раньше, обмен просто завершится, и ничего не будет обменяно. Ваш ZEC остаётся вашим, но может потребоваться до часа, прежде чем он снова станет доступен для расходования.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'Ожидаем поступления вашего депозита. Если вы ещё не отправили средства с другого кошелька, сделайте это до истечения срока действия курса.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Этот обмен всё ещё ожидает депозита. Инструкции по депозиту больше не доступны на этом устройстве — если вы уже отправили средства, они будут обнаружены; если нет, дайте этому обмену истечь и начните новый.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'Окно для депозита закрывается $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'Окно для депозита истекло. Если депозит не был отправлен вовремя, обмен завершится, а ваш ZEC останется в вашем кошельке.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'Окно для депозита истекло. Если вы ещё не отправили депозит, этот обмен просто завершится — получите новый курс, когда будете готовы.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Обмены выполняются',
      many: 'Обмены выполняются',
      few: 'Обмены выполняются',
      one: 'Обмен выполняется',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'Ваш ZEC уже в пути к поставщику.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'Ожидаем, когда ваш депозит поступит к поставщику.';

  @override
  String get walletSwapInFlightRowGeneric => 'Обмен выполняется.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'Окно для депозита истекло — проверьте статус этого обмена.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Этот обмен пока не достиг подтверждённого результата здесь — откройте его, чтобы проверить. Любой ZEC, возвращающийся в этот кошелёк, появится в вашем балансе после синхронизации.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Этот обмен пока не достиг подтверждённого результата здесь — откройте его, чтобы проверить. Любой ZEC, который этот обмен доставит в этот кошелёк, появится в вашем балансе после синхронизации.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Обмен завершён.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Обмен возвращён.';

  @override
  String get walletSwapRowOutcomeFailed => 'Обмен не завершён.';

  @override
  String get walletSwapRemove => 'Удалить';

  @override
  String get walletSwapRemoveTitle => 'Удалить этот обмен из списка?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Это только удалит обмен из этого списка — сам обмен не будет отменён, и этот кошелёк перестанет отслеживать возврат средств по нему. ZEC, возвращённый позже, всё равно принадлежит этому кошельку; полное повторное сканирование может его найти.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Это только удалит обмен из этого списка — сам обмен не будет отменён, и этот кошелёк перестанет отслеживать поступающий ZEC. ZEC, доставленный позже, всё равно принадлежит этому кошельку; полное повторное сканирование может его найти. Если вместо этого обмен будет возвращён, возврат вернётся в активе, который вы отправили, — вне этого кошелька.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Это только удалит обмен из этого списка — сам обмен не будет отменён, и этот кошелёк перестанет отслеживать ZEC, который всё ещё поступает по нему. ZEC, который поступит позже, всё равно принадлежит этому кошельку; полное повторное сканирование может его найти.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Это удалит завершённый обмен из списка.';

  @override
  String get walletSwapRemoveCancel => 'Отмена';

  @override
  String get walletSwapRemoveConfirm => 'Удалить';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Начат $time';
  }

  @override
  String get walletSwapViewSwap => 'Показать обмен';

  @override
  String get walletSwapsInFlightError =>
      'Не удалось загрузить текущие обмены сейчас.';

  @override
  String get walletSwapsInFlightRetry => 'Повторить';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Попытка…';

  @override
  String get walletSwapStartAnother => 'Начать ещё один обмен';

  @override
  String get walletSwapStatusUnderTitle => 'Ожидание полного депозита';

  @override
  String get walletSwapStatusUnderBody =>
      'Часть депозита поступила. Остальное завершается, либо поставщик вернёт средства.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Часть вашего депозита поступила. Отправьте недостающую сумму до истечения срока, иначе поставщик вернёт то, что поступило.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Получено $received; ещё не хватает $missing. Окно для депозита закрывается: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Депозит получен';

  @override
  String get walletSwapStatusDetectedBody =>
      'Поставщик получил ваш депозит и обработает обмен.';

  @override
  String get walletSwapStatusProcessingTitle => 'Обработка обмена';

  @override
  String get walletSwapStatusProcessingBody => 'Поставщик завершает ваш обмен.';

  @override
  String get walletSwapStatusSuccessTitle => 'Обмен завершён';

  @override
  String get walletSwapStatusSuccessBody => 'Ваш обмен успешно завершён.';

  @override
  String get walletSwapStatusRefundedTitle => 'Обмен возвращён';

  @override
  String get walletSwapStatusRefundedBody =>
      'Обмен не был завершён, поэтому поставщик отправил средства обратно на ваш адрес для возврата.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'Обмен не был завершён, поэтому поставщик отправил ваш ZEC обратно в этот кошелёк. Средства поступят как незащищённые и появятся в вашем балансе после следующей синхронизации кошелька — это может занять некоторое время.';

  @override
  String get walletSwapStatusFailedTitle => 'Обмен не удался';

  @override
  String get walletSwapStatusFailedBody =>
      'Обмен не удалось завершить. Внесённые средства будут зачислены или возвращены на стороне поставщика.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Обмен не найден';

  @override
  String get walletSwapStatusNotFoundBody =>
      'У поставщика больше нет записи об этом обмене — скорее всего, срок его действия истёк. Если депозит был внесён, поставщик должен вернуть его на адрес для возврата. Обмен останется в вашем списке, и этот кошелёк продолжит отслеживать его ZEC на случай, если он всё же поступит; вы можете удалить его из списка в любое время.';

  @override
  String get walletSwapStatusUnknownTitle => 'Статус недоступен';

  @override
  String get walletSwapStatusUnknownBody =>
      'Сейчас не удаётся прочитать статус этого обмена.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'Отслеживание недоступно';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'Обмен отключён, поэтому отследить его здесь нельзя. Средства будут зачислены или возвращены на стороне поставщика.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'Здесь обмен отключён, поэтому отследить этот обмен сейчас нельзя. Если средства были возвращены, ZEC вернётся в этот кошелёк — он появится в вашем балансе после того, как обмен снова включат и кошелёк синхронизируется.';

  @override
  String get walletSwapTrackingError => 'Не удалось отследить этот обмен.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Не удалось открыть отслеживание этого обмена. Сам обмен, возможно, всё ещё продолжается — внесённые средства будут зачислены или возвращены на стороне поставщика.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Введите адрес, на который вы хотите получить обменянный актив.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Этот адрес назначения недействителен для данного актива. Проверьте его и повторите попытку.';

  @override
  String get walletSwapFaultExpired =>
      'Срок действия этого курса истёк. Получите новый курс, чтобы продолжить.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'Цена поставщика вышла за пределы установленного вами лимита, поэтому обмен был остановлен до того, как что-либо переместилось. Повторите попытку.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'Лимит проскальзывания слишком высок для безопасного обмена. Повторите попытку.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'Поставщик обмена сейчас недоступен. Повторите попытку через некоторое время.';

  @override
  String get walletSwapFaultConnection =>
      'Не удалось связаться со службой обмена. Проверьте подключение к интернету и повторите попытку.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'Поставщик обмена вернул неожиданный ответ, поэтому обмен был остановлен. Повторите попытку.';

  @override
  String get walletSwapFaultSwapOff => 'Обмен сейчас отключён.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Не удалось отправить ваш депозит, поэтому из кошелька ничего не списано. Получите новый курс, чтобы повторить попытку.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Обмен уже выполняется. Начать новый можно будет только после того, как текущий будет полностью завершён и подтверждён в сети, либо истечёт срок действия курса — это может занять некоторое время.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Кошелёк пока не может настроить адрес для возврата средств — обычно это означает, что первая синхронизация ещё не завершена. Дождитесь завершения синхронизации и повторите попытку.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Кошелёк пока не может настроить адрес для получения этого обмена — обычно это означает, что первая синхронизация ещё не завершена. Дождитесь завершения синхронизации и повторите попытку.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'Обмен не удалось запустить вовремя — возможно, соединение было медленным, либо кошелёк был занят. Получите новый курс и повторите попытку.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'Кошелёк на мгновение занят. Повторите попытку.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Этот курс не совпадает с тем, который выдал ваш кошелёк, поэтому ничего не отправлено. Получите новый курс и попробуйте снова.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Для этого обмена нужно около $needed ZEC, включая комиссию сети, но сейчас доступно для расходования только $spendable ZEC.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Это приложение сейчас ограничивает обмен суммой $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Для этого обмена нужно около $needed ZEC, включая комиссию сети, но сейчас доступно для расходования только $spendable ZEC. Ваш баланс всё ещё наверстывает упущенное — вскоре может стать доступно больше.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'Кошельку не удалось безопасно записать этот обмен, поэтому ничего не было перемещено. Повторите попытку.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Этот запрос на обмен не удалось обработать. Получите новый курс и повторите попытку.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Не удалось получить курс обмена. Проверьте данные и повторите попытку.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Кошелёк пока не готов. Вернитесь назад и повторите попытку.';

  @override
  String get walletSwapDirectionBuy => 'Купить ZEC';

  @override
  String get walletSwapDirectionSell => 'Продать ZEC';

  @override
  String get walletSwapRefundLabel => 'Ваш адрес для возврата';

  @override
  String get walletSwapRefundHint =>
      'Куда вернутся ваши монеты, если обмен не удастся';

  @override
  String get walletSwapRefundHelper =>
      'В сети, из которой вы отправляете, — не адрес Zcash.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Ваш адрес для возврата в сети $chain';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Адрес сети $chain — куда вернутся ваши монеты, если обмен не удастся. Не адрес Zcash.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'Об адресе для возврата';

  @override
  String get walletSwapRefundInfoBody =>
      'Если обмен не удастся завершить, поставщик отправит ваши монеты обратно на этот адрес в сети, из которой вы платили. Указывайте адрес, который контролируете вы сами, — кошелёк не может проверить сторонний адрес за вас, поэтому проверяйте его внимательно.';

  @override
  String get walletSwapRefundScanTooltip =>
      'Отсканировать QR-код адреса для возврата';

  @override
  String get walletSwapScanTitle => 'Сканирование адреса';

  @override
  String get walletSwapScanInstruction => 'Наведите камеру на QR-код адреса.';

  @override
  String get walletSwapScanManualEntry => 'Ввести вручную';

  @override
  String get walletSwapScanCancel => 'Отмена';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Камера недоступна. Введите адрес вручную ниже.';

  @override
  String get walletSwapSourceAssetLabel => 'Актив для обмена';

  @override
  String get walletSwapSourceAssetHint => 'Выберите актив';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Сумма для отправки ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Сумма для отправки';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol в сети $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Выберите актив для обмена';

  @override
  String get walletSwapPickerTitleReceive => 'Выберите актив для получения';

  @override
  String get walletSwapPickerStale =>
      'Не удалось обновить список активов — показан последний известный список.';

  @override
  String get walletSwapPickerEmpty =>
      'Сейчас нет доступных активов для обмена. Попробуйте позже.';

  @override
  String get walletSwapPickerSearchHint => 'Поиск по названию или сети';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Нет активов, соответствующих запросу «$query».';
  }

  @override
  String get walletSwapPickerError =>
      'Не удалось загрузить список активов. Проверьте подключение и повторите попытку.';

  @override
  String get walletSwapPickerRetry => 'Повторить';

  @override
  String get walletSwapSlippageLabel => 'Допустимое проскальзывание';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Своё';

  @override
  String get walletSwapSlippageCustomLabel => 'Своё значение проскальзывания';

  @override
  String get walletSwapSlippageMayFail =>
      'Очень низкое — обмен может не удаться при изменении цены.';

  @override
  String get walletSwapSlippageNormal => 'Безопасное значение.';

  @override
  String get walletSwapSlippageRisky =>
      'Высокое — вы можете получить заметно меньше, чем указано в курсе.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Слишком высокое — обмен будет отклонён. Снизьте до 10% или меньше.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Вы получите не менее $zec ZEC — это ваш минимум при допустимом проскальзывании $slippage%. Итоговая сумма не может быть ниже этого значения.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'Вы получаете ZEC на собственный адрес';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Пока вы не защитите их — это одно нажатие, о котором вам напомнят при поступлении — полученная сумма будет ненадолго публичной и видимой в блокчейне. Небольшое поступление может оставаться публичным, пока не накопится достаточная сумма.';

  @override
  String get walletSwapRefundVerifyTitle => 'Проверьте адрес для возврата';

  @override
  String get walletSwapRefundVerifyBody =>
      'Проверьте его посимвольно — именно сюда вернутся ваши монеты, если обмен не удастся. Кошелёк не может проверить сторонний адрес за вас.';

  @override
  String get walletSwapRefundVerifyAck =>
      'Адрес для возврата проверен и указан верно.';

  @override
  String get walletSwapPayoutVerifyTitle => 'Проверьте адрес получения';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Проверьте его посимвольно — именно на этот адрес вы получите $asset. Кошелёк не может проверить сторонний адрес за вас.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'Адрес получения проверен и указан верно.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'Здесь обмен отключён. Любые ZEC, уже находящиеся в пути, появятся в кошельке после следующей синхронизации.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Введите сумму, которую хотите обменять.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Введите адрес для возврата в исходной сети.';

  @override
  String get walletSwapDepositTitle => 'Отправьте платёж';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Отправьте ровно $amount $asset в сети $chain на указанный ниже адрес.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Отправьте точную сумму. Если отправить меньше или после закрытия окна для отправки, поставщик вернёт средства на ваш адрес для возврата.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Окно для депозита: осталось $time';
  }

  @override
  String get walletSwapDepositExpired =>
      'Окно для депозита закрыто. Не отправляйте средства сейчас — начните новый обмен. Если вы уже отправили, поставщик должен вернуть средства на ваш адрес для возврата.';

  @override
  String get walletSwapDepositQrLabel => 'QR-код адреса для депозита';

  @override
  String get walletSwapDepositAddressLabel => 'Адрес для депозита';

  @override
  String get walletSwapDepositCopy => 'Скопировать адрес для депозита';

  @override
  String get walletSwapDepositCopied => 'Адрес для депозита скопирован';

  @override
  String get walletSwapDepositMemoRequired =>
      'Для этого депозита нужна заметка/тег';

  @override
  String get walletSwapDepositMemoWarning =>
      'Вы ОБЯЗАНЫ указать именно эту заметку при депозите. Отправка без неё — или с неверной заметкой — может привести к безвозвратной потере средств.';

  @override
  String get walletSwapDepositMemoLabel => 'Заметка/тег для депозита';

  @override
  String get walletSwapDepositMemoCopy => 'Скопировать заметку';

  @override
  String get walletSwapDepositMemoCopied => 'Заметка скопирована';

  @override
  String get walletSwapDepositSent => 'Средства отправлены';

  @override
  String get walletSwapDepositBackTitle => 'Покинуть этот экран?';

  @override
  String get walletSwapDepositBackBody =>
      'Это не отменит обмен — он продолжится в фоне. Но для оплаты вам понадобится адрес депозита, поэтому сначала скопируйте его, если ещё не сделали этого.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Это не отменит обмен — он продолжится в фоне. Окно для депозита закрыто. Не отправляйте средства на адрес депозита сейчас. Если вы уже отправили, поставщик должен вернуть средства на ваш адрес для возврата.';

  @override
  String get walletSwapDepositBackStay => 'Остаться';

  @override
  String get walletSwapDepositBackLeave => 'Покинуть';

  @override
  String get walletReceive => 'Получить';

  @override
  String get walletReceiveSubtitle =>
      'Поделитесь этим адресом, чтобы получить ZEC. Его можно безопасно публиковать открыто.';

  @override
  String get walletReceiveCopy => 'Скопировать адрес';

  @override
  String get walletReceiveCopied => 'Адрес скопирован';

  @override
  String get walletReceiveUnavailable => 'Кошелёк ещё не готов.';

  @override
  String get walletReceiveError =>
      'Не удалось загрузить ваш адрес. Повторите попытку.';

  @override
  String get walletReceivePreparing => 'Подготовка адреса…';

  @override
  String get walletReceivePreparingHint =>
      'Ваш кошелёк готовит этот адрес на вашем устройстве — это может занять некоторое время, если кошелёк занят другой работой.';

  @override
  String get walletReceiveRetry => 'Повторить';

  @override
  String get walletReceiveQrLabel => 'QR-код вашего адреса для получения';

  @override
  String get walletReceiveTypeShielded => 'Защищённый';

  @override
  String get walletReceiveTypeTransparent => 'Публичный';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Поделитесь этим публичным адресом, чтобы получить ZEC от отправителя, который не может платить на защищённый адрес.';

  @override
  String get walletReceiveTransparentWarning =>
      'Это публичный адрес: он виден в блокчейне и связывает ваши платежи при повторном использовании. Предпочтительнее использовать защищённый адрес; после получения защитите эти средства.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'QR-код вашего публичного адреса для получения';

  @override
  String get walletReceiveFreshAddress => 'Использовать новый адрес';

  @override
  String get walletReceiveFreshCaption =>
      'Новый адрес — его нельзя связать с другими вашими адресами. Платежи на него всё равно поступают в этот кошелёк, а ваши прежние адреса продолжают работать. Он больше не будет показан здесь — скопируйте его сейчас.';

  @override
  String get walletReceiveFreshError =>
      'Не удалось создать новый адрес. Повторите попытку.';

  @override
  String get walletReceiveFreshBusy =>
      'Кошелёк сейчас занят. Повторите попытку с новым адресом через мгновение.';

  @override
  String get walletReceiveShare => 'Поделиться';

  @override
  String get walletReceiveRequestAmount => 'Запросить сумму';

  @override
  String get walletReceiveRequestAmountLabel => 'Сумма (необязательно)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Он больше не будет показан здесь — скопируйте его сейчас.';

  @override
  String get walletSecurityMenuItem => 'Безопасность…';

  @override
  String get securityTitle => 'Безопасность';

  @override
  String get securityUnavailableBody =>
      'Настройки безопасности кошелька управляются этим приложением, а не самим кошельком.';

  @override
  String get securityCustodySectionTitle => 'Хранение ключей';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (аппаратно)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (аппаратно)';

  @override
  String get securityCustodyTierTee => 'Аппаратное хранилище ключей (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Программное хранилище ключей';

  @override
  String get securityCustodyTierKeychain => 'Keychain (программное шифрование)';

  @override
  String get securityCustodyTierNone => 'Нет аппаратного хранилища ключей';

  @override
  String get securityCustodyTierUnknown => 'Неизвестно';

  @override
  String get securityCustodyHardwareKey =>
      'Ключ, которым заблокирован этот кошелёк, хранится в защищённом оборудовании этого устройства и удаляется вместе с кошельком.';

  @override
  String get securityCustodyBestEffort =>
      'Удаление удаляет ключи «по мере возможности»: до тех пор, пока устройство не перезапишет освободившееся хранилище, может оставаться короткое окно для криминалистического восстановления. Для полной уверенности дополнительно используйте функцию устройства «Стереть все настройки и содержимое».';

  @override
  String get securityCustodyProbeError =>
      'Не удалось прочитать статус хранения ключей. Вернитесь назад и повторите попытку.';

  @override
  String get securityDeleteWalletButton => 'Удалить кошелёк';

  @override
  String get securityDeleteWalletSubtitle =>
      'Удалить этот кошелёк и его ключ с этого устройства. Ваши средства остаются в блокчейне и восстановимы из фразы восстановления.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Удалить этот кошелёк и его ключ с этого устройства. Он не содержит ключей расходования, поэтому нечего резервировать, — добавьте его снова в любой момент с помощью ключа просмотра.';

  @override
  String get securityDeleteDialogTitle => 'Удалить этот кошелёк?';

  @override
  String get securityDeleteDialogBody =>
      'Это удалит кошелёк и его ключ с этого устройства. Убедитесь, что вы сохранили фразу восстановления, — это ЕДИНСТВЕННЫЙ способ восстановить средства.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Это удалит кошелёк и его ключ с этого устройства. Он не содержит ключей расходования, поэтому нечего резервировать, — вы сможете снова добавить его позже с помощью ключа просмотра.';

  @override
  String get securityDeleteDialogConfirm => 'Удалить';

  @override
  String get securityDeleteDialogCancel => 'Отмена';

  @override
  String get securityDeleteFailedSnack =>
      'Не удалось удалить кошелёк — он остался без изменений. Повторите попытку.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Сначала завершите смену сервера — она завершится или остановится в течение $seconds секунд. Затем попробуйте удалить кошелёк снова.';
  }

  @override
  String get walletParkedTitle => 'Сохранено и ожидает';

  @override
  String get walletParkedSubtitle =>
      'Эти платежи ещё не были отправлены. Их суммы по-прежнему учтены в вашем балансе.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Эти платежи ещё не были отправлены. Их суммы по-прежнему учтены в вашем балансе — за исключением тех, что кошелёк сейчас отправляет: их сумма, возможно, уже отложена.';

  @override
  String get walletParkedCancel => 'Отменить';

  @override
  String get walletParkedPausedHint =>
      'Приостановлено — этот платёж не отправится сам по себе. Ваши средства в безопасности. Отправьте его сейчас или отмените.';

  @override
  String get walletParkedRetryStale =>
      'Этот платёж больше не ожидает. Проверьте ожидающие платежи и вкладку «Активность».';

  @override
  String get walletParkedAlreadyInProgress =>
      'Этот платёж больше не ожидает — возможно, кошелёк уже отправляет его. Проверьте «Сохранено и ожидает» и вкладку «Активность».';

  @override
  String get walletReclaimExplainer =>
      'Отправка через одноразовые адреса застряла. Вы можете возобновить её — небольшая сумма переместится между вашими собственными адресами и вернётся обратно.';

  @override
  String get walletReclaimButton => 'Возобновить отправку';

  @override
  String get walletReclaimInProgress => 'Возобновление…';

  @override
  String get walletReclaimConfirmTitle =>
      'Возобновить отправку через одноразовые адреса?';

  @override
  String get walletReclaimConfirmBody =>
      'Это переместит небольшую сумму между вашими собственными адресами, чтобы освободить отправку через одноразовые адреса, а затем вернёт её обратно. Расходы составят пару комиссий сети. После подтверждения верните перемещённую сумму с помощью кнопки «Восстановить сейчас».';

  @override
  String get walletReclaimConfirmCancel => 'Не сейчас';

  @override
  String get walletReclaimConfirmAction => 'Возобновить';

  @override
  String get walletReclaimStarted =>
      'Возобновление запущено. После подтверждения отправьте приостановленный платёж, а затем верните перемещённую сумму с помощью кнопки «Восстановить сейчас».';

  @override
  String get walletReclaimNothing => 'Сейчас нечего возобновлять.';

  @override
  String get walletReclaimNotBroadcast =>
      'Не удалось подтвердить, что операция дошла до сети. Она всё же могла пройти — повторите попытку через некоторое время.';

  @override
  String get walletReclaimNeedsFunds =>
      'Вам нужно немного защищённых ZEC, чтобы возобновить отправку.';

  @override
  String get walletReclaimFailed =>
      'Не удалось возобновить отправку сейчас. Средства остались без изменений. Повторите попытку.';

  @override
  String get walletReclaimUnknown =>
      'Возобновление завершено. Проверьте свои отправки через одноразовые адреса и верните любую перемещённую сумму с помощью кнопки «Восстановить сейчас».';

  @override
  String get walletParkedError =>
      'Не удалось загрузить ожидающие платежи сейчас.';

  @override
  String get walletParkedErrorRetry => 'Повторить';

  @override
  String get walletParkedErrorRetryInProgress => 'Попытка…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Отменить этот ожидающий платёж?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Это удалит сохранённый платёж. Он ещё не был отправлен, поэтому из кошелька ничего не спишется — но это действие нельзя отменить.';

  @override
  String get walletParkedCancelConfirmKeep => 'Оставить';

  @override
  String get walletParkedCancelConfirmDiscard => 'Удалить платёж';

  @override
  String get walletParkedCancelDone => 'Ожидающий платёж отменён.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Этот платёж, возможно, уже в пути — проверьте вкладку «Активность».';

  @override
  String get walletParkedCancelFailed =>
      'Не удалось отменить платёж сейчас. Платёж остался без изменений. Повторите попытку.';

  @override
  String get walletRecoverNow => 'Восстановить сейчас';

  @override
  String get walletRecoverConfirmTitle => 'Восстановить в защищённый баланс?';

  @override
  String get walletRecoverConfirmBody =>
      'Это проверит ваши одноразовые адреса и переместит найденные средства в приватный защищённый баланс. Эту операцию безопасно повторять в любое время.';

  @override
  String get walletRecoverConfirmCancel => 'Не сейчас';

  @override
  String get walletRecoverConfirmAction => 'Восстановить';

  @override
  String get walletRecoverInProgress => 'Восстановление…';

  @override
  String walletRecoverDone(String amount) {
    return 'Восстановление $amount в защищённый баланс.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'Восстановление $amount — часть средств требует ещё одной попытки.';
  }

  @override
  String get walletRecoverRetry =>
      'Часть средств требует ещё одной попытки — запустите восстановление снова.';

  @override
  String get walletRecoverTruncated =>
      'Проверены ещё не все одноразовые адреса — запустите снова, чтобы проверить остальные.';

  @override
  String get walletRecoverNothing => 'Сейчас нечего восстанавливать.';

  @override
  String get walletRecoverFailed =>
      'Не удалось выполнить восстановление сейчас. Средства остались без изменений. Повторите попытку.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount сохранено и ожидает · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Отменить платёж на $amount, сохранённый $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount приостановлено · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount готовится к отправке · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Ваш кошелёк готовит этот платёж — его сумма, возможно, уже зарезервирована. Ваши средства в безопасности. Если процесс не завершится, платёж сам вернётся в список.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Ваш кошелёк готовит этот платёж — его сумма, возможно, уже зарезервирована. Ваши средства в безопасности, но платёж сможет завершиться только тогда, когда ваш кошелёк снова будет синхронизироваться.';

  @override
  String get walletParkedSendNow => 'Отправить сейчас';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Отправка платежа на $amount, сохранённого $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Отправить сейчас платёж на $amount, сохранённый $time';
  }

  @override
  String get walletParkedSendNowInProgress => 'Отправка…';

  @override
  String get walletParkedAuthorizeSent => 'Ваш платёж отправляется сейчас.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Ваш платёж отправляется сейчас. Если он не пройдёт, кошелёк сможет завершить платёж только тогда, когда снова будет синхронизироваться.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Пока не готово к отправке. Ваш платёж сохранён и не изменился.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Пока не готово к отправке. Ваш платёж сохранён и больше не приостановлен — попробуйте позже снова воспользоваться кнопкой «Отправить сейчас» или отмените его.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Не удалось отправить платёж сейчас. Платёж остался без изменений. Повторите попытку.';

  @override
  String get walletTransparentFundsMenuItem => 'Публичные средства…';

  @override
  String get walletTransparentFundsTitle => 'Публичные средства';

  @override
  String get walletTransparentFundsIntro =>
      'Публичные средства публично видны в блокчейне — сумма, адреса и история монет.';

  @override
  String get walletExpertToggleLabel => 'Дополнительно: публичные средства';

  @override
  String get walletExpertToggleDescription =>
      'Показать дополнительные настройки для хранения публичных средств и отключения автоматической защиты.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Показать дополнительные настройки для хранения публичных средств.';

  @override
  String get walletAutoShieldToggleLabel => 'Защищать автоматически';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Когда ваш публичный баланс достигает $minZec ZEC, средства автоматически перемещаются в защищённый баланс. Если это отключено, публичные средства остаются публично видимыми, пока вы не защитите их самостоятельно.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Не удалось сохранить настройку. Повторите попытку.';

  @override
  String get walletAutoShieldIncomplete =>
      'Автоматическая защита не завершилась — эти средства по-прежнему публично видны. Вы можете защитить их сейчас.';

  @override
  String get walletSendPrivacyShielded =>
      'Защищённый платёж — сумма и получатель остаются приватными в блокчейне.';

  @override
  String get walletSendPrivacyTransparent =>
      'Публичный платёж — сумма и адреса видны в блокчейне.';

  @override
  String get walletActivityPublicBadge => 'Публично видно в блокчейне';

  @override
  String get walletShieldWalletEnded =>
      'Сеанс работы с кошельком завершён. Закройте и снова откройте кошелёк, чтобы повторить попытку.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Новые публичные средства автоматически защищаются и переводятся в ваш приватный баланс, как только их сумма достигает $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'Автоматическая защита отключена — публичные средства остаются публично видимыми, пока вы их не защитите.';

  @override
  String get walletMoveAutoShieldNote =>
      'Автоматическая защита включена: после поступления эти средства будут автоматически защищены (за отдельную комиссию). Чтобы оставить их публичными, сначала отключите автоматическую защиту в разделе «Публичные средства».';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'После этого перевода ваш публичный баланс составит $amount ZEC — меньше $floor ZEC, необходимых для повторной защиты. Он останется публичным, пока не поступят ещё средства.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Вы переводите средства на собственный публичный адрес. Запись об этом переводе навсегда останется в публичном реестре.';

  @override
  String get walletTxDetailVisibility => 'Видимость';

  @override
  String get walletTransparentFundsAutoDenied =>
      'Автоматическая защита приостановлена для этого сеанса — она не была одобрена. Вы всё ещё можете защитить средства вручную.';

  @override
  String get walletDeepScanMenuItem => 'Проверить старые адреса обмена…';

  @override
  String get walletMenuSyncNotRunningHint =>
      'Синхронизация сейчас не выполняется.';

  @override
  String get walletDeepScanTitle => 'Проверить старые адреса обмена';

  @override
  String get walletDeepScanBody =>
      'Если вы восстановили этот кошелёк и он раньше часто использовался для обмена, средства с самых старых обменов могут потребовать дополнительного шага для обнаружения. Эта проверка ищет их — всё найденное появится в вашем балансе по мере синхронизации кошелька.';

  @override
  String get walletDeepScanCoverage =>
      'Ваши старые адреса обмена проверены до этого места. Если средства от старого обмена всё ещё не найдены, проверьте ещё глубже.';

  @override
  String get walletDeepScanCoveragePending =>
      'Текущий диапазон всё ещё проверяется — всё найденное появится в вашем балансе. Это может занять некоторое время.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Проверяет наличие средств от самых старых обменов вашего кошелька.';

  @override
  String get walletDeepScanCheckButton => 'Проверить более старые адреса';

  @override
  String get walletDeepScanCheckDeeperButton =>
      'Проверить ещё более старые адреса';

  @override
  String get walletDeepScanChecking => 'Проверка…';

  @override
  String get walletDeepScanClose => 'Закрыть';

  @override
  String get walletDeepScanTorHint =>
      'Сейчас вы не подключены через Tor. Для большей приватности рекомендуем дождаться активации Tor перед проверкой.';

  @override
  String get walletDeepScanRescanBusy =>
      'Вы сможете проверить старые адреса обмена, как только пересканирование завершится.';

  @override
  String get walletDeepScanRan =>
      'Проверка старых адресов обмена — всё найденное появится в вашем балансе.';

  @override
  String get walletDeepScanFailed =>
      'Не удалось начать проверку. Ничего не изменилось — попробуйте снова.';

  @override
  String get walletDeepScanSlow =>
      'Это занимает больше времени, чем обычно. Если ваши старые адреса обмена были проверены, всё найденное появится в вашем балансе — проверьте снова чуть позже.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'Обмен сейчас отключён, поэтому выполнить это нельзя. Повторите попытку, когда обмен станет доступен.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Последний диапазон всё ещё проверяется — это может занять до пары дней, но обычно намного меньше. Это происходит автоматически — просто проверьте снова позже.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'Пока не удаётся подтвердить приватность вашего соединения. Для большей приватности рекомендуем дождаться активации Tor перед проверкой.';

  @override
  String get walletDeepScanBannerChecking =>
      'Старые адреса обмена всё ещё проверяются — всё найденное появится в вашем балансе.';

  @override
  String get walletRescanSwapPointer =>
      'Ищете средства от старого обмена? Пересканирование их не найдёт — вместо этого используйте «Проверить старые адреса обмена».';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Восстановили кошелёк, который использовался для обмена?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Если у этого кошелька была очень долгая история обменов, средства с самых старых обменов могут потребовать дополнительного шага для обнаружения. Большинству кошельков ничего не требуется.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Проверить сейчас';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Закрыть';

  @override
  String walletTorHostPath(String transport) {
    return 'Через приватный путь вашего приложения ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Через приватный путь вашего приложения ($transport); прокси может связать соединения между собой';
  }

  @override
  String get walletTorHostOtherTransport => 'приватный путь';

  @override
  String get walletTorHostDirect =>
      'Не приватно (прямое подключение вашего приложения)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Сохранённый сервер использует незашифрованный адрес, который приватный путь вашего приложения не может передавать. Используется $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Подробнее: $label';
  }

  @override
  String get walletSendPaste => 'Вставить';

  @override
  String get walletSendScanQr => 'Сканировать QR-код';

  @override
  String get walletSendRecipientGetsLabel => 'Получатель получит';

  @override
  String get walletSwapDepositCopyAmount => 'Копировать сумму';

  @override
  String get walletSwapDepositAmountCopied => 'Сумма скопирована';

  @override
  String get walletScanOpenSettings => 'Открыть настройки';

  @override
  String get walletScanOpenSettingsFailed => 'Не удалось открыть настройки.';

  @override
  String get walletSendLeaveTitle => 'Отправка продолжается';

  @override
  String get walletSendLeaveBody =>
      'Платёж продолжится, если вы уйдёте. Чем он закончился, вы увидите в истории.';

  @override
  String get walletSendLeaveStay => 'Остаться';

  @override
  String get walletSendLeaveConfirm => 'Уйти';

  @override
  String get walletSheetLeaveBody =>
      'Это продолжится, если вы уйдёте. Чем всё закончилось, вы увидите в истории.';

  @override
  String get walletLoadingLabel => 'Загрузка';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'Проверьте, прежде чем защищать снова';

  @override
  String get walletShieldUnknownBody =>
      'Не удалось подтвердить защиту средств. Проверьте раздел «Активность», прежде чем повторить попытку.';

  @override
  String get walletMoveUnknownTitle => 'Проверьте, прежде чем переводить снова';

  @override
  String get walletMoveUnknownBody =>
      'Не удалось подтвердить этот перевод. Проверьте раздел «Активность», прежде чем повторить попытку.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
