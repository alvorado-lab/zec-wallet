// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Ukrainian (`uk`).
class WalletLocalizationsUk extends WalletLocalizations {
  WalletLocalizationsUk([String locale = 'uk']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Налаштування';

  @override
  String get walletTitle => 'Гаманець';

  @override
  String get walletNotSetUpTitle => 'Гаманець ще не налаштовано';

  @override
  String get walletNotSetUpBody =>
      'Налаштування гаманця з\'явиться в наступній версії. Воно проведе вас через запис фрази відновлення ще до того, як гаманець зможе отримувати кошти, — тому без резервної копії нічого не буде під загрозою.';

  @override
  String get walletStartupFailedTitle => 'Не вдалося запустити гаманець';

  @override
  String get walletStartupFailedBody =>
      'Щось завадило гаманцю завантажитися на цьому пристрої. Якщо у вас уже є гаманець, його кошти не постраждали — вони перебувають у мережі Zcash і можуть бути відновлені за допомогою фрази відновлення. Спробуйте ще раз; якщо це повторюється, закрийте застосунок і відкрийте його знову.';

  @override
  String get walletBalanceLabel => 'Баланс';

  @override
  String get walletHideBalance => 'Приховати баланс';

  @override
  String get walletShowBalance => 'Показати баланс';

  @override
  String get walletBalanceHiddenAmount => 'Баланс приховано';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Доступно для витрати';

  @override
  String get walletArrivingLabel => 'Надходить';

  @override
  String get walletNotSpendableYetLabel => 'Поки не можна витратити';

  @override
  String get walletActivityTitle => 'Активність';

  @override
  String get walletActivityEmpty => 'Активності поки немає';

  @override
  String get walletActivityError => 'Не вдалося завантажити активність';

  @override
  String get walletActivityReceived => 'Отримано';

  @override
  String get walletActivitySent => 'Надіслано';

  @override
  String get walletActivityPending => 'В очікуванні';

  @override
  String get walletActivityQueued => 'У черзі';

  @override
  String get walletActivityRetrying => 'Повторне надсилання';

  @override
  String get walletActivitySaved => 'Збережено';

  @override
  String get walletActivityExpired => 'Прострочено';

  @override
  String get walletActivityFailed => 'Не виконано';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count підтвердження',
      many: '$count підтверджень',
      few: '$count підтвердження',
      one: '$count підтвердження',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count платежів отримано',
      many: '$count платежів отримано',
      few: '$count платежі отримано',
      one: 'Платіж отримано',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Показати деталі транзакції';

  @override
  String get walletTxDetailStatus => 'Статус';

  @override
  String get walletTxDetailFee => 'Комісія мережі';

  @override
  String get walletTxDetailDate => 'Дата';

  @override
  String get walletTxDetailHeight => 'Висота блока';

  @override
  String get walletTxDetailMemo => 'Примітка';

  @override
  String get walletTxDetailMemoAttached => 'Додано';

  @override
  String get walletTxDetailTxid => 'ID транзакції';

  @override
  String get walletTxDetailCopyTxid => 'Копіювати ID транзакції';

  @override
  String get walletTxDetailCopied => 'ID транзакції скопійовано';

  @override
  String get walletTxDetailClose => 'Закрити';

  @override
  String get walletTxFundsKept => 'Кошти не залишали ваш гаманець';

  @override
  String get walletTxExplainQueued =>
      'Збережено на цьому пристрої, в розділі «Збережено й очікує» — там його можна надіслати або скасувати.';

  @override
  String get walletTxExplainPending =>
      'Надіслано в мережу Zcash — очікує підтвердження в блоці.';

  @override
  String get walletTxExplainRetrying =>
      'Гаманець поки не зміг надіслати це в мережу Zcash. Він зберігає підписану транзакцію та повторює спробу під час кожної синхронізації, доки вона не пройде або не спливе.';

  @override
  String get walletTxExplainSaved =>
      'Гаманець зберіг цю підписану транзакцію, але наразі не надсилає її самостійно.';

  @override
  String get walletTxExplainConfirmed => 'Підтверджено в мережі Zcash.';

  @override
  String get walletTxExplainExpired =>
      'Термін дії цієї транзакції минув до її підтвердження мережею, тому її скасовано. Сума й далі доступна вам для витрати.';

  @override
  String get walletTxExplainFailed =>
      'Мережа відхилила цю транзакцію, тому вона не пройшла. Сума й далі доступна вам для витрати.';

  @override
  String get walletTxExplainUnknown =>
      'Поточний статус цієї транзакції визначити не вдалося. Він оновиться після наступної синхронізації.';

  @override
  String get walletMenuTooltip => 'Більше опцій';

  @override
  String get walletRescanMenuItem => 'Повторне сканування історії…';

  @override
  String get walletCheckOneTimeMenuItem => 'Перевірити одноразові адреси…';

  @override
  String get walletRescanTitle => 'Повторне сканування історії';

  @override
  String get walletRescanBody =>
      'Бракує старіших коштів? Скануйте блокчейн із глибшої точки, щоб знайти надходження, які пропустила попередня дата початку. Ваші кошти та фраза відновлення ніколи не піддаються ризику.';

  @override
  String get walletRescanRangeTitle => 'З якого моменту сканувати';

  @override
  String get walletRescanRangeAll =>
      'Сканувати всю історію — найповільніше, але знаходить усе.';

  @override
  String get walletRescanRangeDefault =>
      'Сканування від моменту створення вашого гаманця. Досі бракує старіших коштів? Виберіть ранішу дату або скануйте всю історію.';

  @override
  String get walletRescanRangeResolving =>
      'Підготовка рекомендованого діапазону…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Приблизно $blocks блоків для сканування.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Сканування від $date. Досі бракує старіших коштів? Виберіть ранішу дату або скануйте всю історію.';
  }

  @override
  String get walletRescanPick => 'Вибрати дату';

  @override
  String get walletRescanChange => 'Змінити дату';

  @override
  String get walletRescanScanAll => 'Сканувати всю історію';

  @override
  String get walletRescanDatePick => 'Найраніша дата для сканування';

  @override
  String get walletRescanWarning =>
      'Це повторно сканує блокчейн. Недавні дати займають хвилини; сканування далекого минулого може тривати години. Синхронізація триває у фоні — гаманцем можна користуватися далі.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Платіж із цього гаманця усе ще підтверджується. Гаманець зазвичай відмовляє у повторному скануванні, поки він не завершиться — ви можете спробувати, але очікуйте на відмову.';

  @override
  String get walletRescanConfirm => 'Почати повторне сканування';

  @override
  String get walletRescanCancel => 'Скасувати';

  @override
  String get walletRescanRunning => 'Відновлення…';

  @override
  String get walletRescanRebuildingAll =>
      'Відновлення історії — сканується весь ланцюг. Баланс і активність заповнюються в міру просування.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Відновлення історії від $date — баланс і активність заповнюються в міру просування.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Відновлення історії від моменту створення вашого гаманця — баланс і активність заповнюються в міру просування.';

  @override
  String get walletCatchUpBanner =>
      'Надолужування — баланс і активність заповнюються, поки гаманець синхронізується. Усе, що ви отримали, в безпеці.';

  @override
  String get walletCatchUpRescanBanner =>
      'Відновлення історії після повторного сканування — баланс і активність заповнюються в міру просування. Усе, що ви отримали, в безпеці.';

  @override
  String get walletRescanFailedNotice =>
      'Наразі не вдалося повторно сканувати — ваші кошти в безпеці, хоча балансу та історії може знадобитися трохи часу, щоб надолужити згаяне. Спробуйте ще раз за хвилину.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Платіж усе ще підтверджується, тому повторне сканування призупинено, щоб захистити ваші кошти. Гаманець не змінився — спробуйте ще раз за кілька годин і тримайте застосунок відкритим і в мережі.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'Повторне сканування відновлює вашу історію в міру синхронізації гаманця, а синхронізація зараз не триває. Гаманець не змінився — спробуйте ще раз, щойно синхронізація триватиме.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Недостатньо вільного місця для відновлення історії гаманця — ваші кошти в безпеці, хоча балансу та історії може знадобитися трохи часу, щоб надолужити згаяне. Звільніть трохи місця та спробуйте ще раз.';

  @override
  String get walletRescanFailedDismiss => 'Закрити';

  @override
  String get walletActivityRebuilding => 'Відновлення історії…';

  @override
  String get walletActivityCatchingUp =>
      'Надолужування триває — усе, що ви отримали, з’явиться тут.';

  @override
  String get walletActivitySyncNotRunning =>
      'Ваші баланс та історія завантажаться повністю, щойно синхронізація триватиме.';

  @override
  String get walletActivityLoadMore => 'Завантажити ще';

  @override
  String get walletPendingChangeLabel => 'Решта в очікуванні';

  @override
  String get walletTransparentLabel => 'Незахищені (публічні)';

  @override
  String get walletTransparentNote =>
      'Не входять до «Доступно для витрати» — захистіть ці кошти, щоб їх витратити. До того часу вони залишаються публічно видимими в блокчейні.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Ці кошти публічно видимі в блокчейні.';

  @override
  String walletPoolShielded(String amount) {
    return 'Захищено $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Публічно $amount';
  }

  @override
  String get walletPoolAllShielded => 'Все захищено · приватно';

  @override
  String get walletPoolTapHint => 'Показати публічні кошти';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount вашого балансу перебуває на одноразовій адресі (можна повернути).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount вашого балансу перебуває на одноразовій адресі.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Платежі на загальну суму $amount зарезервовані й досі завершуються через одноразові адреси під контролем вашого гаманця. Не надсилайте ці кошти повторно.',
      many:
          'Платежі на загальну суму $amount зарезервовані й досі завершуються через одноразові адреси під контролем вашого гаманця. Не надсилайте ці кошти повторно.',
      few:
          'Платежі на загальну суму $amount зарезервовані й досі завершуються через одноразові адреси під контролем вашого гаманця. Не надсилайте ці кошти повторно.',
      one:
          '$amount зарезервовано за платежем, який ваш гаманець ще завершує через одноразову адресу під його контролем. Не надсилайте ці кошти повторно.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Платежі на загальну суму $amount зарезервовані і завершуються на півдорозі через одноразові адреси під контролем вашого гаманця. Призупинено, поки ваш гаманець знову не синхронізується. Не надсилайте ці кошти повторно.',
      many:
          'Платежі на загальну суму $amount зарезервовані і завершуються на півдорозі через одноразові адреси під контролем вашого гаманця. Призупинено, поки ваш гаманець знову не синхронізується. Не надсилайте ці кошти повторно.',
      few:
          'Платежі на загальну суму $amount зарезервовані і завершуються на півдорозі через одноразові адреси під контролем вашого гаманця. Призупинено, поки ваш гаманець знову не синхронізується. Не надсилайте ці кошти повторно.',
      one:
          '$amount зарезервовано за платежем, який ваш гаманець завершує на півдорозі через одноразову адресу під його контролем. Призупинено, поки ваш гаманець знову не синхронізується. Не надсилайте ці кошти повторно.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Не вдалося перевірити, чи платіж ще завершується. Повторюємо спробу — а поки перевірте, чи немає платежу, що очікує, у вашій активності, перш ніж надсилати знову.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount вашого балансу перебуває на одноразовій адресі (ще підтверджується).';
  }

  @override
  String get walletShieldButton => 'Захистити';

  @override
  String get walletShieldSheetTitle => 'Захистити публічні кошти';

  @override
  String get walletShieldNote =>
      'Це переміщує кошти з публічного, видимого в блокчейні балансу у ваш приватний захищений баланс.';

  @override
  String get walletShieldPreparing => 'Підготовка…';

  @override
  String get walletShieldAmountLabel => 'Захищається';

  @override
  String get walletShieldFeeLabel => 'Комісія мережі';

  @override
  String get walletShieldNetLabel => 'Надійде захищеним';

  @override
  String get walletShieldConfirmButton => 'Захистити зараз';

  @override
  String get walletShieldSubmitting => 'Захищення…';

  @override
  String get walletShieldNothingTitle => 'Поки нема чого захищати';

  @override
  String get walletShieldNothingBody =>
      'Ці кошти зараз менші за суму, яку варто захищати, — комісія мережі переважить вигоду. Їх можна буде захистити, щойно надійде трохи більше.';

  @override
  String get walletShieldDoneTitle => 'Захист надіслано';

  @override
  String get walletShieldDoneBody =>
      'Ваші кошти переміщуються у захищений баланс. Незабаром це підтвердиться в блокчейні.';

  @override
  String get walletShieldSavedTitle => 'Збережено — ми завершимо захист';

  @override
  String get walletShieldSavedBody =>
      'Зараз не вдалося з\'єднатися з мережею. Ваш захист збережено, і гаманець завершить його під час однієї з пізніших синхронізацій. Нічого не втрачено.';

  @override
  String get walletShieldAlreadyTitle => 'Уже надіслано';

  @override
  String get walletShieldFailedTitle => 'Наразі не вдалося захистити';

  @override
  String get walletShieldStaleBody =>
      'Гаманець ще синхронізується. Спробуйте захистити ще раз за хвилину.';

  @override
  String get walletShieldTransientBody =>
      'Не вдалося підготувати екранування просто зараз. Спробуйте ще раз за хвилину.';

  @override
  String get walletShieldStorageFullBody =>
      'Недостатньо вільного місця для захисту зараз. Звільніть трохи місця та спробуйте ще раз. Ваші кошти в безпеці.';

  @override
  String get walletShieldClose => 'Закрити';

  @override
  String get walletShieldRetry => 'Спробувати ще раз';

  @override
  String get walletMoveMenuItem => 'Перемістити у публічні…';

  @override
  String get walletMoveSheetTitle => 'Перемістити у публічні кошти';

  @override
  String get walletMoveSheetSubtitle =>
      'Надішліть захищені ZEC на свою публічну адресу — це корисно, якщо біржа не приймає захищені депозити.';

  @override
  String get walletMoveDestinationLabel => 'Ваша публічна адреса';

  @override
  String walletMoveAvailable(String amount) {
    return 'Доступно для переміщення: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Доступно для переміщення: $amount ZEC — баланс ще надолужує згаяне';
  }

  @override
  String get walletMoveDeshieldTitle =>
      'Це переміщення робить ваші кошти публічними';

  @override
  String get walletMoveDeshieldBody =>
      'Переміщення на публічну адресу виводить ці кошти з вашого захищеного балансу — сума та ваша публічна адреса стають публічно видимими в блокчейні Zcash.';

  @override
  String get walletMoveWalletEnded =>
      'Сеанс гаманця завершився. Закрийте й відкрийте його знову, щоб спробувати ще раз.';

  @override
  String get walletMoveLoading => 'Підготовка…';

  @override
  String get walletMovePreparing => 'Перевірка суми…';

  @override
  String get walletMoveSubmitting => 'Переміщення…';

  @override
  String get walletMoveReviewButton => 'Перевірити';

  @override
  String get walletMoveCancel => 'Скасувати';

  @override
  String get walletMoveReviewTitle => 'Перевірка переміщення';

  @override
  String get walletMoveOwnAddressNote =>
      'Ви переміщуєте кошти на власну публічну адресу. Пізніше їх можна знову захистити, але цей запис назавжди залишиться в публічному реєстрі.';

  @override
  String get walletMoveConfirmButton => 'Перемістити у публічні';

  @override
  String get walletMoveBackButton => 'Назад';

  @override
  String get walletMoveDoneTitle => 'Переміщено у публічні';

  @override
  String get walletMoveDoneBody =>
      'Ваші кошти переміщуються на публічну адресу. Незабаром це підтвердиться в блокчейні.';

  @override
  String get walletMoveSavedTitle => 'Збережено — ми завершимо переміщення';

  @override
  String get walletMoveSavedBody =>
      'Це переміщення збережено, і гаманець надішле його під час однієї з пізніших синхронізацій. Нічого не втрачено.';

  @override
  String get walletMoveAlreadyTitle => 'Уже надіслано';

  @override
  String get walletMoveAlreadyBody =>
      'Ці кошти вже надіслано, вони прямують на вашу публічну адресу.';

  @override
  String get walletMoveFailedTitle => 'Не вдалося завершити переміщення';

  @override
  String get walletMoveNothingTitle => 'Поки нема чого переміщувати';

  @override
  String get walletMoveNothingBody =>
      'Наразі у вас немає доступного захищеного балансу для переміщення. Щойно кошти підтвердяться, ви зможете перемістити їх на публічну адресу.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Гаманець ще надолужує згаяне — усе, що ви отримали, стане доступним для переміщення після завершення синхронізації.';

  @override
  String get walletMoveCouldNotLoad =>
      'Не вдалося завантажити вашу публічну адресу. Спробуйте ще раз.';

  @override
  String get walletMoveRetry => 'Спробувати ще раз';

  @override
  String get walletMoveClose => 'Закрити';

  @override
  String get walletSnapshotUnavailable =>
      'Наразі не вдалося зчитати гаманець. Він оновиться самостійно.';

  @override
  String get walletBalanceStale =>
      'Не вдалося оновити — показано ваш останній відомий баланс.';

  @override
  String get walletSyncStartFailed =>
      'Не вдалося почати синхронізацію. Ми продовжимо спроби.';

  @override
  String get walletSyncRetry => 'Спробувати ще раз';

  @override
  String get walletSyncTryNow => 'Спробувати зараз';

  @override
  String get walletSyncIdle => 'Синхронізація ще не почалася';

  @override
  String get walletSyncIdleDetail => 'Синхронізація починається автоматично.';

  @override
  String get walletSyncDisabled => 'Синхронізація вимкнена';

  @override
  String get walletSyncDisabledDetail =>
      'Увімкніть синхронізацію в налаштуваннях цього застосунку, щоб оновити баланс.';

  @override
  String get walletSyncExplainDisabled =>
      'Синхронізацію вимкнено в налаштуваннях цього застосунку. Ваші кошти в безпеці. Баланс і активність показують останній синхронізований стан і не оновлюватимуться, доки синхронізацію не увімкнено.';

  @override
  String get walletParkedSyncPausedNote =>
      'Ваш гаманець не синхронізується, тому ці платежі не надішлються самі по собі. Скористайтеся кнопкою «Надіслати зараз», щоб надіслати платіж самостійно.';

  @override
  String get walletSyncPausedMoneyNote =>
      'Призупинено, поки ваш гаманець знову не синхронізується.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'З\'єднання…';

  @override
  String get walletSyncStartingDetail =>
      'Встановлюємо з\'єднання з мережею Zcash і готуємося до сканування.';

  @override
  String get walletSyncConnecting => 'З\'єднання…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'З\'єднання… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Сканування $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'Сканування…';

  @override
  String get walletSyncSpendableReady => 'Кошти готові до витрати.';

  @override
  String get walletSyncCatchingUp =>
      'Наздоганяємо мережу — глибока початкова синхронізація може тривати довше. Ви можете користуватися застосунком, поки вона триває';

  @override
  String walletSyncScanRemaining(String count) {
    return 'Залишилося блоків: $count';
  }

  @override
  String get walletSyncUpToDate => 'Синхронізовано';

  @override
  String get walletSyncOffline => 'Немає з\'єднання';

  @override
  String get walletSyncOfflineDetail =>
      'Надсилання в черзі залишаються збереженими в розділі «Збережено й очікує».';

  @override
  String get walletSyncUnknown => 'Синхронізація…';

  @override
  String get walletSyncStalled => 'Синхронізацію призупинено';

  @override
  String get walletStallEndpoint =>
      'Наразі не вдається з\'єднатися з мережею Zcash. Ми продовжимо спроби автоматично — перевірте своє інтернет-з\'єднання, або сервер може бути тимчасово недоступний.';

  @override
  String get walletStallTor =>
      'Приватний шлях вашого застосунку недоступний, тож гаманець не з\'єднується. Перевірте мережеві налаштування застосунку або вимкніть приватний шлях. Синхронізація відновиться, щойно шлях повернеться.';

  @override
  String get walletStallStorage =>
      'Пам\'ять пристрою заповнена. Звільніть трохи місця — синхронізація відновиться.';

  @override
  String get walletStallReorg =>
      'Ланцюг реорганізувався; перевіряємо останні блоки повторно.';

  @override
  String get walletStallInternal =>
      'Локальна проблема зупинила синхронізацію. Якщо це повторюється, відновіть гаманець із фрази відновлення.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Цей сервер надіслав дані, які не можуть бути правильними, тому синхронізацію зупинено. Це не проблема з\'єднання — перейдіть на інший сервер. Якщо кожен сервер відхиляється, виконайте повторне сканування історії: гаманець може зберігати хибний запис від попереднього сервера.';

  @override
  String get walletStallBirthdayInFuture =>
      'Цей гаманець налаштовано починати з блоку, якого цей сервер ще не досяг. Перевірте початковий блок, заданий для цього гаманця, або спробуйте інший сервер.';

  @override
  String get walletStallStorageUnavailable =>
      'Синхронізацію на цьому пристрої призупинено. Повторна спроба.';

  @override
  String get walletStallUnknown =>
      'Синхронізацію зупинено з невідомої причини.';

  @override
  String get walletSyncBadgeHint => 'Показати деталі синхронізації';

  @override
  String get walletSyncSheetClose => 'Закрити';

  @override
  String get walletSyncSheetProgress => 'Прогрес';

  @override
  String get walletSyncSheetBlocksLeft => 'Залишилося блоків';

  @override
  String get walletSyncSheetSyncedTo => 'Синхронізовано до блока';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Відстає щонайменше на $blocks блоку',
      many: 'Відстає щонайменше на $blocks блоків',
      few: 'Відстає щонайменше на $blocks блоки',
      one: 'Відстає щонайменше на $blocks блок',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'Синхронізація ще не почалася — вона почнеться автоматично. Дій не потрібно.';

  @override
  String get walletSyncExplainStartFailed =>
      'Синхронізація не змогла початися. Ваші кошти в безпеці — гаманець просто зараз не перевіряє нову активність. Спробуйте ще раз нижче або відкрийте застосунок знову.';

  @override
  String get walletSyncExplainStarting =>
      'Гаманець з\'єднується з мережею Zcash і готується до сканування. Зазвичай це триває кілька секунд.';

  @override
  String get walletSyncExplainConnecting =>
      'Встановлення з\'єднання з мережею Zcash.';

  @override
  String get walletSyncExplainScanning =>
      'Гаманець перевіряє блоки блокчейну на наявність ваших коштів. Баланс і активність оновлюються з появою нових транзакцій — ви можете користуватися застосунком, поки це триває.';

  @override
  String get walletSyncExplainUpToDate =>
      'Повністю синхронізовано з мережею Zcash. Баланс і активність актуальні.';

  @override
  String get walletSyncExplainStalled =>
      'Синхронізація зіткнулася з проблемою і призупинена. Спроби повторюються автоматично.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Не вдається з\'єднатися з мережею Zcash — це нормально, якщо ви офлайн, або сервер може бути тимчасово недоступний. Ваші кошти в безпеці: баланс показує останній синхронізований стан, а надсилання в черзі залишаються збереженими в розділі «Збережено й очікує». З\'єднання повторюється самостійно.';

  @override
  String get walletSyncExplainOffline =>
      'Немає мережевого з\'єднання. Ваші кошти в безпеці — баланс показує останній синхронізований стан, а надсилання в черзі залишаються збереженими в розділі «Збережено й очікує».';

  @override
  String get walletSyncExplainUnknown =>
      'Гаманець синхронізується. Баланс і активність оновлюються в міру просування.';

  @override
  String get walletTorOff => 'Tor вимкнено';

  @override
  String get walletTorBootstrapping => 'Приватний шлях запускається…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport запускається…';
  }

  @override
  String get walletTorActive => 'Tor активний';

  @override
  String get walletTorActiveUnverified =>
      'Tor активний (неперевірене середовище виконання)';

  @override
  String get walletTorActiveUnattested =>
      'Використовується приватний шлях (приватність не підтверджено)';

  @override
  String get walletTorFellBack =>
      'Tor недоступний — використовується пряме з\'єднання';

  @override
  String get walletTorUnavailable =>
      'Приватний шлях недоступний — з\'єднання немає';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport недоступний — з\'єднання немає';
  }

  @override
  String get walletTorUnanswered =>
      'Приватний шлях з\'єднано — нічого не повертається';

  @override
  String get walletTorUnansweredUnattested =>
      'Приватний шлях з\'єднано — нічого не повертається (приватність не підтверджено)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport з\'єднано — нічого не повертається';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Не приватно (пряме з\'єднання вашого застосунку) — нічого не повертається';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'З\'єднано через $transport — нічого не повертається; проксі може пов\'язати з\'єднання між собою';
  }

  @override
  String get walletTorUnknown =>
      'Статус Tor невідомий — вважайте з\'єднання незахищеним';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Баланс (станом на блок $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Баланс · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Баланс (станом на блок $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'З\'єднання';

  @override
  String get walletSyncSheetServer => 'Сервер';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Сервер, $host, відкриває вибір сервера';
  }

  @override
  String get walletSyncServerSheetTitle => 'Сервер синхронізації';

  @override
  String get walletSyncServerInUse => 'Використовується';

  @override
  String get walletSyncServerAppDefault => 'Типовий у застосунку';

  @override
  String get walletSyncServerCustom => 'Власний сервер…';

  @override
  String get walletSyncServerCustomHint => 'https://хост:порт';

  @override
  String get walletSyncServerCheck => 'Перевірити сервер';

  @override
  String get walletSyncServerChecking => 'Перевірка…';

  @override
  String get walletSyncServerUse => 'Використати цей сервер';

  @override
  String get walletSyncServerSwitching => 'Перемикання…';

  @override
  String get walletSyncServerContinue => 'Продовжити';

  @override
  String get walletSyncServerCancel => 'Скасувати';

  @override
  String get walletSyncServerTrustTitle => 'Довіряти цьому серверу?';

  @override
  String get walletSyncServerTrustNotice =>
      'Ви довіряєте цьому серверу повідомляти ваш баланс та історію і передавати ваші платежі. Він бачитиме вашу IP-адресу, якщо Tor не ввімкнено, приблизно коли створено гаманець, публічні адреси, які перевіряє гаманець, транзакції, які він запитує, і транзакції, які ви надсилаєте.';

  @override
  String get walletSyncServerKeyLabel => 'Ключ доступу (необов\'язково)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Заголовок ключа';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Введіть заголовок, який очікує ваш сервер';

  @override
  String get walletSyncServerKeyInvalid =>
      'Цей ключ або заголовок не можна використати';

  @override
  String get walletSyncServerKeySaved => 'Ключ збережено';

  @override
  String get walletSyncServerKeyShow => 'Показати';

  @override
  String get walletSyncServerKeyHide => 'Сховати';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Ваш ключ ідентифікує вас для цього сервера. Він може пов\'язати ваші платежі з вашим гаманцем, навіть через Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Перемикання перезапустить поточну синхронізацію. Баланс та історія збережуться. Кошти можуть відображатися як такі, що надходять, доки сканування нового сервера не наздожене.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Перемикання знову під\'єднається до нового сервера. Баланс та історія збережуться.';

  @override
  String get walletSyncServerUnreachable =>
      'Не вдалося зв\'язатися з цим сервером. Перевірте адресу — і якщо вона правильна, то або цей сервер не відповідає, або ваш застосунок зараз не може до нього дістатися. Спробуйте ще раз або виберіть інший сервер.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Не вдалося зв\'язатися з цим сервером. Гаманець не може визначити, чи цей сервер не відповідає, чи ваш застосунок зараз не може до нього дістатися. Виберіть інший сервер або спробуйте пізніше.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Цей сервер працює в іншій мережі Zcash.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Це не схоже на адресу сервера. Використовуйте https://хост:порт.';

  @override
  String get walletSyncServerNotOffered =>
      'Цей застосунок не пропонує цей сервер.';

  @override
  String get walletSyncServerBusy =>
      'Гаманець зараз зайнятий. Спробуйте трохи пізніше.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'Обраний сервер більше не пропонується цим застосунком. Використовується $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'Не вдалося прочитати збережений вибір сервера. Використовується $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Не вдалося перемкнутися — і далі використовується $host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Трафік гаманця з\'єднується із сервером напряму. Сервер бачить вашу IP-адресу.';

  @override
  String get walletTransportExplainTor =>
      'Трафік гаманця маршрутизується через мережу Tor, яка приховує вашу IP-адресу від сервера.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Приватний шлях вашого застосунку запускається. Трафік гаманця чекає на нього перед з\'єднанням.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport запускається. Трафік гаманця чекає на нього перед з\'єднанням.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Не вдалося з\'єднатися з Tor, тому трафік перейшов на пряме з\'єднання. Сервер бачить вашу IP-адресу.';

  @override
  String get walletTransportExplainUnavailable =>
      'Приватний шлях вашого застосунку недоступний, тож гаманець не з\'єднується. Вимкніть приватний шлях або перевірте мережеві налаштування застосунку.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport недоступний, тож гаманець не з\'єднується. Вимкніть його або перевірте мережеві налаштування застосунку.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'Приватний шлях прийняв з\'єднання, але вже хвилину ним нічого не повертається. Причиною може бути шлях або сервер гаманця — гаманець не може визначити, що саме. Він продовжує спроби; якщо це не мине, спробуйте інший сервер або перевірте мережеві налаштування застосунку.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport прийняв з\'єднання, але вже хвилину ним нічого не повертається. Причиною може бути шлях або сервер гаманця — гаманець не може визначити, що саме. Він продовжує спроби; якщо це не мине, спробуйте інший сервер або перевірте мережеві налаштування застосунку.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Трафік гаманця з\'єднується із сервером напряму. Сервер бачить вашу IP-адресу. З\'єднання прийнято, але вже хвилину ним нічого не повертається. Причиною може бути шлях або сервер гаманця — гаманець не може визначити, що саме. Він продовжує спроби; якщо це не мине, спробуйте інший сервер або перевірте мережеві налаштування застосунку.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'Конфіденційність цього з\'єднання перевірити не вдалося — вважайте його незахищеним. З\'єднання прийнято, але вже хвилину ним нічого не повертається. Причиною може бути шлях або сервер гаманця — гаманець не може визначити, що саме. Він продовжує спроби; якщо це не мине, спробуйте інший сервер або перевірте мережеві налаштування застосунку.';

  @override
  String get walletTransportExplainUnverified =>
      'Конфіденційність цього з\'єднання перевірити не вдалося — вважайте його незахищеним.';

  @override
  String get walletTransportExplainHostProxy =>
      'Трафік гаманця маршрутизується через приватний транспорт цього застосунку, який приховує вашу IP-адресу від сервера.';

  @override
  String get walletOnboardingWelcomeTitle => 'Налаштуйте гаманець';

  @override
  String get walletOnboardingWelcomeBody =>
      'Створіть новий гаманець, щоб отримувати й зберігати ZEC. Ми згенеруємо фразу відновлення та проведемо вас через її резервне копіювання ще до того, як зможуть надійти кошти, — тому без резервної копії нічого не буде під загрозою.';

  @override
  String get walletCreateButton => 'Створити новий гаманець';

  @override
  String get walletRestoreButton => 'Відновити з фрази відновлення';

  @override
  String get walletWatchOnlyButton =>
      'Спостерігати за гаманцем (лише перегляд)';

  @override
  String get walletWatchOnlyTitle => 'Спостереження за гаманцем';

  @override
  String get walletWatchOnlyBody =>
      'Вставте ключ перегляду, щоб спостерігати за гаманцем без його ключів витрачання. Ви побачите баланс і історію, але не зможете надсилати кошти. Оберіть приблизну дату початку гаманця, щоб ми знали, як далеко в минуле потрібно шукати.';

  @override
  String get walletWatchOnlyKeyLabel => 'Ключ перегляду';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => 'Сканувати QR-код ключа перегляду';

  @override
  String get walletWatchOnlyScanTitle => 'Сканування ключа перегляду';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Наведіть камеру на QR-код ключа перегляду.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Камера недоступна. Вставте ключ вручну замість цього.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Вставити замість цього';

  @override
  String get walletWatchOnlyScanHint =>
      'Або натисніть кнопку сканування, щоб прочитати QR-код ключа перегляду.';

  @override
  String get walletWatchOnlyScanFilled => 'Ключ перегляду відскановано.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Дата початку гаманця';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Сканування від $date — кошти, отримані раніше, не з\'являться. Старіший гаманець? Виберіть ранішу дату.';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'Оберіть дату початку гаманця';

  @override
  String get walletWatchOnlyBirthdayChange => 'Змінити дату';

  @override
  String get walletWatchOnlySubmit => 'Спостерігати за цим гаманцем';

  @override
  String get walletWatchOnlyBack => 'Назад';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Це не схоже на дійсний ключ перегляду. Перевірте його і спробуйте знову.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Цей ключ перегляду призначений для іншої мережі. Його не можна використати тут.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'На цьому пристрої вже є гаманець. Поверніться назад і відкрийте його.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Ця дата початку занадто пізня. Оберіть більш ранню дату.';

  @override
  String get walletRestoreTitle => 'Відновлення гаманця';

  @override
  String get walletRestoreBody =>
      'Введіть фразу відновлення, щоб відновити гаманець — введіть або вставте слова по порядку, розділені пробілами. Лише стандартні фрази: якщо ваш гаманець використовував додаткову парольну фразу («25-те слово»), цей застосунок поки не може її відновити — ви побачите порожній гаманець, а не помилку.';

  @override
  String get walletRestorePhraseHint => 'слово один  слово два  слово три  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count слова',
      many: '$count слів',
      few: '$count слова',
      one: '$count слово',
      zero: 'Ще немає слів',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'фрази відновлення містять 12, 15, 18, 21 або 24 слова';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count слова не є словами відновлення — виправте виділені',
      many: '$count слів не є словами відновлення — виправте виділені',
      few: '$count слова не є словами відновлення — виправте виділені',
      one: '$count слово не є словом відновлення — виправте виділене',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'слово $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'слово $index: не є словом відновлення';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Видалити слово $index';
  }

  @override
  String get walletRestoreSubmit => 'Відновити гаманець';

  @override
  String get walletRestoreBack => 'Назад';

  @override
  String get walletRestoreBirthdayTitle => 'З якого моменту сканувати';

  @override
  String get walletRestoreBirthdayNone =>
      'Ми проскануємо всю історію — повільніше, але нічого не буде пропущено.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Сканування від $date — кошти, отримані раніше, не з\'являться. Старіший гаманець? Виберіть ранішу дату або скануйте всю історію.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Вибрати дату';

  @override
  String get walletRestoreBirthdayChange => 'Змінити дату';

  @override
  String get walletRestoreBirthdayClear => 'Сканувати всю історію';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'Слово $index не є словом відновлення. Перевірте фразу на помилки та спробуйте ще раз.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Ця фраза відновлення недійсна. Перевірте слова та їх порядок і спробуйте ще раз.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Ця фраза не відповідає гаманцю на цьому пристрої. Перевірте її ще раз і спробуйте знову.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'На цьому пристрої вже є гаманець. Поверніться назад, щоб відкрити його.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Ця дата занадто нещодавня. Виберіть ранішу дату або скануйте все.';

  @override
  String get walletGeneratingLabel => 'Створення гаманця…';

  @override
  String get walletOpeningLabel => 'Відкриття гаманця…';

  @override
  String get walletBackupTitle => 'Створіть резервну копію фрази відновлення';

  @override
  String get walletBackupBody =>
      'Ці слова — ЄДИНИЙ спосіб відновити гаманець і кошти. Запишіть їх по порядку та зберігайте в безпечному й приватному місці. Ніколи не діліться ними й не зберігайте онлайн — будь-хто, хто знає ці слова, може заволодіти вашими коштами.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'На цьому екрані знімки екрана вимкнено.';

  @override
  String get walletBackupSecureNoteOther =>
      'Переконайтеся, що ніхто не може побачити ваш екран.';

  @override
  String get walletBackupReveal => 'Показати фразу відновлення';

  @override
  String get walletBackupRevealing => 'Підготовка фрази відновлення…';

  @override
  String get walletBackupRevealFailed =>
      'Наразі не вдалося показати фразу відновлення. Переконайтеся, що пристрій розблоковано, і спробуйте ще раз.';

  @override
  String get walletBackupRetryReveal => 'Спробувати ще раз';

  @override
  String get walletBackupReauthFailed =>
      'Не вдалося підтвердити, що це ви. Спробуйте ще раз.';

  @override
  String get walletBackupConfirmCheckbox =>
      'Фразу відновлення записано й надійно збережено.';

  @override
  String get walletBackupContinue => 'Продовжити';

  @override
  String get walletBackupSaveFailed =>
      'Не вдалося зберегти ваше підтвердження. Спробуйте ще раз.';

  @override
  String get walletBackupStartOver => 'Почати спочатку';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Почати спочатку без цього гаманця?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Це видалить цей гаманець із пристрою та поверне вас на початок. Поки налаштування не завершено, внести кошти через цей застосунок неможливо.\n\nЯкщо в цьому гаманці колись були кошти — або його було відновлено з фрази відновлення — відновити його зможе лише ця фраза.';

  @override
  String get walletBackupStartOverConfirm => 'Видалити й почати спочатку';

  @override
  String get walletBackupStartOverKeep => 'Зберегти цей гаманець';

  @override
  String get walletBackupSectionTitle => 'Фраза відновлення';

  @override
  String get walletBackupTileTitle =>
      'Створіть резервну копію фрази відновлення';

  @override
  String get walletBackupTileSubtitle =>
      'Покажіть слова, які можуть відновити ваш гаманець і кошти.';

  @override
  String get walletBackupScreenTitle => 'Фраза відновлення';

  @override
  String get walletBackupDone => 'Готово';

  @override
  String get walletBackupManagedTitle => 'Немає окремої фрази відновлення';

  @override
  String get walletBackupManagedBody =>
      'Цей гаманець налаштовано на основі облікового запису із застосунку, який його встановив, тому власної фрази відновлення в нього немає. Ваші кошти відновлюються разом із цим обліковим записом — використовуйте його резервну копію, щоб зберегти їх у безпеці.';

  @override
  String get walletExportViewingKeyTitle => 'Експорт ключа перегляду';

  @override
  String get walletExportViewingKeyTileTitle => 'Експорт ключа перегляду';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Поділіться копією гаманця лише для перегляду — вона може бачити вашу історію, але не може витрачати кошти.';

  @override
  String get walletExportViewingKeyWarning =>
      'Цей ключ дозволяє кожному, хто ним володіє, бачити все, що цей гаманець будь-коли отримував і надсилав, — а також усе, що він отримає та надішле в майбутньому. Він не може витрачати ваші кошти і не може відновити ваш гаманець. Діліться ним лише з тим, кому ви довіряєте бачити всю вашу історію, наприклад із бухгалтером або власним другим пристроєм. Єдиний спосіб згодом закрити доступ — перевести ваші кошти в новий гаманець.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Цей ключ дозволяє кожному, хто ним володіє, бачити все, що цей гаманець будь-коли отримував і надсилав, — а також усе, що він отримає та надішле в майбутньому. Він не може витрачати ваші кошти і не може відновити ваш гаманець. Діліться ним лише з тим, кому ви довіряєте бачити всю вашу історію, наприклад із бухгалтером або власним другим пристроєм. Поділившись ним, ви вже не зможете закрити цей доступ.';

  @override
  String get walletExportViewingKeyReveal => 'Показати ключ перегляду';

  @override
  String get walletExportViewingKeyRetry => 'Спробувати ще раз';

  @override
  String get walletExportViewingKeyRevealing =>
      'Підготовка вашого ключа перегляду…';

  @override
  String get walletExportViewingKeyFailed =>
      'Не вдалося показати ваш ключ перегляду зараз. Спробуйте ще раз за мить.';

  @override
  String get walletExportViewingKeyQrLabel => 'QR-код ключа перегляду';

  @override
  String get walletExportViewingKeyCopy => 'Копіювати ключ перегляду';

  @override
  String get walletExportViewingKeyCopied => 'Ключ перегляду скопійовано';

  @override
  String get walletExportViewingKeyDone => 'Готово';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'На цьому екрані знімки екрана вимкнено.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Переконайтеся, що ніхто не може побачити ваш екран.';

  @override
  String get walletWatchOnlySectionTitle =>
      'Про цей гаманець лише для перегляду';

  @override
  String get walletWatchOnlyAboutBody =>
      'Це гаманець лише для перегляду. Його налаштовано на основі ключа перегляду, тому він бачить ваш баланс та історію, але не має ключів витрачання — тут немає чого резервувати, і він не може надсилати кошти.';

  @override
  String get walletWatchOnlyBadge => 'Лише перегляд';

  @override
  String get walletOnboardingFailedTitle =>
      'Не вдалося завершити налаштування гаманця';

  @override
  String get walletOnboardingRetry => 'Спробувати ще раз';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'Захищене сховище вашого телефона не відповідає. Розблокуйте пристрій і спробуйте ще раз. Якщо це повторюється, перезавантажте телефон.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Цей гаманець відкрито в іншому вікні чи застосунку, або він ще завершує попередню операцію. Закрийте інше вікно, що його використовує, — або зачекайте хвилину, — і спробуйте ще раз.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'Захищений ключ цього гаманця більше недоступний, тому відкрити його на цьому пристрої неможливо. Ваші кошти в безпеці — відновіть їх за фразою відновлення.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Відновити з фрази відновлення';

  @override
  String get walletOnboardingRecoverConfirmTitle => 'Відновити цей гаманець?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Перш ніж продовжити, переконайтеся, що у вас є фраза відновлення — вона знадобиться на наступному екрані для відновлення коштів. Ваші кошти в безпеці в блокчейні та контролюються цією фразою. Ця дія видалить нечитабельні дані гаманця з цього пристрою, щоб його можна було відновити заново.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Скасувати';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Недостатньо вільного місця для налаштування гаманця. Звільніть трохи місця та спробуйте ще раз.';

  @override
  String get walletOnboardingFailedNoVault =>
      'На цьому пристрої немає захищеного сховища ключів, тому гаманець не може захистити тут вашу фразу відновлення.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Не вдалося з\'єднатися з мережею під час налаштування. Перевірте з\'єднання та спробуйте ще раз.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'Налаштування гаманця не завершилося. Спробуйте ще раз, щоб завершити його — нічого не втрачено.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Під час налаштування гаманця щось пішло не так. Спробуйте ще раз.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'Налаштування гаманця в цьому застосунку задане неправильно, тому гаманець не може запуститися. Повторна спроба не допоможе — повідомте про це розробника застосунку. Ваші кошти не постраждали.';

  @override
  String get walletSendButton => 'Надіслати';

  @override
  String get walletSendSyncNotRunning =>
      'Синхронізація не триває — ваш доступний для витрати баланс не зможе оновитися';

  @override
  String get walletSendWaitingForFunds =>
      'Синхронізація ще триває — ви зможете надсилати, щойно матимете доступний для витрати баланс';

  @override
  String get walletSendNoSpendableYet => 'Поки немає доступного балансу';

  @override
  String get walletSendSyncUnavailable =>
      'Ви зможете надсилати, щойно синхронізація відновиться';

  @override
  String get walletSendTitle => 'Надіслати';

  @override
  String get walletSendUnavailable =>
      'Ваш гаманець зараз не готовий. Поверніться назад і спробуйте ще раз.';

  @override
  String get walletSendWatchOnly =>
      'Це гаманець лише для перегляду. Він може показувати баланс і отримувати платежі, але не містить ключів витрачання — тому не може надсилати.';

  @override
  String get walletSendExpiredTitle =>
      'Термін дії цього запиту на оплату минув';

  @override
  String get walletSendExpiredBody =>
      'Екран надсилання відкривався довше п\'яти секунд, тому застосунку повідомили, що нічого не надіслано. Ця відповідь остаточна: сплатити цей запит звідси не можна. Щоб сплатити, почніть знову із застосунку.';

  @override
  String get walletSendFaultWatchOnly =>
      'Це гаманець лише для перегляду — він не містить ключів витрачання, тому не може надсилати.';

  @override
  String walletSendAvailable(String amount) {
    return 'Доступно для надсилання: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Доступно для надсилання: $amount ZEC — баланс ще надолужує згаяне';
  }

  @override
  String get walletSendRecipientLabel => 'Адреса отримувача';

  @override
  String get walletSendRecipientHint =>
      'Адреса Zcash (починається з u, z або t)';

  @override
  String get walletSendRecipientLocked => 'Отримувача не можна змінити тут';

  @override
  String get walletSendAmountLabel => 'Сума (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Примітка (необов\'язково)';

  @override
  String get walletSendMemoHint =>
      'Доставляється лише захищеним (приватним) отримувачам';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Для приміток потрібен захищений отримувач. Ця публічна адреса не може її отримати.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Цей платіж уже несе позначку застосунку, тому не може мати ще й написану нотатку.';

  @override
  String get walletSendMachineMemoTitle => 'Застосунок додає позначку';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Зазначено, що це для: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Вона залишиться з транзакцією, і її не можна буде вилучити. Гаманець не може перевірити її вміст.';

  @override
  String get walletSendRecipientShielded => 'Захищена · приватна';

  @override
  String get walletSendRecipientTransparent => 'Публічна';

  @override
  String get walletSendRecipientInvalid =>
      'Це не схоже на дійсну адресу Zcash.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Ця адреса призначена для іншої мережі Zcash.';

  @override
  String get walletSendReviewButton => 'Перевірити платіж';

  @override
  String get walletSendQueueButton => 'Додати в чергу на пізніше';

  @override
  String get walletSendQueueHint =>
      'Платіж у черзі очікує в розділі «Збережено й очікує», де його можна надіслати або скасувати. Комісію мережі визначають у момент надсилання.';

  @override
  String get walletSendPreparing => 'Підготовка платежу…';

  @override
  String get walletSendSubmitting => 'Надсилання…';

  @override
  String get walletSendQueuing => 'Додавання в чергу…';

  @override
  String get walletSendReviewTitle => 'Підтвердження платежу';

  @override
  String get walletSendTotalLabel => 'Разом';

  @override
  String get walletSendFeeLabel => 'Комісія мережі';

  @override
  String get walletSendChangeLabel => 'Повернена решта';

  @override
  String get walletSendDeshieldTitle => 'Цей платіж не є приватним';

  @override
  String get walletSendDeshieldBody =>
      'Він надсилається на публічну адресу, тому сума та отримувач будуть публічно видимі в блокчейні Zcash.';

  @override
  String get walletSendPublicAckLabel =>
      'Я розумію, що цей платіж буде публічним.';

  @override
  String get walletSendConfirmButton => 'Надіслати зараз';

  @override
  String get walletSendBackButton => 'Назад';

  @override
  String get walletSendSelfSendNote =>
      'Ви надсилаєте на власний гаманець. Комісія мережі все одно застосовується.';

  @override
  String get walletSendLargeConfirmTitle => 'Надіслати велику суму?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Це майже весь ваш баланс. Надісланий платіж не можна скасувати.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Це велика сума платежу. Надісланий платіж не можна скасувати.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Це велика сума платежу — майже весь ваш баланс. Надісланий платіж не можна скасувати.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Надіслати $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Повернутися';

  @override
  String get walletSendSentTitle => 'Платіж надіслано';

  @override
  String get walletSendSentBody => 'Ваш платіж транслюється в мережу.';

  @override
  String get walletSendSavedTitle => 'Збережено — ми завершимо надсилання';

  @override
  String get walletSendSavedBody =>
      'Ваш платіж зараз не вдалося надіслати, тому його збережено — гаманець надішле його під час однієї з пізніших синхронізацій. Нічого не втрачено.';

  @override
  String get walletSendKeptTitle => 'Збережено';

  @override
  String get walletSendKeptBody =>
      'Гаманець зберіг цю транзакцію, але не обіцяв надіслати її самостійно. Її стан можна переглянути в розділі «Активність».';

  @override
  String get walletSendPartialBody =>
      'Частину вашого платежу надіслано; гаманець завершить решту під час однієї з пізніших синхронізацій. Нічого не втрачено.';

  @override
  String get walletSendInMotionTitle => 'Платіж виконується';

  @override
  String get walletSendInMotionBody =>
      'Ваш платіж розпочато, і він рухається через одноразову адресу під контролем вашого гаманця. Не надсилайте його повторно. Якщо він не завершиться, ви зможете повернути кошти на екрані гаманця.';

  @override
  String get walletSendAlreadyTitle => 'Уже надіслано';

  @override
  String get walletSendAlreadyBody =>
      'Цей платіж уже надіслано — повторно він не надішлеться.';

  @override
  String get walletSendFailedTitle => 'Не вдалося виконати платіж';

  @override
  String get walletSendFailedBody =>
      'Під час виконання платежу щось пішло не так, нічого не надіслано. Можете спробувати ще раз.';

  @override
  String get walletSendTryAgain => 'Спробувати ще раз';

  @override
  String get walletSendDone => 'Готово';

  @override
  String get walletSendAnother => 'Надіслати ще один';

  @override
  String get walletSendQueuedTitle => 'Додано в чергу на надсилання';

  @override
  String get walletSendQueuedBody =>
      'Цей платіж збережено. Ви знайдете його в розділі «Збережено й очікує», де можна надіслати його зараз або скасувати.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Недостатньо доступного балансу — у вас $available ZEC, а потрібно $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Мережу Zcash було оновлено, і цьому застосунку потрібне оновлення, перш ніж він зможе надсилати. Ваші кошти в безпеці.';

  @override
  String get walletSyncUpToDateLimited =>
      'Синхронізовано настільки, наскільки ця версія може прочитати';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Мережу Zcash було оновлено. Ця версія просканувала все, що може прочитати, але новіші блоки можуть містити кошти, які вона поки не може показати, а нотатки до останніх платежів недоступні. Оновіть застосунок, щоб побачити все.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Синхронізовано, але цей сервер обслуговує не всі пули';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Цей сервер відмовляє, приховує або неправильно повідомляє дані одного із захищених пулів Zcash. Кошти, отримані в цьому пулі, не можна витратити через нього, а показаний баланс — це нижня межа. Перейдіть на інший сервер, щоб скористатися ними — це не проблема з\'єднання.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: цей сервер відмовляється його обслуговувати';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: цей сервер приховує його частину';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: цей сервер повідомляє про нього неправильні дані';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: невідомо, чи обслуговує його цей сервер';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Синхронізовано з цим сервером, але сервер відстає від мережі';

  @override
  String get walletSyncExplainEndpointBehind =>
      'Ланцюг цього сервера закінчується на блоці, який мережа пройшла ще до збирання цієї версії застосунку, тому баланс актуальний лише до цього блоку. Нові вхідні платежі можуть поки не відображатися, а платіж, надісланий звідси, може не дійти. Перейдіть на інший сервер, щоб наздогнати мережу — це не проблема з\'єднання.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Очікування оновлення застосунку — ваші кошти в безпеці, нічого не надіслано.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Очікування сервера, який повідомляє версію мережі — змініть сервер. Ваші кошти в безпеці, нічого не надіслано.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Очікування сервера, який повідомляє версію мережі. Якщо дата й час на цьому пристрої неправильні, спочатку виправте їх — а потім змініть сервер. Ваші кошти в безпеці, нічого не надіслано.';

  @override
  String get walletSyncUnverified =>
      'Синхронізовано, але цей сервер не повідомляє версію мережі';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'Надсилання ще працює близько $hours години — потім змініть сервер.',
      many: 'Надсилання ще працює близько $hours годин — потім змініть сервер.',
      few: 'Надсилання ще працює близько $hours годин — потім змініть сервер.',
      one: 'Надсилання ще працює близько $hours години — потім змініть сервер.',
      zero: 'Надсилання ще працює менше години — потім змініть сервер.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'Надсилання ще працює близько $blocks блоків — потім змініть сервер.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Цей сервер не повідомляє версію мережі вже $blocks блоків, тому застосунок не може підтвердити, що надсилати безпечно. Перейдіть на інший сервер.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Цей сервер не повідомляє версію мережі вже добу, тому застосунок не може підтвердити, що надсилати безпечно. Якщо дата й час на цьому пристрої неправильні, спочатку виправте їх — а потім перейдіть на сервер, який повідомляє версію мережі.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Цей сервер жодного разу не повідомив версію мережі, тому застосунок не може підтвердити, що надсилати безпечно. Перейдіть на інший сервер.';

  @override
  String get walletSyncExplainUnverified =>
      'Цей сервер не каже, на якій версії мережі Zcash він працює, тому застосунок не може підтвердити, що підписаний ним платіж буде прийнято. Ваш баланс актуальний. Перейдіть на інший сервер — це не проблема з\'єднання.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Цей сервер не каже, на якій версії мережі Zcash він працює, тому застосунок не може підтвердити, що підписаний ним платіж буде прийнято. Він також продовжував віддавати блоки, які цьому гаманцю потім довелося скасувати, тому ваш баланс може бути неактуальним. Перейдіть на інший сервер — це не проблема з\'єднання.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Цей сервер також продовжує віддавати блоки, які гаманцю потім доводиться скасовувати — змініть сервер.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Баланс ще надолужує згаяне — може стати доступно більше, поки гаманець синхронізується.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC ще надходить і стане доступним для витрати, коли гаманець наздожене мережу.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Введіть суму для надсилання.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Введіть суму у вигляді числа, наприклад 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'ZEC має щонайбільше 8 десяткових знаків.';

  @override
  String get walletSendFaultAmountNotPositive => 'Введіть суму більшу за нуль.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Ця сума перевищує загальний обсяг ZEC.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Зараз цей застосунок обмежує надсилання сумою $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Це не схоже на дійсну адресу Zcash для цієї мережі. Перевірте її та спробуйте ще раз.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Цей отримувач не може отримати примітку. Видаліть примітку або надішліть на захищену (приватну) адресу.';

  @override
  String get walletSendFaultMemoTooLong =>
      'Ваша примітка задовга. Скоротіть її та спробуйте ще раз.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Цю примітку не можна надіслати. Видаліть її та спробуйте ще раз.';

  @override
  String get walletSendFaultMemoConflict =>
      'Не вдалося надіслати цей платіж — застосунок прикріпив до нього дві нотатки. Нічого не надіслано.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Ця адреса призначена для іншої мережі.';

  @override
  String get walletSendFaultUriInvalid =>
      'Не вдалося сформувати цей платіж. Перевірте адресу та суму.';

  @override
  String get walletSendFaultNotSynced =>
      'Ваш гаманець ще недостатньо синхронізований. Зачекайте, поки синхронізація наздожене, або додайте платіж у чергу на пізніше.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Ваш гаманець ще недостатньо синхронізований. Зачекайте, поки синхронізація наздожене.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Ваш гаманець ще недостатньо синхронізований, а синхронізація зараз не триває. Перевірте стан синхронізації на екрані гаманця.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Суми застаріли, поки ви переглядали платіж. Перевірте платіж ще раз.';

  @override
  String get walletSendFaultQueueFull =>
      'Забагато надсилань очікують у черзі. Дочекайтеся їх надсилання, тоді спробуйте ще раз.';

  @override
  String get walletSendFaultWalletBusy =>
      'Гаманець зараз зайнятий. Спробуйте ще раз за хвилину.';

  @override
  String get walletSendFaultStorageFull =>
      'Недостатньо вільного місця для завершення цього надсилання. Звільніть трохи місця та спробуйте ще раз.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Зараз використовується забагато одноразових адрес. Деякі з них можуть звільнитися, коли підтвердяться перекази, але ситуація може не вирішитися сама собою. Ваші кошти в безпеці.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Не вдалося підготувати цей платіж. Перевірте дані та спробуйте ще раз.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Не вдалося підготувати цей платіж просто зараз. Спробуйте ще раз за хвилину.';

  @override
  String get walletSwapButton => 'Обмін';

  @override
  String get walletSwapTitle => 'Обмін ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'Ваш гаманець зараз не готовий. Поверніться назад і спробуйте ще раз.';

  @override
  String get walletSwapUnavailableOff => 'Обмін зараз недоступний.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Це гаманець лише для перегляду — він не може обмінювати.';

  @override
  String get walletSwapDone => 'Готово';

  @override
  String get walletSwapBackToWallet => 'Назад до гаманця';

  @override
  String walletSwapAvailable(String amount) {
    return 'Доступно для обміну: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Доступно для обміну: $amount ZEC — баланс ще надолужує згаяне';
  }

  @override
  String get walletSwapAssetLabel => 'Актив для отримання';

  @override
  String get walletSwapAmountLabel => 'Сума для обміну (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Адреса призначення';

  @override
  String get walletSwapDestinationHint =>
      'Ваша адреса отримання в мережі призначення';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Ваша адреса отримання в мережі $chain';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Адреса $chain — куди надсилається обміняний актив. Ретельно перевірте правильність мережі.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Сканувати QR-код адреси призначення';

  @override
  String get walletSwapTargetAssetHint => 'Виберіть актив для отримання';

  @override
  String get walletSwapQuoteButton => 'Отримати курс';

  @override
  String get walletSwapQuoting => 'Отримання курсу…';

  @override
  String get walletSwapExecuting => 'Запуск обміну…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Ще триває — обмін запускається. Це може зайняти до хвилини.';

  @override
  String get walletSwapReviewTitle => 'Підтвердження обміну';

  @override
  String get walletSwapYouSendLabel => 'Ви надсилаєте';

  @override
  String get walletSwapYouReceiveLabel => 'Ви отримаєте щонайменше';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Комісія мережі';

  @override
  String get walletSwapNetworkFeeValue =>
      'Додається під час надсилання депозиту';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Курс дійсний ще приблизно $time — підтвердьте до закінчення терміну.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Курс дійсний ще менше хвилини — підтвердьте до закінчення терміну.';

  @override
  String get walletSwapQuoteExpired =>
      'Термін дії цього курсу минув. Поверніться й отримайте новий — попередній курс більше не гарантований, і надсилання зараз може призвести до повернення коштів.';

  @override
  String get walletCountdownUnderMinute => 'менше хвилини';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes хв';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds с';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '$hours год $minutes хв';
  }

  @override
  String get walletSwapDeshieldTitle => 'Цей обмін не є приватним';

  @override
  String get walletSwapDeshieldBody =>
      'Обмін ZEC на інший актив знімає захист із ваших ZEC — депозит є публічною транзакцією, а сторона постачальника публічна в його мережі.';

  @override
  String get walletSwapDiscloseTitle => 'Що побачить постачальник обміну';

  @override
  String get walletSwapDiscloseAmounts => 'Суми з обох сторін';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Що ці ZEC і актив, який ви отримуєте, — один обмін';

  @override
  String get walletSwapDiscloseDestination => 'Вашу адресу призначення';

  @override
  String get walletSwapDiscloseSource => 'Вашу адресу джерела';

  @override
  String get walletSwapDiscloseIp =>
      'Вашу IP-адресу (якщо не використовуєте Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'Інші деталі цього обміну';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'Власні транзакції постачальника публічні в його мережі';

  @override
  String get walletSwapAckLabel =>
      'Я розумію, що постачальник побачить наведену вище інформацію.';

  @override
  String get walletSwapConfirmButton => 'Почати обмін';

  @override
  String get walletSwapBackButton => 'Назад';

  @override
  String get walletSwapStatusPendingTitle => 'Обмін розпочато';

  @override
  String get walletSwapStatusCheckingTitle => 'Перевірка статусу обміну…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Ваш гаманець надсилає депозит ZEC постачальнику. Якщо ви ненадовго опинитеся офлайн, кошти буде надіслано автоматично, щойно з\'явиться з\'єднання — але вікно для надсилання коротке, і якщо воно закриється раніше, обмін просто завершується, і нічого не обмінюється. Ваш ZEC залишається вашим, але може знадобитися до години, перш ніж він знову стане доступним для витрати.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'Очікуємо надходження вашого депозиту. Якщо ви ще не надіслали кошти з іншого гаманця, зробіть це до завершення терміну дії курсу.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Цей обмін досі очікує на депозит. Інструкції щодо депозиту більше не доступні на цьому пристрої — якщо ви вже надіслали кошти, їх буде виявлено; якщо ні, дайте цьому обміну завершитися і почніть новий.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'Вікно депозиту закривається $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'Вікно депозиту минуло. Якщо депозит не було надіслано вчасно, обмін завершиться, а ваш ZEC залишиться у вашому гаманці.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'Вікно депозиту минуло. Якщо ви ще не надіслали депозит, цей обмін просто завершується — отримайте новий курс, коли будете готові.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Обміни виконуються',
      many: 'Обміни виконуються',
      few: 'Обміни виконуються',
      one: 'Обмін виконується',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'Ваш ZEC уже прямує до постачальника.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'Очікуємо, доки ваш депозит надійде постачальнику.';

  @override
  String get walletSwapInFlightRowGeneric => 'Обмін виконується.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'Вікно депозиту минуло — перевірте статус цього обміну.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Цей обмін ще не досяг підтвердженого результату тут — відкрийте його, щоб перевірити. Будь-який ZEC, що повертається до цього гаманця, з\'явиться у вашому балансі після синхронізації.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Цей обмін ще не досяг підтвердженого результату тут — відкрийте його, щоб перевірити. Будь-який ZEC, що цей обмін доставляє до цього гаманця, з\'явиться у вашому балансі після синхронізації.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Обмін завершено.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Обмін повернено.';

  @override
  String get walletSwapRowOutcomeFailed => 'Обмін не завершено.';

  @override
  String get walletSwapRemove => 'Видалити';

  @override
  String get walletSwapRemoveTitle => 'Видалити цей обмін зі списку?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Це лише видалить обмін із цього списку — сам обмін не буде скасовано, і цей гаманець припинить відстежувати повернення коштів за ним. ZEC, повернений пізніше, усе одно належить цьому гаманцю; повне повторне сканування може його знайти.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Це лише видалить обмін із цього списку — сам обмін не буде скасовано, і цей гаманець припинить відстежувати вхідний ZEC. ZEC, доставлений пізніше, усе одно належить цьому гаманцю; повне повторне сканування може його знайти. Якщо натомість обмін буде повернено, повернення надійде в активі, який ви надіслали, поза цим гаманцем.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Це лише видалить обмін із цього списку — сам обмін не буде скасовано, і цей гаманець припинить відстежувати ZEC, що ще надходить від нього. ZEC, що надійде пізніше, усе одно належить цьому гаманцю; повне повторне сканування може його знайти.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Це видалить завершений обмін зі списку.';

  @override
  String get walletSwapRemoveCancel => 'Скасувати';

  @override
  String get walletSwapRemoveConfirm => 'Видалити';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Розпочато $time';
  }

  @override
  String get walletSwapViewSwap => 'Показати обмін';

  @override
  String get walletSwapsInFlightError =>
      'Наразі не вдалося завантажити обміни, що тривають.';

  @override
  String get walletSwapsInFlightRetry => 'Спробувати ще раз';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Спроба…';

  @override
  String get walletSwapStartAnother => 'Почати ще один обмін';

  @override
  String get walletSwapStatusUnderTitle => 'Очікування повного депозиту';

  @override
  String get walletSwapStatusUnderBody =>
      'Частина депозиту надійшла. Решта завершується, або постачальник поверне кошти.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Частина вашого депозиту надійшла. Надішліть суму, якої бракує, до завершення терміну, інакше постачальник поверне те, що надійшло.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Отримано $received; ще бракує $missing. Вікно депозиту закривається: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Депозит отримано';

  @override
  String get walletSwapStatusDetectedBody =>
      'Постачальник отримав ваш депозит і обробить обмін.';

  @override
  String get walletSwapStatusProcessingTitle => 'Обробка обміну';

  @override
  String get walletSwapStatusProcessingBody =>
      'Постачальник завершує ваш обмін.';

  @override
  String get walletSwapStatusSuccessTitle => 'Обмін завершено';

  @override
  String get walletSwapStatusSuccessBody => 'Ваш обмін успішно завершився.';

  @override
  String get walletSwapStatusRefundedTitle => 'Кошти обміну повернено';

  @override
  String get walletSwapStatusRefundedBody =>
      'Обмін не завершився, тому постачальник надіслав кошти назад на вашу адресу повернення.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'Обмін не завершився, тому постачальник надіслав ваш ZEC назад у цей гаманець. Кошти надійдуть як незахищені та з\'являться у вашому балансі після наступної синхронізації гаманця — це може зайняти певний час.';

  @override
  String get walletSwapStatusFailedTitle => 'Обмін не вдався';

  @override
  String get walletSwapStatusFailedBody =>
      'Обмін не вдалося завершити. Будь-які внесені кошти буде зараховано або повернено на стороні постачальника.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Обмін не знайдено';

  @override
  String get walletSwapStatusNotFoundBody =>
      'У постачальника більше немає запису про цей обмін — найімовірніше, його термін минув. Якщо депозит було внесено, постачальник має повернути його на адресу повернення. Обмін залишається у вашому списку, і цей гаманець продовжує відстежувати його ZEC на випадок, якщо він все ж надійде; ви можете видалити його зі списку будь-коли.';

  @override
  String get walletSwapStatusUnknownTitle => 'Статус недоступний';

  @override
  String get walletSwapStatusUnknownBody =>
      'Наразі не вдається зчитати статус цього обміну.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'Відстеження недоступне';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'Обмін вимкнено, тому відстежити це тут неможливо. Будь-які кошти буде зараховано або повернено на стороні постачальника.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'Тут обмін вимкнено, тому відстежити цей обмін зараз неможливо. Якщо кошти було повернено, ZEC повертається до цього гаманця — він з\'явиться у вашому балансі після того, як обмін знову увімкнуть і гаманець синхронізується.';

  @override
  String get walletSwapTrackingError => 'Не вдалося відстежити цей обмін.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Не вдалося відкрити відстеження цього обміну. Сам обмін, можливо, все ще триває — будь-які внесені кошти буде зараховано або повернено на стороні постачальника.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Введіть адресу, на яку хочете отримати обміняний актив.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Ця адреса призначення недійсна для цього активу. Перевірте її та спробуйте ще раз.';

  @override
  String get walletSwapFaultExpired =>
      'Термін дії цього курсу минув. Отримайте новий курс, щоб продовжити.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'Ціна постачальника вийшла за межі вашого ліміту, тому обмін зупинено до переміщення будь-яких коштів. Спробуйте ще раз.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'Ліміт прослизання зависокий для безпечного обміну. Спробуйте ще раз.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'Постачальник обміну зараз недоступний. Спробуйте ще раз за хвилину.';

  @override
  String get walletSwapFaultConnection =>
      'Не вдалося з\'єднатися зі службою обміну. Перевірте інтернет-з\'єднання та спробуйте ще раз.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'Постачальник обміну повернув неочікувану відповідь, тому обмін зупинено. Спробуйте ще раз.';

  @override
  String get walletSwapFaultSwapOff => 'Обмін зараз вимкнено.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Не вдалося надіслати ваш депозит, тому з гаманця нічого не списано. Отримайте новий курс, щоб спробувати ще раз.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Обмін уже виконується. Почати новий можна буде лише після того, як поточний остаточно завершиться й підтвердиться в мережі, або мине термін дії його курсу — це може зайняти певний час.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Цей гаманець поки не може налаштувати адресу повернення — зазвичай це означає, що перша синхронізація ще не завершилася. Зачекайте на завершення синхронізації, а потім спробуйте ще раз.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Цей гаманець поки не може налаштувати адресу отримання для цього обміну — зазвичай це означає, що перша синхронізація ще не завершилася. Зачекайте на завершення синхронізації, а потім спробуйте ще раз.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'Обмін не вдалося запустити вчасно — можливо, з\'єднання було повільним, або гаманець був зайнятий. Отримайте новий курс і спробуйте ще раз.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'Гаманець на мить зайнятий. Спробуйте ще раз.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Цей курс не збігається з тим, який видав ваш гаманець, тому нічого не надіслано. Отримайте новий курс і спробуйте ще раз.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Для цього обміну потрібно приблизно $needed ZEC, включно з комісією мережі, але наразі доступно для витрати лише $spendable ZEC.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Зараз цей застосунок обмежує обмін сумою $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Для цього обміну потрібно приблизно $needed ZEC, включно з комісією мережі, але наразі доступно для витрати лише $spendable ZEC. Ваш баланс ще надолужує згаяне — незабаром може стати доступно більше.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'Гаманець не зміг безпечно зафіксувати цей обмін, тому нічого не переміщено. Спробуйте ще раз.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Цей запит на обмін не вдалося обробити. Отримайте новий курс і спробуйте ще раз.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Не вдалося отримати курс обміну. Перевірте дані та спробуйте ще раз.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Ваш гаманець зараз не готовий. Поверніться назад і спробуйте ще раз.';

  @override
  String get walletSwapDirectionBuy => 'Купити ZEC';

  @override
  String get walletSwapDirectionSell => 'Продати ZEC';

  @override
  String get walletSwapRefundLabel => 'Ваша адреса повернення';

  @override
  String get walletSwapRefundHint =>
      'Куди повернуться монети, якщо обмін не вдасться';

  @override
  String get walletSwapRefundHelper =>
      'У мережі, з якої ви надсилаєте, — не адреса Zcash.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Ваша адреса повернення в мережі $chain';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Адреса $chain — куди повернуться монети, якщо обмін не вдасться. Не адреса Zcash.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'Про вашу адресу повернення';

  @override
  String get walletSwapRefundInfoBody =>
      'Якщо обмін не вдасться завершити, постачальник надішле ваші монети назад на цю адресу в мережі, з якої ви платили. Вводьте адресу, яку контролюєте самі, — гаманець не може перевірити сторонню адресу за вас, тож перевірте її уважно.';

  @override
  String get walletSwapRefundScanTooltip =>
      'Сканувати QR-код адреси повернення';

  @override
  String get walletSwapScanTitle => 'Сканування адреси';

  @override
  String get walletSwapScanInstruction => 'Наведіть камеру на QR-код адреси.';

  @override
  String get walletSwapScanManualEntry => 'Ввести вручну';

  @override
  String get walletSwapScanCancel => 'Скасувати';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Камера недоступна. Введіть адресу вручну нижче.';

  @override
  String get walletSwapSourceAssetLabel => 'Актив для обміну';

  @override
  String get walletSwapSourceAssetHint => 'Виберіть актив';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Сума для надсилання ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Сума для надсилання';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol у мережі $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Виберіть актив для обміну';

  @override
  String get walletSwapPickerTitleReceive => 'Виберіть актив для отримання';

  @override
  String get walletSwapPickerStale =>
      'Не вдалося оновити список активів — показано останній відомий список.';

  @override
  String get walletSwapPickerEmpty =>
      'Наразі немає доступних активів для обміну. Спробуйте пізніше.';

  @override
  String get walletSwapPickerSearchHint => 'Пошук за назвою або мережею';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Немає активів, що відповідають запиту «$query».';
  }

  @override
  String get walletSwapPickerError =>
      'Не вдалося завантажити список активів. Перевірте з\'єднання та спробуйте ще раз.';

  @override
  String get walletSwapPickerRetry => 'Спробувати ще раз';

  @override
  String get walletSwapSlippageLabel => 'Допустиме прослизання';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Інше';

  @override
  String get walletSwapSlippageCustomLabel => 'Власне прослизання';

  @override
  String get walletSwapSlippageMayFail =>
      'Дуже низьке — обмін може не вдатися, якщо ціна зміниться.';

  @override
  String get walletSwapSlippageNormal => 'Безпечний рівень.';

  @override
  String get walletSwapSlippageRisky =>
      'Високе — ви можете отримати помітно менше за вказаний курс.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Занадто високе — обмін буде відхилено. Знизьте до 10% або менше.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Ви отримаєте щонайменше $zec ZEC — це ваш мінімум із урахуванням прослизання $slippage%. Кінцева сума не опуститься нижче цього значення.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'Ви отримуєте ZEC на власну адресу';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Доки ви не захистите ці кошти — одним дотиком, про що вам нагадають після надходження — отримана сума буде тимчасово публічною та видимою в блокчейні. Невелике надходження може лишатися публічним, доки не накопичиться достатня сума.';

  @override
  String get walletSwapRefundVerifyTitle => 'Перевірте адресу повернення';

  @override
  String get walletSwapRefundVerifyBody =>
      'Перевірте її посимвольно — саме сюди повернуться ваші монети, якщо обмін не вдасться. Гаманець не може перевірити сторонню адресу за вас.';

  @override
  String get walletSwapRefundVerifyAck =>
      'Адресу повернення перевірено — вона правильна.';

  @override
  String get walletSwapPayoutVerifyTitle => 'Перевірте адресу отримання';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Перевірте її посимвольно — саме на цю адресу ви отримаєте $asset. Гаманець не може перевірити сторонню адресу за вас.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'Адресу отримання перевірено — вона правильна.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'Тут обмін вимкнено. Будь-які ZEC, що вже в дорозі, з\'являться у вашому гаманці після наступної синхронізації.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Введіть суму, яку хочете обміняти.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Введіть адресу повернення в мережі джерела.';

  @override
  String get walletSwapDepositTitle => 'Надішліть платіж';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Надішліть рівно $amount $asset у мережі $chain на адресу нижче.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Надішліть точну суму. Якщо надіслати менше або після закінчення вікна, постачальник поверне кошти на вашу адресу повернення.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Вікно депозиту: залишилося $time';
  }

  @override
  String get walletSwapDepositExpired =>
      'Це вікно депозиту закрилося. Не надсилайте кошти зараз — почніть новий обмін. Якщо ви вже надіслали, постачальник має повернути кошти на вашу адресу повернення.';

  @override
  String get walletSwapDepositQrLabel => 'QR-код адреси депозиту';

  @override
  String get walletSwapDepositAddressLabel => 'Адреса депозиту';

  @override
  String get walletSwapDepositCopy => 'Копіювати адресу депозиту';

  @override
  String get walletSwapDepositCopied => 'Адресу депозиту скопійовано';

  @override
  String get walletSwapDepositMemoRequired =>
      'Цей депозит потребує примітки / тега';

  @override
  String get walletSwapDepositMemoWarning =>
      'Ви ОБОВ\'ЯЗКОВО маєте додати саме цю примітку до депозиту. Надсилання без неї — або з неправильною приміткою — може призвести до безповоротної втрати коштів.';

  @override
  String get walletSwapDepositMemoLabel => 'Примітка / тег депозиту';

  @override
  String get walletSwapDepositMemoCopy => 'Копіювати примітку';

  @override
  String get walletSwapDepositMemoCopied => 'Примітку скопійовано';

  @override
  String get walletSwapDepositSent => 'Кошти надіслано';

  @override
  String get walletSwapDepositBackTitle => 'Покинути цей екран?';

  @override
  String get walletSwapDepositBackBody =>
      'Це не скасує ваш обмін — він продовжиться у фоні. Але для оплати вам знадобиться адреса депозиту, тож спершу скопіюйте її, якщо ще не зробили цього.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Це не скасує ваш обмін — він продовжиться у фоні. Вікно депозиту закрилося. Не надсилайте кошти на адресу депозиту зараз. Якщо ви вже надіслали, постачальник має повернути кошти на вашу адресу повернення.';

  @override
  String get walletSwapDepositBackStay => 'Залишитися';

  @override
  String get walletSwapDepositBackLeave => 'Покинути';

  @override
  String get walletReceive => 'Отримати';

  @override
  String get walletReceiveSubtitle =>
      'Поділіться цією адресою, щоб отримати ZEC. Її безпечно поширювати публічно.';

  @override
  String get walletReceiveCopy => 'Копіювати адресу';

  @override
  String get walletReceiveCopied => 'Адресу скопійовано';

  @override
  String get walletReceiveUnavailable => 'Ваш гаманець ще не готовий.';

  @override
  String get walletReceiveError =>
      'Не вдалося завантажити вашу адресу. Спробуйте ще раз.';

  @override
  String get walletReceivePreparing => 'Підготовка адреси…';

  @override
  String get walletReceivePreparingHint =>
      'Ваш гаманець готує цю адресу на вашому пристрої — це може зайняти трохи часу, якщо гаманець зайнятий іншою роботою.';

  @override
  String get walletReceiveRetry => 'Спробувати ще раз';

  @override
  String get walletReceiveQrLabel => 'QR-код вашої адреси отримання';

  @override
  String get walletReceiveTypeShielded => 'Захищена';

  @override
  String get walletReceiveTypeTransparent => 'Публічна';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Поділіться цією публічною адресою, щоб отримати ZEC від відправника, який не може платити на захищену адресу.';

  @override
  String get walletReceiveTransparentWarning =>
      'Це публічна адреса: вона видима в блокчейні та пов\'язує ваші платежі в разі повторного використання. Надавайте перевагу захищеній адресі; захистіть ці кошти після отримання.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'QR-код вашої публічної адреси отримання';

  @override
  String get walletReceiveFreshAddress => 'Використати нову адресу';

  @override
  String get walletReceiveFreshCaption =>
      'Нова адреса — її не можна пов\'язати з вашими іншими адресами. Платежі на неї все одно надходять до цього гаманця, а ваші попередні адреси продовжують працювати. Вона більше не буде показана тут — скопіюйте її зараз.';

  @override
  String get walletReceiveFreshError =>
      'Не вдалося створити нову адресу. Спробуйте ще раз.';

  @override
  String get walletReceiveFreshBusy =>
      'Гаманець зараз зайнятий. Спробуйте ще раз із новою адресою за мить.';

  @override
  String get walletReceiveShare => 'Поділитися';

  @override
  String get walletReceiveRequestAmount => 'Запросити суму';

  @override
  String get walletReceiveRequestAmountLabel => 'Сума (необов\'язково)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Вона більше не буде показана тут — скопіюйте її зараз.';

  @override
  String get walletSecurityMenuItem => 'Безпека…';

  @override
  String get securityTitle => 'Безпека';

  @override
  String get securityUnavailableBody =>
      'Налаштуваннями безпеки гаманця керує цей застосунок, а не сам гаманець.';

  @override
  String get securityCustodySectionTitle => 'Зберігання ключів';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (апаратне)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (апаратне)';

  @override
  String get securityCustodyTierTee => 'Апаратне сховище ключів (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Програмне сховище ключів';

  @override
  String get securityCustodyTierKeychain => 'Keychain (програмне шифрування)';

  @override
  String get securityCustodyTierNone => 'Немає апаратного сховища ключів';

  @override
  String get securityCustodyTierUnknown => 'Невідомо';

  @override
  String get securityCustodyHardwareKey =>
      'Ключ, яким заблоковано цей гаманець, зберігається в захищеному апаратному забезпеченні цього пристрою й видаляється разом із гаманцем.';

  @override
  String get securityCustodyBestEffort =>
      'Видалення прибирає ваші ключі за принципом найкращих зусиль; короткий проміжок часу для криміналістичного відновлення може лишатися, доки пристрій не перевикористає це сховище. Для повної гарантії скористайтеся також функцією пристрою «Стерти весь вміст».';

  @override
  String get securityCustodyProbeError =>
      'Не вдалося зчитати статус зберігання ключів. Поверніться назад і спробуйте ще раз.';

  @override
  String get securityDeleteWalletButton => 'Видалити гаманець';

  @override
  String get securityDeleteWalletSubtitle =>
      'Видалить цей гаманець і його ключ із цього пристрою. Ваші кошти залишаються в блокчейні та можуть бути відновлені за фразою відновлення.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Видалить цей гаманець і його ключ із цього пристрою. Він не має ключів витрачання, тому тут немає чого резервувати — додайте його знову будь-коли за допомогою ключа перегляду.';

  @override
  String get securityDeleteDialogTitle => 'Видалити цей гаманець?';

  @override
  String get securityDeleteDialogBody =>
      'Це видалить гаманець і його ключ із цього пристрою. Переконайтеся, що ви зробили резервну копію фрази відновлення — це ЄДИНИЙ спосіб відновити ваші кошти.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Це видалить гаманець і його ключ із цього пристрою. Він не має ключів витрачання, тому тут немає чого резервувати — ви можете додати його знову пізніше за допомогою ключа перегляду.';

  @override
  String get securityDeleteDialogConfirm => 'Видалити';

  @override
  String get securityDeleteDialogCancel => 'Скасувати';

  @override
  String get securityDeleteFailedSnack =>
      'Не вдалося видалити гаманець — гаманець не змінився. Спробуйте ще раз.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Спочатку завершіть зміну сервера — вона завершиться або зупиниться протягом $seconds секунд. Потім спробуйте видалити гаманець знову.';
  }

  @override
  String get walletParkedTitle => 'Збережено й очікує';

  @override
  String get walletParkedSubtitle =>
      'Ці платежі ще не надіслано. Їхні суми досі входять до вашого балансу.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Ці платежі ще не надіслано. Їхні суми досі входять до вашого балансу — окрім тих, що гаманець зараз надсилає: їхню суму, можливо, вже зарезервовано.';

  @override
  String get walletParkedCancel => 'Скасувати';

  @override
  String get walletParkedPausedHint =>
      'Призупинено — цей платіж не надішлеться сам собою. Ваші кошти в безпеці. Надішліть його зараз або скасуйте.';

  @override
  String get walletParkedRetryStale =>
      'Цей платіж більше не очікує. Перевірте платежі, що очікують, і активність.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Цей платіж більше не очікує — можливо, гаманець уже надсилає його. Перевірте «Збережено й очікує» і активність.';

  @override
  String get walletReclaimExplainer =>
      'Надсилання через одноразові адреси застрягло. Ви можете його поновити — це перемістить невелику суму між вашими власними адресами і поверне її назад.';

  @override
  String get walletReclaimButton => 'Поновити надсилання';

  @override
  String get walletReclaimInProgress => 'Поновлення…';

  @override
  String get walletReclaimConfirmTitle =>
      'Поновити надсилання через одноразові адреси?';

  @override
  String get walletReclaimConfirmBody =>
      'Це перемістить невелику суму між вашими власними адресами, щоб звільнити одноразові адреси для надсилання, а потім поверне її назад. Коштуватиме кілька комісій мережі. Щойно переказ підтвердиться, поверніть переміщену суму кнопкою «Повернути зараз».';

  @override
  String get walletReclaimConfirmCancel => 'Не зараз';

  @override
  String get walletReclaimConfirmAction => 'Поновити';

  @override
  String get walletReclaimStarted =>
      'Поновлення почалося. Щойно це підтвердиться, надішліть призупинений платіж, а потім поверніть переміщену суму кнопкою «Повернути зараз».';

  @override
  String get walletReclaimNothing => 'Наразі немає чого поновлювати.';

  @override
  String get walletReclaimNotBroadcast =>
      'Не вдалося підтвердити, що це дійшло до мережі. Можливо, це ще пройде — зачекайте трохи, перш ніж спробувати ще раз.';

  @override
  String get walletReclaimNeedsFunds =>
      'Потрібні захищені ZEC, щоб поновити надсилання.';

  @override
  String get walletReclaimFailed =>
      'Наразі не вдалося поновити надсилання. Ваші кошти не змінилися. Спробуйте ще раз.';

  @override
  String get walletReclaimUnknown =>
      'Поновлення завершено. Перевірте свої надсилання через одноразові адреси та поверніть будь-яку переміщену суму кнопкою «Повернути зараз».';

  @override
  String get walletParkedError =>
      'Наразі не вдалося завантажити платежі, що очікують.';

  @override
  String get walletParkedErrorRetry => 'Спробувати ще раз';

  @override
  String get walletParkedErrorRetryInProgress => 'Спроба…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Скасувати цей платіж, що очікує?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Це видалить збережений платіж. Його ще не надіслано, тому з гаманця нічого не спишеться — але цю дію не можна скасувати.';

  @override
  String get walletParkedCancelConfirmKeep => 'Залишити';

  @override
  String get walletParkedCancelConfirmDiscard => 'Видалити платіж';

  @override
  String get walletParkedCancelDone => 'Очікуваний платіж скасовано.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Цей платіж, можливо, уже в дорозі — перевірте активність.';

  @override
  String get walletParkedCancelFailed =>
      'Наразі не вдалося скасувати. Платіж не змінився. Спробуйте ще раз.';

  @override
  String get walletRecoverNow => 'Повернути зараз';

  @override
  String get walletRecoverConfirmTitle => 'Повернути в захищений баланс?';

  @override
  String get walletRecoverConfirmBody =>
      'Це перевірить ваші одноразові адреси й перемістить усе знайдене у ваш приватний захищений баланс. Цю дію безпечно повторювати будь-коли.';

  @override
  String get walletRecoverConfirmCancel => 'Не зараз';

  @override
  String get walletRecoverConfirmAction => 'Повернути';

  @override
  String get walletRecoverInProgress => 'Повернення…';

  @override
  String walletRecoverDone(String amount) {
    return 'Повертаємо $amount у ваш захищений баланс.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'Повертаємо $amount — частину коштів потрібно спробувати повернути ще раз.';
  }

  @override
  String get walletRecoverRetry =>
      'Частину коштів потрібно спробувати повернути ще раз — запустіть повернення знову.';

  @override
  String get walletRecoverTruncated =>
      'Ще не всі одноразові адреси перевірено — запустіть ще раз, щоб перевірити решту.';

  @override
  String get walletRecoverNothing => 'Наразі немає чого повертати.';

  @override
  String get walletRecoverFailed =>
      'Наразі не вдалося повернути кошти. Вони не змінилися. Спробуйте ще раз.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount збережено й очікує · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Скасувати платіж на $amount, збережений $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount призупинено · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount готується до надсилання · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Ваш гаманець готує цей платіж — його суму, можливо, вже зарезервовано. Ваші кошти в безпеці. Якщо це не завершиться, платіж сам повернеться до списку.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Ваш гаманець готує цей платіж — його суму, можливо, вже зарезервовано. Ваші кошти в безпеці, але платіж зможе завершитися лише тоді, коли ваш гаманець знову синхронізуватиметься.';

  @override
  String get walletParkedSendNow => 'Надіслати зараз';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Надсилання платежу на $amount, збереженого $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Надіслати зараз платіж на $amount, збережений $time';
  }

  @override
  String get walletParkedSendNowInProgress => 'Надсилання…';

  @override
  String get walletParkedAuthorizeSent => 'Ваш платіж надсилається зараз.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Ваш платіж надсилається зараз. Якщо він не пройде, гаманець зможе завершити платіж лише тоді, коли знову синхронізуватиметься.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Ще не готово до надсилання. Ваш платіж збережений і не змінився.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Ще не готово до надсилання. Ваш платіж збережений, і він більше не призупинений — спробуйте пізніше знову скористатися кнопкою «Надіслати зараз» або скасуйте його.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Наразі не вдалося надіслати. Платіж не змінився. Спробуйте ще раз.';

  @override
  String get walletTransparentFundsMenuItem => 'Публічні кошти…';

  @override
  String get walletTransparentFundsTitle => 'Публічні кошти';

  @override
  String get walletTransparentFundsIntro =>
      'Публічні кошти публічно видимі в блокчейні — сума, адреси та історія коштів.';

  @override
  String get walletExpertToggleLabel => 'Розширено: публічні кошти';

  @override
  String get walletExpertToggleDescription =>
      'Показати розширені налаштування для зберігання публічних коштів і вимкнення автоматичного захисту.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Показати розширені налаштування для зберігання публічних коштів.';

  @override
  String get walletAutoShieldToggleLabel => 'Захищати автоматично';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Коли ваш публічний баланс досягає $minZec ZEC, кошти автоматично переміщуються у ваш захищений баланс. Якщо це вимкнено, публічні кошти залишаються публічно видимими, доки ви не захистите їх самостійно.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Не вдалося зберегти налаштування. Спробуйте ще раз.';

  @override
  String get walletAutoShieldIncomplete =>
      'Автоматичний захист не завершився — ці кошти досі публічно видимі. Ви можете захистити їх зараз.';

  @override
  String get walletSendPrivacyShielded =>
      'Захищений платіж — сума та отримувач лишаються приватними в блокчейні.';

  @override
  String get walletSendPrivacyTransparent =>
      'Публічний платіж — сума та адреси видимі в блокчейні.';

  @override
  String get walletActivityPublicBadge => 'Публічно видно в блокчейні';

  @override
  String get walletShieldWalletEnded =>
      'Сеанс гаманця завершився. Закрийте й відкрийте його знову, щоб спробувати ще раз.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Нові публічні кошти автоматично захищаються та переміщуються у ваш приватний баланс, щойно їхня сума досягає $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'Автоматичний захист вимкнено — публічні кошти залишаються публічно видимими, доки ви їх не захистите.';

  @override
  String get walletMoveAutoShieldNote =>
      'Автоматичний захист увімкнено: після надходження ці кошти буде автоматично захищено (за додаткову комісію). Щоб залишити їх публічними, спершу вимкніть автоматичний захист у розділі «Публічні кошти».';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Після цього переміщення ваш публічний баланс становитиме $amount ZEC — менше за $floor ZEC, потрібні для повторного захисту. Він залишиться публічним, доки не надійдуть ще кошти.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Ви переміщуєте кошти на власну публічну адресу. Цей запис назавжди залишиться в публічному реєстрі.';

  @override
  String get walletTxDetailVisibility => 'Видимість';

  @override
  String get walletTransparentFundsAutoDenied =>
      'Автоматичний захист призупинено для цього сеансу — його не було схвалено. Ви все ще можете захистити кошти самостійно.';

  @override
  String get walletDeepScanMenuItem => 'Перевірити старіші адреси обміну…';

  @override
  String get walletMenuSyncNotRunningHint => 'Синхронізація зараз не триває.';

  @override
  String get walletDeepScanTitle => 'Перевірити старіші адреси обміну';

  @override
  String get walletDeepScanBody =>
      'Якщо ви відновили цей гаманець і він раніше часто використовувався для обміну, коштам із найстаріших обмінів може знадобитися додатковий крок, щоб їх знайти. Ця перевірка шукає їх — усе знайдене з\'явиться у вашому балансі під час синхронізації гаманця.';

  @override
  String get walletDeepScanCoverage =>
      'Ваші старіші адреси обміну перевірено до цього моменту. Якщо кошти зі старого обміну досі не з\'явилися, перевірте ще старіші адреси.';

  @override
  String get walletDeepScanCoveragePending =>
      'Поточний діапазон ще перевіряється — усе знайдене з\'явиться у вашому балансі. Це може зайняти трохи часу.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Перевіряє наявність коштів від найстаріших обмінів вашого гаманця.';

  @override
  String get walletDeepScanCheckButton => 'Перевірити старіші адреси';

  @override
  String get walletDeepScanCheckDeeperButton => 'Перевірити ще старіші адреси';

  @override
  String get walletDeepScanChecking => 'Перевірка…';

  @override
  String get walletDeepScanClose => 'Закрити';

  @override
  String get walletDeepScanTorHint =>
      'Зараз ви не підключені через Tor. Для більшої приватності варто зачекати, поки Tor стане активним, перш ніж перевіряти.';

  @override
  String get walletDeepScanRescanBusy =>
      'Ви зможете перевірити старіші адреси обміну, щойно повторне сканування завершиться.';

  @override
  String get walletDeepScanRan =>
      'Перевірка старіших адрес обміну триває — усе знайдене з\'явиться у вашому балансі.';

  @override
  String get walletDeepScanFailed =>
      'Не вдалося розпочати перевірку. Нічого не змінилося — спробуйте ще раз.';

  @override
  String get walletDeepScanSlow =>
      'Це триває довше, ніж зазвичай. Якщо ваші старіші адреси обміну було перевірено, усе знайдене з\'явиться у вашому балансі — перевірте ще раз незабаром.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'Обмін зараз вимкнено, тому це неможливо виконати. Спробуйте ще раз, коли обмін стане доступним.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Останній діапазон ще перевіряється — це може зайняти до двох днів, але зазвичай набагато менше. Перевірка завершиться сама — перевірте ще раз пізніше.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'Поки не вдається підтвердити приватність вашого з\'єднання. Для більшої приватності варто перевірити, щойно Tor стане активним.';

  @override
  String get walletDeepScanBannerChecking =>
      'Старіші адреси обміну ще перевіряються — усе знайдене з\'явиться у вашому балансі.';

  @override
  String get walletRescanSwapPointer =>
      'Шукаєте кошти зі старого обміну? Повторне сканування їх не знайде — натомість скористайтеся опцією «Перевірити старіші адреси обміну».';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Відновили гаманець, який використовувався для обміну?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Якщо цей гаманець мав дуже довгу історію обмінів, коштам із найстаріших обмінів може знадобитися додатковий крок, щоб їх знайти. Більшості гаманців нічого не потрібно.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Перевірити зараз';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Закрити';

  @override
  String walletTorHostPath(String transport) {
    return 'Через приватний шлях вашого застосунку ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Через приватний шлях вашого застосунку ($transport); проксі може пов\'язати з\'єднання між собою';
  }

  @override
  String get walletTorHostOtherTransport => 'приватний шлях';

  @override
  String get walletTorHostDirect =>
      'Не приватно (пряме з\'єднання вашого застосунку)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Збережений сервер використовує незашифровану адресу, яку приватний шлях вашого застосунку не може передавати. Використовується $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Докладніше: $label';
  }

  @override
  String get walletSendPaste => 'Вставити';

  @override
  String get walletSendScanQr => 'Сканувати QR-код';

  @override
  String get walletSendRecipientGetsLabel => 'Отримувач отримає';

  @override
  String get walletSwapDepositCopyAmount => 'Копіювати суму';

  @override
  String get walletSwapDepositAmountCopied => 'Суму скопійовано';

  @override
  String get walletScanOpenSettings => 'Відкрити налаштування';

  @override
  String get walletScanOpenSettingsFailed =>
      'Не вдалося відкрити налаштування.';

  @override
  String get walletSendLeaveTitle => 'Надсилання триває';

  @override
  String get walletSendLeaveBody =>
      'Платіж триватиме, якщо ви підете. Чим він завершився, побачите в історії.';

  @override
  String get walletSendLeaveStay => 'Залишитися';

  @override
  String get walletSendLeaveConfirm => 'Піти';

  @override
  String get walletSheetLeaveBody =>
      'Це триватиме, якщо ви підете. Чим усе завершилося, побачите в історії.';

  @override
  String get walletLoadingLabel => 'Завантаження';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'Перевірте, перш ніж захищати знову';

  @override
  String get walletShieldUnknownBody =>
      'Не вдалося підтвердити захист коштів. Перегляньте розділ «Активність», перш ніж спробувати знову.';

  @override
  String get walletMoveUnknownTitle => 'Перевірте, перш ніж переміщувати знову';

  @override
  String get walletMoveUnknownBody =>
      'Не вдалося підтвердити це переміщення. Перегляньте розділ «Активність», перш ніж спробувати знову.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
