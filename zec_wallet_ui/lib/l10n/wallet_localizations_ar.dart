// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Arabic (`ar`).
class WalletLocalizationsAr extends WalletLocalizations {
  WalletLocalizationsAr([String locale = 'ar']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'الإعدادات';

  @override
  String get walletTitle => 'المحفظة';

  @override
  String get walletNotSetUpTitle => 'لم يتم إعداد المحفظة بعد';

  @override
  String get walletNotSetUpBody =>
      'سيتوفر إعداد المحفظة في إصدار لاحق. سيرشدك الإعداد إلى تدوين عبارة الاسترداد الخاصة بك قبل أن يصبح استلام أي أموال ممكنًا — حتى لا يتعرض أي شيء للخطر دون نسخة احتياطية.';

  @override
  String get walletStartupFailedTitle => 'تعذّر تشغيل المحفظة';

  @override
  String get walletStartupFailedBody =>
      'شيء ما حال دون تحميل المحفظة على هذا الجهاز. إذا كانت لديك محفظة بالفعل، فأموالها لم تتأثر — فهي موجودة على شبكة Zcash ويمكن استعادتها باستخدام عبارة الاسترداد الخاصة بك. أعد المحاولة؛ وإذا استمرت المشكلة، أغلق التطبيق وأعد فتحه.';

  @override
  String get walletBalanceLabel => 'الرصيد';

  @override
  String get walletHideBalance => 'إخفاء الرصيد';

  @override
  String get walletShowBalance => 'إظهار الرصيد';

  @override
  String get walletBalanceHiddenAmount => 'الرصيد مخفي';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'قابل للإنفاق الآن';

  @override
  String get walletArrivingLabel => 'وارد';

  @override
  String get walletNotSpendableYetLabel => 'غير قابل للإنفاق بعد';

  @override
  String get walletActivityTitle => 'النشاط';

  @override
  String get walletActivityEmpty => 'لا يوجد نشاط بعد';

  @override
  String get walletActivityError => 'تعذّر تحميل النشاط';

  @override
  String get walletActivityReceived => 'مستلَم';

  @override
  String get walletActivitySent => 'مُرسَل';

  @override
  String get walletActivityPending => 'قيد الانتظار';

  @override
  String get walletActivityQueued => 'في قائمة الانتظار';

  @override
  String get walletActivityRetrying => 'تجري إعادة المحاولة';

  @override
  String get walletActivitySaved => 'محفوظة';

  @override
  String get walletActivityExpired => 'منتهي الصلاحية';

  @override
  String get walletActivityFailed => 'فشل';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count تأكيدات',
      one: 'تأكيد واحد',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'تم استلام $count دفعة',
      many: 'تم استلام $count دفعة',
      few: 'تم استلام $count دفعات',
      two: 'تم استلام دفعتين',
      one: 'تم استلام دفعة',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'إظهار تفاصيل المعاملة';

  @override
  String get walletTxDetailStatus => 'الحالة';

  @override
  String get walletTxDetailFee => 'رسوم الشبكة';

  @override
  String get walletTxDetailDate => 'التاريخ';

  @override
  String get walletTxDetailHeight => 'ارتفاع الكتلة';

  @override
  String get walletTxDetailMemo => 'المذكرة';

  @override
  String get walletTxDetailMemoAttached => 'مُرفقة';

  @override
  String get walletTxDetailTxid => 'معرّف المعاملة';

  @override
  String get walletTxDetailCopyTxid => 'نسخ معرّف المعاملة';

  @override
  String get walletTxDetailCopied => 'تم نسخ معرّف المعاملة';

  @override
  String get walletTxDetailClose => 'إغلاق';

  @override
  String get walletTxFundsKept => 'لم تُخصم أي أموال من محفظتك';

  @override
  String get walletTxExplainQueued =>
      'محفوظة على هذا الجهاز، ضمن \"محفوظة وقيد الانتظار\" — يمكنك إرسالها أو إلغاؤها من هناك.';

  @override
  String get walletTxExplainPending =>
      'أُرسلت إلى شبكة Zcash — بانتظار التأكيد ضمن كتلة.';

  @override
  String get walletTxExplainRetrying =>
      'لم تتمكن محفظتك من إرسال هذه المعاملة إلى شبكة Zcash بعد. تحتفظ بالمعاملة الموقّعة وتعيد المحاولة عند كل مزامنة حتى تنجح أو تنتهي صلاحيتها.';

  @override
  String get walletTxExplainSaved =>
      'احتفظت محفظتك بهذه المعاملة الموقّعة لكنها لا ترسلها من تلقاء نفسها في الوقت الحالي.';

  @override
  String get walletTxExplainConfirmed => 'تم تأكيدها على شبكة Zcash.';

  @override
  String get walletTxExplainExpired =>
      'انتهت صلاحية هذه المعاملة قبل أن تؤكدها الشبكة، لذا أُلغيت. لا يزال بإمكانك إنفاق هذا المبلغ.';

  @override
  String get walletTxExplainFailed =>
      'رفضت الشبكة هذه المعاملة، فلم تُنفَّذ. لا يزال بإمكانك إنفاق هذا المبلغ.';

  @override
  String get walletTxExplainUnknown =>
      'يتعذّر تحديد الحالة الحالية لهذه المعاملة. ستُحدَّث بعد المزامنة التالية.';

  @override
  String get walletMenuTooltip => 'خيارات إضافية';

  @override
  String get walletRescanMenuItem => 'إعادة فحص السجل…';

  @override
  String get walletCheckOneTimeMenuItem => 'التحقق من العناوين لمرة واحدة…';

  @override
  String get walletRescanTitle => 'إعادة فحص سجلّك';

  @override
  String get walletRescanBody =>
      'هل تفتقد أموالًا أقدم؟ أعد فحص سلسلة الكتل من نقطة أبعد في الماضي لاستعادة الإيداعات التي فات تاريخ البدء السابق اكتشافها. أموالك وعبارة الاسترداد لا تتعرضان للخطر أبدًا.';

  @override
  String get walletRescanRangeTitle => 'إلى متى ترجع عملية الفحص';

  @override
  String get walletRescanRangeAll =>
      'فحص كامل سجلّك — الأبطأ، لكنه يستعيد كل شيء.';

  @override
  String get walletRescanRangeDefault =>
      'الفحص جارٍ من بداية محفظتك فصاعدًا. ما زلت تفتقد أموالًا أقدم؟ اختر تاريخًا أبكر، أو فحص كل السجل.';

  @override
  String get walletRescanRangeResolving => 'جارٍ تجهيز النطاق الموصى به…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'نحو $blocks كتلة للفحص.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'الفحص جارٍ من $date فصاعدًا. ما زلت تفتقد أموالًا أقدم؟ اختر تاريخًا أبكر، أو فحص كل السجل.';
  }

  @override
  String get walletRescanPick => 'اختيار تاريخ';

  @override
  String get walletRescanChange => 'تغيير التاريخ';

  @override
  String get walletRescanScanAll => 'فحص كل السجل';

  @override
  String get walletRescanDatePick => 'أقدم تاريخ للفحص';

  @override
  String get walletRescanWarning =>
      'سيؤدي هذا إلى إعادة فحص سلسلة الكتل. التواريخ الحديثة تستغرق دقائق؛ أما الفحص إلى الماضي البعيد فقد يستغرق ساعات. تعمل المزامنة في الخلفية — يمكنك متابعة استخدام محفظتك.';

  @override
  String get walletRescanSettlingAdvisory =>
      'لا تزال هناك عملية دفع من هذه المحفظة قيد التسوية. عادةً ما ترفض المحفظة إعادة الفحص حتى تكتمل — يمكنك المحاولة، لكن توقّع أن يُرفض الطلب.';

  @override
  String get walletRescanConfirm => 'بدء إعادة الفحص';

  @override
  String get walletRescanCancel => 'إلغاء';

  @override
  String get walletRescanRunning => 'جارٍ إعادة البناء…';

  @override
  String get walletRescanRebuildingAll =>
      'جارٍ إعادة بناء سجلّك — يتم فحص السلسلة بأكملها. سيظهر رصيدك ونشاطك تدريجيًا مع تقدّم العملية.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'جارٍ إعادة بناء سجلّك ابتداءً من $date — سيظهر رصيدك ونشاطك تدريجيًا مع تقدّم العملية.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'جارٍ إعادة بناء سجلّك ابتداءً من بداية محفظتك — سيظهر رصيدك ونشاطك تدريجيًا مع تقدّم العملية.';

  @override
  String get walletCatchUpBanner =>
      'جارٍ اللحاق بالركب — يظهر رصيدك ونشاطك تدريجيًا مع مزامنة المحفظة. كل ما تستلمه آمن.';

  @override
  String get walletCatchUpRescanBanner =>
      'جارٍ إعادة بناء سجلّك بعد إعادة الفحص — سيظهر رصيدك ونشاطك تدريجيًا مع تقدّم العملية. كل ما تستلمه آمن.';

  @override
  String get walletRescanFailedNotice =>
      'تعذّرت إعادة الفحص الآن — أموالك آمنة، لكن رصيدك وسجلّك قد يحتاجان بعض الوقت للحاق بالركب. أعد المحاولة بعد قليل.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'لا تزال هناك عملية دفع قيد التسوية، لذا تم إيقاف إعادة الفحص مؤقتًا لحماية أموالك. محفظتك دون تغيير — أعد المحاولة خلال ساعتين تقريبًا مع إبقاء التطبيق مفتوحًا ومتصلًا بالإنترنت.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'إعادة الفحص تعيد بناء سجلّك أثناء مزامنة محفظتك، والمزامنة لا تعمل الآن. محفظتك دون تغيير — أعد المحاولة بمجرد أن تعمل المزامنة.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'لا توجد مساحة كافية لإعادة بناء سجل محفظتك — أموالك آمنة، لكن رصيدك وسجلّك قد يحتاجان بعض الوقت للحاق بالركب. حرّر بعض المساحة وأعد المحاولة.';

  @override
  String get walletRescanFailedDismiss => 'إغلاق';

  @override
  String get walletActivityRebuilding => 'جارٍ إعادة بناء سجلّك…';

  @override
  String get walletActivityCatchingUp =>
      'لا يزال اللحاق بالركب جاريًا — كل ما تستلمه سيظهر هنا.';

  @override
  String get walletActivitySyncNotRunning =>
      'سيكتمل تحميل رصيدك وسجلّك بمجرد أن تعمل المزامنة.';

  @override
  String get walletActivityLoadMore => 'تحميل المزيد';

  @override
  String get walletPendingChangeLabel => 'الباقي قيد الانتظار';

  @override
  String get walletTransparentLabel => 'غير محمي (علني)';

  @override
  String get walletTransparentNote =>
      'غير مشمولة في \"قابل للإنفاق الآن\" — قم بحماية هذه الأموال لتتمكن من إنفاقها. وحتى ذلك الحين تبقى مرئية علنًا على السلسلة.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'هذه الأموال مرئية علنًا على السلسلة.';

  @override
  String walletPoolShielded(String amount) {
    return 'محمي $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'علني $amount';
  }

  @override
  String get walletPoolAllShielded => 'الكل محمي · خاص';

  @override
  String get walletPoolTapHint => 'إظهار الأموال العلنية';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount من رصيدك موجود على عنوان لمرة واحدة (قابل للاسترداد).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount من رصيدك موجود على عنوان لمرة واحدة.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'دفعات بمجموع $amount مخصّصة ولا تزال تُستكمل عبر عناوين لمرة واحدة تتحكم فيها محفظتك. لا ترسلها مرة أخرى.',
      one:
          '$amount مخصّص لدفعة لا تزال محفظتك تُتمّها عبر عنوان لمرة واحدة تتحكم فيه. لا ترسلها مرة أخرى.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'دفعات بمجموع $amount مخصّصة وفي منتصف الطريق عبر عناوين لمرة واحدة تتحكم فيها محفظتك. إنها متوقفة مؤقتًا إلى أن تتزامن محفظتك من جديد. لا ترسلها مرة أخرى.',
      one:
          '$amount مخصّص لدفعة في منتصف الطريق عبر عنوان لمرة واحدة تتحكم فيه محفظتك. إنها متوقفة مؤقتًا إلى أن تتزامن محفظتك من جديد. لا ترسلها مرة أخرى.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'تعذّر التحقق مما إذا كانت هناك دفعة لا تزال قيد الإكمال. جارٍ إعادة المحاولة — وإلى ذلك الحين، ابحث عن دفعة معلّقة في نشاطك قبل أن ترسل مرة أخرى.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount من رصيدك موجود على عنوان لمرة واحدة (لا يزال قيد التأكيد).';
  }

  @override
  String get walletShieldButton => 'حماية';

  @override
  String get walletShieldSheetTitle => 'حماية الأموال العلنية';

  @override
  String get walletShieldNote =>
      'سينقل هذا الأموال من رصيدك العلني المرئي على السلسلة إلى رصيدك المحمي والخاص.';

  @override
  String get walletShieldPreparing => 'جارٍ التحضير…';

  @override
  String get walletShieldAmountLabel => 'قيد الحماية';

  @override
  String get walletShieldFeeLabel => 'رسوم الشبكة';

  @override
  String get walletShieldNetLabel => 'يصل محميًا';

  @override
  String get walletShieldConfirmButton => 'حماية الآن';

  @override
  String get walletShieldSubmitting => 'جارٍ الحماية…';

  @override
  String get walletShieldNothingTitle => 'لا يوجد ما يمكن حمايته بعد';

  @override
  String get walletShieldNothingBody =>
      'هذه الأموال أقل من المبلغ الذي يستحق الحماية حاليًا — إذ ستفوق رسوم الشبكة الفائدة منها. ستصبح قابلة للحماية بمجرد وصول مبلغ إضافي.';

  @override
  String get walletShieldDoneTitle => 'تم إرسال طلب الحماية';

  @override
  String get walletShieldDoneBody =>
      'أموالك في طريقها إلى رصيدك المحمي. سيتم تأكيدها على السلسلة قريبًا.';

  @override
  String get walletShieldSavedTitle => 'تم الحفظ — سنُكمل الحماية';

  @override
  String get walletShieldSavedBody =>
      'تعذّر الوصول إلى الشبكة الآن. تم حفظ عملية الحماية وستُكملها محفظتك في مزامنة لاحقة. لم يُفقد شيء.';

  @override
  String get walletShieldAlreadyTitle => 'تم الإرسال مسبقًا';

  @override
  String get walletShieldFailedTitle => 'تعذّرت الحماية الآن';

  @override
  String get walletShieldStaleBody =>
      'لا تزال المحفظة قيد المزامنة. أعد محاولة الحماية بعد قليل.';

  @override
  String get walletShieldTransientBody =>
      'تعذّر تجهيز عملية الحماية الآن. حاول مرة أخرى بعد لحظات.';

  @override
  String get walletShieldStorageFullBody =>
      'لا توجد مساحة كافية للحماية الآن. حرّر بعض المساحة وأعد المحاولة. أموالك آمنة.';

  @override
  String get walletShieldClose => 'إغلاق';

  @override
  String get walletShieldRetry => 'إعادة المحاولة';

  @override
  String get walletMoveMenuItem => 'النقل إلى علني…';

  @override
  String get walletMoveSheetTitle => 'النقل إلى علني';

  @override
  String get walletMoveSheetSubtitle =>
      'أرسل ZEC محميًا إلى عنوانك العلني الخاص — مفيد عند التعامل مع منصة تبادل لا تقبل الإيداع المحمي.';

  @override
  String get walletMoveDestinationLabel => 'عنوانك العلني';

  @override
  String walletMoveAvailable(String amount) {
    return 'المتاح للنقل: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'المتاح للنقل: $amount ZEC — رصيدك لا يزال يلحق بالركب';
  }

  @override
  String get walletMoveDeshieldTitle => 'هذا النقل يجعل أموالك علنية';

  @override
  String get walletMoveDeshieldBody =>
      'النقل إلى عنوان علني يُخرج هذه الأموال من رصيدك المحمي — إذ يصبح المبلغ وعنوانك العلني مرئيَين علنًا على سلسلة كتل Zcash.';

  @override
  String get walletMoveWalletEnded =>
      'انتهت جلسة المحفظة. أغلقها وأعد فتحها للمحاولة مرة أخرى.';

  @override
  String get walletMoveLoading => 'جارٍ التحضير…';

  @override
  String get walletMovePreparing => 'جارٍ التحقق من المبلغ…';

  @override
  String get walletMoveSubmitting => 'جارٍ النقل…';

  @override
  String get walletMoveReviewButton => 'مراجعة';

  @override
  String get walletMoveCancel => 'إلغاء';

  @override
  String get walletMoveReviewTitle => 'مراجعة النقل';

  @override
  String get walletMoveOwnAddressNote =>
      'أنت تنقل الأموال إلى عنوانك العلني الخاص. يمكنك حماية هذه الأموال مرة أخرى لاحقًا، لكن هذا النقل يبقى في السجل العلني بشكل دائم.';

  @override
  String get walletMoveConfirmButton => 'النقل إلى علني';

  @override
  String get walletMoveBackButton => 'رجوع';

  @override
  String get walletMoveDoneTitle => 'تم النقل إلى علني';

  @override
  String get walletMoveDoneBody =>
      'أموالك في طريقها إلى عنوانك العلني. ستُؤكَّد على السلسلة قريبًا.';

  @override
  String get walletMoveSavedTitle => 'تم الحفظ — سنُكمل النقل';

  @override
  String get walletMoveSavedBody =>
      'تم حفظ هذا النقل وستُرسله محفظتك في مزامنة لاحقة. لم يُفقد شيء.';

  @override
  String get walletMoveAlreadyTitle => 'تم الإرسال مسبقًا';

  @override
  String get walletMoveAlreadyBody =>
      'تم إرسال هذه الأموال مسبقًا وهي في طريقها إلى عنوانك العلني.';

  @override
  String get walletMoveFailedTitle => 'تعذّر إتمام هذا النقل';

  @override
  String get walletMoveNothingTitle => 'لا يوجد ما يمكن نقله بعد';

  @override
  String get walletMoveNothingBody =>
      'ليس لديك رصيد محمي متاح للنقل حاليًا. بمجرد تأكيد الأموال، يمكنك نقلها إلى عنوانك العلني.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'محفظتك لا تزال تلحق بالركب — كل ما تستلمه يصبح متاحًا للنقل بمجرد اكتمال المزامنة.';

  @override
  String get walletMoveCouldNotLoad =>
      'تعذّر تحميل عنوانك العلني. أعد المحاولة.';

  @override
  String get walletMoveRetry => 'إعادة المحاولة';

  @override
  String get walletMoveClose => 'إغلاق';

  @override
  String get walletSnapshotUnavailable =>
      'تعذّرت قراءة المحفظة الآن. ستتحدّث تلقائيًا.';

  @override
  String get walletBalanceStale => 'تعذّر التحديث — يُعرض آخر رصيد معروف.';

  @override
  String get walletSyncStartFailed => 'تعذّر بدء المزامنة. سنواصل المحاولة.';

  @override
  String get walletSyncRetry => 'إعادة المحاولة';

  @override
  String get walletSyncTryNow => 'حاول الآن';

  @override
  String get walletSyncIdle => 'لا مزامنة بعد';

  @override
  String get walletSyncIdleDetail => 'تبدأ المزامنة تلقائيًا.';

  @override
  String get walletSyncDisabled => 'المزامنة متوقفة';

  @override
  String get walletSyncDisabledDetail =>
      'فعّل المزامنة من إعدادات هذا التطبيق لتحديث رصيدك.';

  @override
  String get walletSyncExplainDisabled =>
      'تم إيقاف المزامنة في إعدادات هذا التطبيق. أموالك آمنة. يعرض رصيدك ونشاطك آخر حالة تمت مزامنتها ولن يتم تحديثهما حتى يتم تفعيل المزامنة.';

  @override
  String get walletParkedSyncPausedNote =>
      'محفظتك لا تُزامن الآن، لذا لن تُرسل هذه الدفعات من تلقاء نفسها. استخدم \"إرسال الآن\" لإرسال إحداها بنفسك.';

  @override
  String get walletSyncPausedMoneyNote =>
      'متوقف مؤقتًا إلى أن تتزامن محفظتك من جديد.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'جارٍ الاتصال…';

  @override
  String get walletSyncStartingDetail =>
      'جارٍ الوصول إلى شبكة Zcash والتحضير للفحص.';

  @override
  String get walletSyncConnecting => 'جارٍ الاتصال…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'جارٍ الاتصال… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'جارٍ الفحص $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'جارٍ الفحص…';

  @override
  String get walletSyncSpendableReady => 'الأموال جاهزة للإنفاق.';

  @override
  String get walletSyncCatchingUp =>
      'جارٍ اللحاق بالشبكة — قد تستغرق المزامنة الأولية الكاملة بعض الوقت. يمكنك متابعة استخدام التطبيق حتى تكتمل';

  @override
  String walletSyncScanRemaining(String count) {
    return 'تبقّى $count كتلة';
  }

  @override
  String get walletSyncUpToDate => 'محدّثة بالكامل';

  @override
  String get walletSyncOffline => 'غير متصل';

  @override
  String get walletSyncOfflineDetail =>
      'تبقى العمليات المُدرجة في قائمة الانتظار محفوظة ضمن \"محفوظة وقيد الانتظار\".';

  @override
  String get walletSyncUnknown => 'جارٍ المزامنة…';

  @override
  String get walletSyncStalled => 'المزامنة متوقفة مؤقتًا';

  @override
  String get walletStallEndpoint =>
      'يتعذّر الوصول إلى شبكة Zcash حاليًا. سنواصل المحاولة تلقائيًا — يُرجى التحقق من اتصالك بالإنترنت، أو قد يكون الخادم غير متاح مؤقتًا.';

  @override
  String get walletStallTor =>
      'المسار الخاص في تطبيقك غير متاح، لذا لا تتصل المحفظة. تحقّق من إعدادات الشبكة في تطبيقك، أو أوقف المسار الخاص. ستستأنف المزامنة بمجرد عودة المسار.';

  @override
  String get walletStallStorage =>
      'تخزين الجهاز ممتلئ. حرّر بعض المساحة وستُستأنف المزامنة.';

  @override
  String get walletStallReorg =>
      'أعادت السلسلة تنظيم نفسها؛ جارٍ إعادة التحقق من الكتل الأخيرة.';

  @override
  String get walletStallInternal =>
      'أوقفت مشكلة محلية المزامنة. إذا استمرت المشكلة، استعد المحفظة من عبارة الاسترداد.';

  @override
  String get walletStallEndpointMisbehaving =>
      'أرسل هذا الخادم بيانات لا يمكن أن تكون صحيحة، لذلك توقفت المزامنة. هذه ليست مشكلة في الاتصال — انتقل إلى خادم آخر. إذا رُفضت جميع الخوادم، أعد فحص السجل: قد تحتفظ المحفظة بسجل خاطئ من خادم سابق.';

  @override
  String get walletStallBirthdayInFuture =>
      'هذه المحفظة مضبوطة للبدء من كتلة لم يصل إليها هذا الخادم بعد. تحقق من كتلة البداية المضبوطة لهذه المحفظة، أو جرّب خادماً آخر.';

  @override
  String get walletStallStorageUnavailable =>
      'توقفت المزامنة مؤقتاً على هذا الجهاز. جارٍ إعادة المحاولة.';

  @override
  String get walletStallUnknown => 'توقفت المزامنة لسبب غير معروف.';

  @override
  String get walletSyncBadgeHint => 'إظهار تفاصيل المزامنة';

  @override
  String get walletSyncSheetClose => 'إغلاق';

  @override
  String get walletSyncSheetProgress => 'التقدّم';

  @override
  String get walletSyncSheetBlocksLeft => 'الكتل المتبقية';

  @override
  String get walletSyncSheetSyncedTo => 'تمت المزامنة حتى الكتلة';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'متأخر بما لا يقل عن $blocks كتلة',
      many: 'متأخر بما لا يقل عن $blocks كتلة',
      few: 'متأخر بما لا يقل عن $blocks كتل',
      two: 'متأخر بكتلتين على الأقل',
      one: 'متأخر بكتلة واحدة على الأقل',
      zero: 'متأخر بما لا يقل عن $blocks كتلة',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'لم تبدأ المزامنة بعد — تبدأ تلقائيًا. لا حاجة لأي إجراء.';

  @override
  String get walletSyncExplainStartFailed =>
      'تعذّر بدء المزامنة. أموالك آمنة — المحفظة لا تتحقق حاليًا من أي نشاط جديد فحسب. أعد المحاولة أدناه، أو أعد فتح التطبيق.';

  @override
  String get walletSyncExplainStarting =>
      'تتصل المحفظة بشبكة Zcash وتستعد للفحص. يستغرق هذا عادةً بضع ثوانٍ.';

  @override
  String get walletSyncExplainConnecting => 'جارٍ إنشاء اتصال بشبكة Zcash.';

  @override
  String get walletSyncExplainScanning =>
      'تتحقق المحفظة من كتل سلسلة الكتل بحثًا عن أموالك. يتحدّث رصيدك ونشاطك عند العثور على معاملات جديدة — يمكنك متابعة استخدام التطبيق حتى تكتمل العملية.';

  @override
  String get walletSyncExplainUpToDate =>
      'متزامنة بالكامل مع شبكة Zcash. رصيدك ونشاطك محدّثان.';

  @override
  String get walletSyncExplainStalled =>
      'واجهت المزامنة مشكلة وهي متوقفة مؤقتًا. ستُعاد المحاولة تلقائيًا.';

  @override
  String get walletSyncExplainStalledOffline =>
      'يتعذّر الوصول إلى شبكة Zcash — وهذا أمر طبيعي إذا كنت غير متصل بالإنترنت، أو قد يكون الخادم غير متاح مؤقتًا. أموالك آمنة: يُعرض الرصيد بحسب آخر حالة تمت مزامنتها، وتبقى العمليات المُدرجة في قائمة الانتظار محفوظة ضمن \"محفوظة وقيد الانتظار\". تتم إعادة محاولة الاتصال تلقائيًا.';

  @override
  String get walletSyncExplainOffline =>
      'لا يوجد اتصال بالشبكة. أموالك آمنة — يُعرض الرصيد بحسب آخر حالة تمت مزامنتها، وتبقى العمليات المُدرجة في قائمة الانتظار محفوظة ضمن \"محفوظة وقيد الانتظار\".';

  @override
  String get walletSyncExplainUnknown =>
      'المحفظة قيد المزامنة. يتحدّث رصيدك ونشاطك مع تقدّم العملية.';

  @override
  String get walletTorOff => 'Tor متوقف';

  @override
  String get walletTorBootstrapping => 'جارٍ تشغيل المسار الخاص…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return 'جارٍ تشغيل $transport…';
  }

  @override
  String get walletTorActive => 'Tor نشط';

  @override
  String get walletTorActiveUnverified => 'Tor نشط (بيئة تشغيل غير موثّقة)';

  @override
  String get walletTorActiveUnattested =>
      'المسار الخاص قيد الاستخدام (الخصوصية غير مؤكدة)';

  @override
  String get walletTorFellBack => 'Tor غير متاح — يُستخدم اتصال مباشر';

  @override
  String get walletTorUnavailable => 'المسار الخاص غير متاح — غير متصل';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport غير متاح — غير متصل';
  }

  @override
  String get walletTorUnanswered => 'المسار الخاص متصل — لا يصل أي رد';

  @override
  String get walletTorUnansweredUnattested =>
      'المسار الخاص متصل — لا يصل أي رد (الخصوصية غير مؤكدة)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport متصل — لا يصل أي رد';
  }

  @override
  String get walletTorUnansweredDirect =>
      'غير خاص (اتصال تطبيقك المباشر) — لا يصل أي رد';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'متصل عبر $transport — لا يصل أي رد؛ يمكن للوكيل ربط الاتصالات ببعضها';
  }

  @override
  String get walletTorUnknown =>
      'حالة Tor غير معروفة — تعامل معها على أنها غير محمية';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'الرصيد (حتى الكتلة $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'الرصيد · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'الرصيد (حتى الكتلة $height، $time)';
  }

  @override
  String get walletSyncSheetConnection => 'الاتصال';

  @override
  String get walletSyncSheetServer => 'الخادم';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'الخادم، $host، يفتح اختيار الخادم';
  }

  @override
  String get walletSyncServerSheetTitle => 'خادم المزامنة';

  @override
  String get walletSyncServerInUse => 'قيد الاستخدام';

  @override
  String get walletSyncServerAppDefault => 'الافتراضي للتطبيق';

  @override
  String get walletSyncServerCustom => 'خادم مخصص…';

  @override
  String get walletSyncServerCustomHint => 'https://المضيف:المنفذ';

  @override
  String get walletSyncServerCheck => 'التحقق من الخادم';

  @override
  String get walletSyncServerChecking => 'جارٍ التحقق…';

  @override
  String get walletSyncServerUse => 'استخدام هذا الخادم';

  @override
  String get walletSyncServerSwitching => 'جارٍ التبديل…';

  @override
  String get walletSyncServerContinue => 'متابعة';

  @override
  String get walletSyncServerCancel => 'إلغاء';

  @override
  String get walletSyncServerTrustTitle => 'هل تثق بهذا الخادم؟';

  @override
  String get walletSyncServerTrustNotice =>
      'أنت تثق بهذا الخادم للإبلاغ عن رصيدك وسجلك ولتمرير مدفوعاتك. سيرى عنوان IP الخاص بك ما لم يكن Tor مفعّلًا، ووقت إنشاء محفظتك تقريبًا، والعناوين العلنية التي تتحقق منها محفظتك، والمعاملات التي تستعلم عنها، والمعاملات التي ترسلها.';

  @override
  String get walletSyncServerKeyLabel => 'مفتاح الوصول (اختياري)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'ترويسة المفتاح';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'أدخل الترويسة التي يتوقعها خادمك';

  @override
  String get walletSyncServerKeyInvalid =>
      'لا يمكن استخدام هذا المفتاح أو هذه الترويسة';

  @override
  String get walletSyncServerKeySaved => 'تم حفظ المفتاح';

  @override
  String get walletSyncServerKeyShow => 'إظهار';

  @override
  String get walletSyncServerKeyHide => 'إخفاء';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'مفتاحك يعرّفك لدى هذا الخادم. يمكنه ربط مدفوعاتك بمحفظتك، حتى عبر Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'سيعيد التبديل بدء المزامنة الجارية. يبقى رصيدك وسجلك كما هما. قد تظهر الأموال كواردة حتى تلحق مزامنة الخادم الجديد.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'سيعيد التبديل الاتصال بالخادم الجديد. يبقى رصيدك وسجلك كما هما.';

  @override
  String get walletSyncServerUnreachable =>
      'تعذّر الوصول إلى هذا الخادم. تحقق من العنوان — وإذا كان صحيحًا، فإما أن هذا الخادم لا يستجيب أو أن تطبيقك لا يستطيع الوصول إليه الآن. حاول مرة أخرى أو اختر خادمًا آخر.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'تعذّر الوصول إلى هذا الخادم. لا تستطيع المحفظة التمييز بين أن هذا الخادم لا يستجيب وأن تطبيقك لا يستطيع الوصول إليه الآن. اختر خادمًا آخر أو حاول لاحقًا.';

  @override
  String get walletSyncServerWrongNetwork =>
      'هذا الخادم على شبكة Zcash مختلفة.';

  @override
  String get walletSyncServerInvalidUrl =>
      'لا يبدو هذا عنوان خادم. استخدم الصيغة https://المضيف:المنفذ.';

  @override
  String get walletSyncServerNotOffered => 'هذا التطبيق لا يوفر هذا الخادم.';

  @override
  String get walletSyncServerBusy =>
      'المحفظة مشغولة الآن. حاول مرة أخرى بعد لحظات.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'لم يعد هذا التطبيق يوفر الخادم الذي اخترته. يتم استخدام $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'تعذّرت قراءة اختيار الخادم المحفوظ. يتم استخدام $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'تعذّر التبديل — لا يزال $host قيد الاستخدام.';
  }

  @override
  String get walletTransportExplainDirect =>
      'تتصل حركة بيانات المحفظة مباشرة بالخادم. يمكن للخادم رؤية عنوان IP الخاص بك.';

  @override
  String get walletTransportExplainTor =>
      'تُوجَّه حركة بيانات المحفظة عبر شبكة Tor، التي تُخفي عنوان IP الخاص بك عن الخادم.';

  @override
  String get walletTransportExplainBootstrapping =>
      'جارٍ تشغيل المسار الخاص لتطبيقك. تنتظر حركة بيانات المحفظة حتى يجهز قبل الاتصال.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return 'جارٍ تشغيل $transport. تنتظر حركة بيانات المحفظة حتى يجهز قبل الاتصال.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'تعذّر الوصول إلى Tor، لذا تحوّلت حركة البيانات إلى اتصال مباشر. يمكن للخادم رؤية عنوان IP الخاص بك.';

  @override
  String get walletTransportExplainUnavailable =>
      'المسار الخاص في تطبيقك غير متاح، لذا لا تتصل المحفظة. أوقف المسار الخاص، أو تحقّق من إعدادات الشبكة في تطبيقك.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport غير متاح، لذا لا تتصل المحفظة. أوقفه، أو تحقّق من إعدادات الشبكة في تطبيقك.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'قبِل المسار الخاص الاتصال، لكن لم يصل أي رد منذ دقيقة. قد يكون السبب المسار أو خادم المحفظة — لا تستطيع المحفظة التمييز بينهما. ستواصل المحاولة؛ وإذا استمر ذلك، جرّب خادمًا آخر أو تحقّق من إعدادات الشبكة في تطبيقك.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport قبِل الاتصال، لكن لم يصل أي رد منذ دقيقة. قد يكون السبب المسار أو خادم المحفظة — لا تستطيع المحفظة التمييز بينهما. ستواصل المحاولة؛ وإذا استمر ذلك، جرّب خادمًا آخر أو تحقّق من إعدادات الشبكة في تطبيقك.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'تتصل حركة بيانات المحفظة مباشرة بالخادم. يمكن للخادم رؤية عنوان IP الخاص بك. تم قبول الاتصال، لكن لم يصل أي رد منذ دقيقة. قد يكون السبب المسار أو خادم المحفظة — لا تستطيع المحفظة التمييز بينهما. ستواصل المحاولة؛ وإذا استمر ذلك، جرّب خادمًا آخر أو تحقّق من إعدادات الشبكة في تطبيقك.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'يتعذّر التحقق من خصوصية هذا الاتصال — تعامل معه على أنه غير خاص. تم قبول الاتصال، لكن لم يصل أي رد منذ دقيقة. قد يكون السبب المسار أو خادم المحفظة — لا تستطيع المحفظة التمييز بينهما. ستواصل المحاولة؛ وإذا استمر ذلك، جرّب خادمًا آخر أو تحقّق من إعدادات الشبكة في تطبيقك.';

  @override
  String get walletTransportExplainUnverified =>
      'يتعذّر التحقق من خصوصية هذا الاتصال — تعامل معه على أنه غير خاص.';

  @override
  String get walletTransportExplainHostProxy =>
      'تُوجَّه حركة بيانات المحفظة عبر وسيلة النقل الخاصة بالتطبيق، التي تُخفي عنوان IP الخاص بك عن الخادم.';

  @override
  String get walletOnboardingWelcomeTitle => 'إعداد محفظتك';

  @override
  String get walletOnboardingWelcomeBody =>
      'أنشئ محفظة جديدة لاستلام ZEC والاحتفاظ به. سننشئ عبارة استرداد ونرشدك إلى نسخها احتياطيًا قبل أن يتسنى وصول أي أموال — حتى لا يتعرض أي شيء للخطر دون نسخة احتياطية.';

  @override
  String get walletCreateButton => 'إنشاء محفظة جديدة';

  @override
  String get walletRestoreButton => 'الاستعادة من عبارة استرداد';

  @override
  String get walletWatchOnlyButton => 'مشاهدة محفظة (للمشاهدة فقط)';

  @override
  String get walletWatchOnlyTitle => 'مشاهدة محفظة';

  @override
  String get walletWatchOnlyBody =>
      'الصق مفتاح مشاهدة لمشاهدة محفظة بدون مفاتيح إنفاقها. سترى رصيدها وسجلها، لكن لن تتمكن من إرسال الأموال. اختر تاريخ بدء المحفظة التقريبي حتى نعرف إلى أي مدى نعود في البحث.';

  @override
  String get walletWatchOnlyKeyLabel => 'مفتاح المشاهدة';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => 'مسح رمز QR لمفتاح المشاهدة';

  @override
  String get walletWatchOnlyScanTitle => 'مسح مفتاح المشاهدة';

  @override
  String get walletWatchOnlyScanInstruction =>
      'وجّه الكاميرا نحو رمز QR الخاص بمفتاح المشاهدة.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'الكاميرا غير متاحة. الصق المفتاح يدويًا بدلاً من ذلك.';

  @override
  String get walletWatchOnlyScanManualEntry => 'الصق بدلاً من ذلك';

  @override
  String get walletWatchOnlyScanHint =>
      'أو اضغط على زر المسح لقراءة رمز QR لمفتاح المشاهدة.';

  @override
  String get walletWatchOnlyScanFilled => 'تم مسح مفتاح المشاهدة.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'تاريخ بدء المحفظة';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'الفحص من $date فصاعدًا — لن تظهر الأموال المستلمة قبل ذلك التاريخ. محفظة أقدم؟ اختر تاريخًا أبكر.';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'اختر تاريخ بدء المحفظة';

  @override
  String get walletWatchOnlyBirthdayChange => 'تغيير التاريخ';

  @override
  String get walletWatchOnlySubmit => 'مشاهدة هذه المحفظة';

  @override
  String get walletWatchOnlyBack => 'رجوع';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'هذا لا يبدو مفتاح مشاهدة صالحًا. تحقق منه وحاول مرة أخرى.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'مفتاح المشاهدة هذا مخصص لشبكة مختلفة. لا يمكن استخدامه هنا.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'توجد بالفعل محفظة على هذا الجهاز. ارجع وافتحها بدلاً من ذلك.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'تاريخ البدء هذا حديث جدًا. اختر تاريخًا أقدم.';

  @override
  String get walletRestoreTitle => 'استعادة محفظتك';

  @override
  String get walletRestoreBody =>
      'أدخل عبارة الاسترداد لاستعادة محفظتك — اكتب الكلمات أو ألصقها بالترتيب، مفصولة بمسافات. العبارات القياسية فقط: إذا استخدمت محفظتك عبارة مرور إضافية (\"الكلمة الخامسة والعشرون\")، لا يمكن لهذا التطبيق استعادتها بعد — سترى محفظة فارغة، وليس خطأً.';

  @override
  String get walletRestorePhraseHint =>
      'الكلمة الأولى  الكلمة الثانية  الكلمة الثالثة  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count كلمة',
      one: 'كلمة واحدة',
      zero: 'لا توجد كلمات بعد',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'تتكوّن عبارات الاسترداد من 12 أو 15 أو 18 أو 21 أو 24 كلمة';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count كلمات ليست كلمات استرداد صحيحة — صحّح الكلمات المميّزة',
      one: 'كلمة واحدة ليست كلمة استرداد صحيحة — صحّح الكلمة المميّزة',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'الكلمة $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'الكلمة $index: ليست كلمة استرداد';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'إزالة الكلمة $index';
  }

  @override
  String get walletRestoreSubmit => 'استعادة المحفظة';

  @override
  String get walletRestoreBack => 'رجوع';

  @override
  String get walletRestoreBirthdayTitle => 'إلى متى ترجع عملية الفحص';

  @override
  String get walletRestoreBirthdayNone =>
      'سنفحص سجلّك بالكامل — أبطأ، لكن لن يُفوَّت شيء.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'الفحص من $date فصاعدًا — لن تظهر الأموال المستلمة قبل ذلك التاريخ. محفظة أقدم؟ اختر تاريخًا أبكر، أو فحص كل السجل.';
  }

  @override
  String get walletRestoreBirthdayPick => 'اختيار تاريخ';

  @override
  String get walletRestoreBirthdayChange => 'تغيير التاريخ';

  @override
  String get walletRestoreBirthdayClear => 'فحص كل السجل';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'الكلمة $index ليست كلمة استرداد. تحقق من عبارتك بحثًا عن أخطاء إملائية، ثم أعد المحاولة.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'عبارة الاسترداد هذه غير صالحة. تحقق من الكلمات وترتيبها، ثم أعد المحاولة.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'لا تطابق هذه العبارة المحفظة الموجودة على هذا الجهاز. تحقق منها جيدًا وأعد المحاولة.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'توجد محفظة بالفعل على هذا الجهاز. ارجع لفتحها.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'هذا التاريخ حديث جدًا. اختر تاريخًا أبكر، أو افحص كل شيء.';

  @override
  String get walletGeneratingLabel => 'جارٍ إنشاء محفظتك…';

  @override
  String get walletOpeningLabel => 'جارٍ فتح محفظتك…';

  @override
  String get walletBackupTitle => 'نسخ عبارة الاسترداد احتياطيًا';

  @override
  String get walletBackupBody =>
      'هذه الكلمات هي الوسيلة الوحيدة لاسترداد محفظتك وأموالك. دوّنها بالترتيب واحتفظ بها في مكان آمن وخاص. لا تشاركها أبدًا ولا تخزّنها على الإنترنت — فأي شخص يملك هذه الكلمات يمكنه الاستيلاء على أموالك.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'لقطات الشاشة معطّلة في هذه الشاشة.';

  @override
  String get walletBackupSecureNoteOther => 'تأكّد من ألا يرى أحد شاشتك.';

  @override
  String get walletBackupReveal => 'إظهار عبارة الاسترداد';

  @override
  String get walletBackupRevealing => 'جارٍ تحضير عبارة الاسترداد…';

  @override
  String get walletBackupRevealFailed =>
      'تعذّر إظهار عبارة الاسترداد الآن. تأكّد من أن جهازك غير مقفل، ثم أعد المحاولة.';

  @override
  String get walletBackupRetryReveal => 'إعادة المحاولة';

  @override
  String get walletBackupReauthFailed =>
      'تعذّر التحقق من هويتك. يُرجى إعادة المحاولة.';

  @override
  String get walletBackupConfirmCheckbox =>
      'لقد دوّنت عبارة الاسترداد الخاصة بي وحفظتها في مكان آمن.';

  @override
  String get walletBackupContinue => 'متابعة';

  @override
  String get walletBackupSaveFailed =>
      'تعذّر حفظ تأكيدك. يُرجى إعادة المحاولة.';

  @override
  String get walletBackupStartOver => 'البدء من جديد';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'البدء من جديد بدون هذه المحفظة؟';

  @override
  String get walletBackupStartOverConfirmBody =>
      'سيؤدي هذا إلى حذف هذه المحفظة من الجهاز وإعادتك إلى البداية. لا يمكن إيداع أي شيء عبر هذا التطبيق قبل اكتمال الإعداد.\n\nإذا سبق أن احتوت هذه المحفظة على أموال — أو استُعيدت من عبارة استرداد — فتلك العبارة وحدها يمكنها استعادتها.';

  @override
  String get walletBackupStartOverConfirm => 'حذف والبدء من جديد';

  @override
  String get walletBackupStartOverKeep => 'الاحتفاظ بهذه المحفظة';

  @override
  String get walletBackupSectionTitle => 'عبارة الاسترداد';

  @override
  String get walletBackupTileTitle => 'نسخ عبارة الاسترداد احتياطيًا';

  @override
  String get walletBackupTileSubtitle =>
      'إظهار الكلمات التي يمكنها استرداد محفظتك وأموالك.';

  @override
  String get walletBackupScreenTitle => 'عبارة الاسترداد';

  @override
  String get walletBackupDone => 'تم';

  @override
  String get walletBackupManagedTitle => 'لا توجد عبارة استرداد منفصلة';

  @override
  String get walletBackupManagedBody =>
      'تم إعداد هذه المحفظة باستخدام حسابك من التطبيق الذي ثبّتها، لذا ليست لها عبارة استرداد خاصة بها. تُسترد أموالك مع ذلك الحساب — استخدم نسخته الاحتياطية للحفاظ عليها بأمان.';

  @override
  String get walletExportViewingKeyTitle => 'تصدير مفتاح المشاهدة';

  @override
  String get walletExportViewingKeyTileTitle => 'تصدير مفتاح المشاهدة';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'مشاركة نسخة للمشاهدة فقط من محفظتك — يمكنها رؤية سجلك، لكن لا يمكنها الإنفاق.';

  @override
  String get walletExportViewingKeyWarning =>
      'يتيح هذا المفتاح لأي شخص يملكه رؤية كل ما استلمته هذه المحفظة وأرسلته على الإطلاق، وكل ما ستستلمه وترسله مستقبلًا. لا يمكن لهذا المفتاح إنفاق أموالك أو استرداد محفظتك. شاركه فقط مع شخص تثق به لرؤية سجلك الكامل، مثل محاسبك أو جهازك الثاني الخاص بك. الطريقة الوحيدة لإلغاء مشاركته لاحقًا هي نقل أموالك إلى محفظة جديدة.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'يتيح هذا المفتاح لأي شخص يملكه رؤية كل ما استلمته هذه المحفظة وأرسلته على الإطلاق، وكل ما ستستلمه وترسله مستقبلًا. لا يمكن لهذا المفتاح إنفاق أموالك أو استرداد محفظتك. شاركه فقط مع شخص تثق به لرؤية سجلك الكامل، مثل محاسبك أو جهازك الثاني الخاص بك. بمجرد مشاركته، لا يمكن إلغاء مشاركته.';

  @override
  String get walletExportViewingKeyReveal => 'إظهار مفتاح المشاهدة';

  @override
  String get walletExportViewingKeyRetry => 'إعادة المحاولة';

  @override
  String get walletExportViewingKeyRevealing => 'جارٍ تحضير مفتاح المشاهدة…';

  @override
  String get walletExportViewingKeyFailed =>
      'تعذّر إظهار مفتاح المشاهدة الآن. أعد المحاولة بعد قليل.';

  @override
  String get walletExportViewingKeyQrLabel => 'رمز QR لمفتاح المشاهدة';

  @override
  String get walletExportViewingKeyCopy => 'نسخ مفتاح المشاهدة';

  @override
  String get walletExportViewingKeyCopied => 'تم نسخ مفتاح المشاهدة';

  @override
  String get walletExportViewingKeyDone => 'تم';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'لقطات الشاشة معطّلة في هذه الشاشة.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'تأكّد من ألا يرى أحد شاشتك.';

  @override
  String get walletWatchOnlySectionTitle => 'حول هذه المحفظة للمشاهدة فقط';

  @override
  String get walletWatchOnlyAboutBody =>
      'هذه محفظة للمشاهدة فقط. تم إعدادها من مفتاح مشاهدة، لذا يمكنها رؤية رصيدك وسجلك لكنها لا تملك أي مفاتيح إنفاق — لا يوجد هنا شيء لنسخه احتياطيًا، ولا يمكنها إرسال الأموال.';

  @override
  String get walletWatchOnlyBadge => 'مشاهدة فقط';

  @override
  String get walletOnboardingFailedTitle => 'تعذّر إكمال إعداد المحفظة';

  @override
  String get walletOnboardingRetry => 'إعادة المحاولة';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'التخزين الآمن في هاتفك لا يستجيب. افتح قفل جهازك وأعد المحاولة. إذا استمر حدوث ذلك، فأعد تشغيل هاتفك.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'هذه المحفظة مفتوحة في نافذة أو تطبيق آخر، أو لا تزال تُنهي عملية سابقة. أغلق أي نافذة أخرى تستخدمها — أو انتظر لحظة — ثم أعد المحاولة.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'لم يعد المفتاح الآمن لهذه المحفظة متاحًا، لذا يتعذّر فتحها على هذا الجهاز. أموالك آمنة — استعدها من عبارة الاسترداد لاستردادها.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'الاستعادة من عبارة الاسترداد';

  @override
  String get walletOnboardingRecoverConfirmTitle => 'استعادة هذه المحفظة؟';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'تأكّد من امتلاكك عبارة الاسترداد قبل المتابعة — ستحتاجها في الشاشة التالية لاسترداد أموالك. أموالك آمنة على سلسلة الكتل وتخضع لتلك العبارة. سيؤدي هذا إلى إزالة بيانات المحفظة غير القابلة للقراءة من هذا الجهاز حتى يتم إعادة بنائها.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'إلغاء';

  @override
  String get walletOnboardingFailedStorageFull =>
      'لا توجد مساحة كافية لإعداد محفظتك. حرّر بعض المساحة وأعد المحاولة.';

  @override
  String get walletOnboardingFailedNoVault =>
      'لا يحتوي هذا الجهاز على مخزن مفاتيح آمن، لذا لا يمكن للمحفظة حماية عبارة الاسترداد هنا.';

  @override
  String get walletOnboardingFailedNetwork =>
      'تعذّر الوصول إلى الشبكة أثناء الإعداد. تحقق من اتصالك وأعد المحاولة.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'لم يكتمل إعداد المحفظة. أعد المحاولة لإتمامه — لم يُفقد شيء.';

  @override
  String get walletOnboardingFailedUnknown =>
      'حدث خطأ ما أثناء إعداد محفظتك. أعد المحاولة.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'إعداد محفظة هذا التطبيق غير مهيأ بشكل صحيح، لذا يتعذّر على المحفظة البدء. إعادة المحاولة لن تُجدي نفعًا — يُرجى إبلاغ مطوّر التطبيق بذلك. أموالك لم تتأثر.';

  @override
  String get walletSendButton => 'إرسال';

  @override
  String get walletSendSyncNotRunning =>
      'المزامنة لا تعمل — لا يمكن تحديث رصيدك القابل للإنفاق';

  @override
  String get walletSendWaitingForFunds =>
      'لا تزال المزامنة جارية — يمكنك الإرسال بمجرد أن يكون لديك رصيد قابل للإنفاق';

  @override
  String get walletSendNoSpendableYet => 'لا يوجد رصيد قابل للإنفاق بعد';

  @override
  String get walletSendSyncUnavailable =>
      'يمكنك الإرسال بمجرد استئناف المزامنة';

  @override
  String get walletSendTitle => 'إرسال';

  @override
  String get walletSendUnavailable =>
      'محفظتك غير جاهزة الآن. ارجع وأعد المحاولة.';

  @override
  String get walletSendWatchOnly =>
      'هذه محفظة للمشاهدة فقط. يمكنها إظهار الأرصدة واستلام المدفوعات، لكنها لا تحتوي على مفاتيح إنفاق — لذا لا يمكنها الإرسال.';

  @override
  String get walletSendExpiredTitle => 'انتهت صلاحية طلب الدفع هذا';

  @override
  String get walletSendExpiredBody =>
      'استغرقت شاشة الإرسال أكثر من خمس ثوانٍ لتفتح، لذا أُبلغ التطبيق بأن شيئًا لم يُرسل. هذا الجواب نهائي: لا يمكن دفع هذا الطلب من هنا. للدفع، ابدأ من جديد من التطبيق.';

  @override
  String get walletSendFaultWatchOnly =>
      'هذه محفظة للمشاهدة فقط — لا تحتوي على مفاتيح إنفاق، لذا لا يمكنها الإرسال.';

  @override
  String walletSendAvailable(String amount) {
    return 'المتاح للإرسال: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'المتاح للإرسال: $amount ZEC — رصيدك لا يزال يلحق بالركب';
  }

  @override
  String get walletSendRecipientLabel => 'عنوان المستلم';

  @override
  String get walletSendRecipientHint => 'عنوان Zcash (يبدأ بـ u أو z أو t)';

  @override
  String get walletSendRecipientLocked => 'لا يمكن تغيير المستلم هنا';

  @override
  String get walletSendAmountLabel => 'المبلغ (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'المذكرة (اختياري)';

  @override
  String get walletSendMemoHint =>
      'تُسلَّم فقط إلى المستلمين المحميين (الخاصين)';

  @override
  String get walletSendMemoTransparentDisabled =>
      'تتطلّب المذكرات مستلمًا محميًا. لا يمكن لهذا العنوان العلني استلام مذكرة.';

  @override
  String get walletSendMemoMachineDisabled =>
      'هذه الدفعة تحمل بالفعل مرجعًا من التطبيق، لذا لا يمكنها أيضًا أخذ مذكرة مكتوبة.';

  @override
  String get walletSendMachineMemoTitle => 'التطبيق يرفق مرجعًا';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'يقول إن هذا من أجل: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'سيبقى مع المعاملة ولا يمكن إزالته لاحقًا. لا يستطيع المحفظة التحقق من محتواه.';

  @override
  String get walletSendRecipientShielded => 'محمي · خاص';

  @override
  String get walletSendRecipientTransparent => 'علني';

  @override
  String get walletSendRecipientInvalid => 'هذا لا يبدو عنوان Zcash صالحًا.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'هذا العنوان مخصص لشبكة Zcash مختلفة.';

  @override
  String get walletSendReviewButton => 'مراجعة الدفعة';

  @override
  String get walletSendQueueButton => 'إضافة إلى قائمة الانتظار للإرسال لاحقًا';

  @override
  String get walletSendQueueHint =>
      'تبقى الدفعة المُدرجة في قائمة الانتظار ضمن \"محفوظة وقيد الانتظار\"، حيث يمكنك إرسالها أو إلغاؤها. تُحسَب رسوم الشبكة عند إرسالها.';

  @override
  String get walletSendPreparing => 'جارٍ تحضير دفعتك…';

  @override
  String get walletSendSubmitting => 'جارٍ الإرسال…';

  @override
  String get walletSendQueuing => 'جارٍ الإضافة إلى قائمة الانتظار…';

  @override
  String get walletSendReviewTitle => 'تأكيد الدفعة';

  @override
  String get walletSendTotalLabel => 'الإجمالي';

  @override
  String get walletSendFeeLabel => 'رسوم الشبكة';

  @override
  String get walletSendChangeLabel => 'الباقي المُعاد';

  @override
  String get walletSendDeshieldTitle => 'هذه الدفعة غير خاصة';

  @override
  String get walletSendDeshieldBody =>
      'تُرسل إلى عنوان علني، لذا سيكون المبلغ والمستلم مرئيَين علنًا على سلسلة كتل Zcash.';

  @override
  String get walletSendPublicAckLabel => 'أفهم أن هذه الدفعة ستكون علنية.';

  @override
  String get walletSendConfirmButton => 'إرسال الآن';

  @override
  String get walletSendBackButton => 'رجوع';

  @override
  String get walletSendSelfSendNote =>
      'أنت ترسل إلى محفظتك الخاصة. لا تزال رسوم الشبكة سارية.';

  @override
  String get walletSendLargeConfirmTitle => 'إرسال مبلغ كبير؟';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'هذا يقارب رصيدك بأكمله. لا يمكن التراجع عن دفعة أُرسلت.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'هذه دفعة كبيرة. لا يمكن التراجع عن دفعة أُرسلت.';

  @override
  String get walletSendLargeConfirmBoth =>
      'هذه دفعة كبيرة — تقارب رصيدك بأكمله. لا يمكن التراجع عن دفعة أُرسلت.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'إرسال $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'الرجوع';

  @override
  String get walletSendSentTitle => 'تم إرسال الدفعة';

  @override
  String get walletSendSentBody => 'تم بث دفعتك إلى الشبكة.';

  @override
  String get walletSendSavedTitle => 'تم الحفظ — سنُكمل الإرسال';

  @override
  String get walletSendSavedBody =>
      'تعذّر إرسال دفعتك الآن، لذا تم حفظها وستُرسلها محفظتك في مزامنة لاحقة. لم يُفقد شيء.';

  @override
  String get walletSendKeptTitle => 'محفوظة';

  @override
  String get walletSendKeptBody =>
      'احتفظت محفظتك بهذه المعاملة ولم تتعهد بإرسالها من تلقاء نفسها. راجع «النشاط» لمعرفة حالتها.';

  @override
  String get walletSendPartialBody =>
      'خرج جزء من دفعتك؛ وستُكمل محفظتك الباقي في مزامنة لاحقة. لم يُفقد شيء.';

  @override
  String get walletSendInMotionTitle => 'الدفعة قيد التنفيذ';

  @override
  String get walletSendInMotionBody =>
      'بدأت دفعتك وهي تنتقل عبر عنوان لمرة واحدة تتحكم فيه محفظتك. لا ترسلها مرة أخرى. إذا لم تكتمل، يمكنك استرداد الأموال من شاشة محفظتك.';

  @override
  String get walletSendAlreadyTitle => 'تم الإرسال مسبقًا';

  @override
  String get walletSendAlreadyBody =>
      'تم إرسال هذه الدفعة مسبقًا — لن تُرسل مرتين.';

  @override
  String get walletSendFailedTitle => 'تعذّر إتمام الدفعة';

  @override
  String get walletSendFailedBody =>
      'حدث خطأ ما أثناء إتمام هذه الدفعة ولم يُرسل شيء. يمكنك إعادة المحاولة.';

  @override
  String get walletSendTryAgain => 'إعادة المحاولة';

  @override
  String get walletSendDone => 'تم';

  @override
  String get walletSendAnother => 'إرسال دفعة أخرى';

  @override
  String get walletSendQueuedTitle => 'أُضيفت إلى قائمة الانتظار للإرسال';

  @override
  String get walletSendQueuedBody =>
      'تم حفظ هذه الدفعة. ستجدها ضمن \"محفوظة وقيد الانتظار\"، حيث يمكنك إرسالها الآن أو إلغاؤها.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'الرصيد القابل للإنفاق غير كافٍ — لديك $available ZEC وهذا يتطلّب $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'تمت ترقية شبكة Zcash ويحتاج هذا التطبيق إلى تحديث قبل أن يتمكن من الإرسال. أموالك آمنة.';

  @override
  String get walletSyncUpToDateLimited =>
      'محدَّث بقدر ما يستطيع هذا الإصدار قراءته';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'تمت ترقية شبكة Zcash. فحص هذا الإصدار كل ما يستطيع قراءته، لكن الكتل الأحدث قد تحتوي على أموال لا يستطيع عرضها بعد، وملاحظات المدفوعات الأخيرة غير متاحة. حدِّث التطبيق لرؤية كل شيء.';

  @override
  String get walletSyncUpToDateDegraded =>
      'محدّث، لكن هذا الخادم لا يخدم كل المجمعات';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'يرفض هذا الخادم أحد مجمعات Zcash المحمية أو يحجبه أو يبلّغ عنه بشكل خاطئ. لا يمكن إنفاق الأموال المستلمة في ذلك المجمع عبر هذا الخادم، والرصيد المعروض هو حد أدنى. انتقل إلى خادم آخر لاستخدامها — هذه ليست مشكلة في الاتصال.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: يرفض هذا الخادم خدمته';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: يحجب هذا الخادم جزءاً منه';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: يُبلّغ هذا الخادم عنه بشكل خاطئ';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: حالة خدمة هذا الخادم له غير معروفة';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'محدّث مع هذا الخادم، لكن الخادم متأخر عن الشبكة';

  @override
  String get walletSyncExplainEndpointBehind =>
      'تتوقف نسخة السلسلة لدى هذا الخادم عند كتلة تجاوزتها الشبكة قبل بناء هذا الإصدار من التطبيق، لذا فإن رصيدك محدّث حتى تلك الكتلة فقط. قد لا تظهر المدفوعات الجديدة الواردة إليك بعد، وقد لا يصل دفع مُرسل من هنا. انتقل إلى خادم آخر للحاق بالشبكة — هذه ليست مشكلة في الاتصال.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'في انتظار تحديث التطبيق — أموالك آمنة ولم يُرسل أي شيء.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'في انتظار خادم يُبلّغ عن إصدار الشبكة — بدّل الخادم. أموالك آمنة ولم يُرسل أي شيء.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'في انتظار خادم يُبلّغ عن إصدار الشبكة. إذا كان تاريخ هذا الجهاز ووقته غير صحيحين، فصحّحهما أولاً — ثم بدّل الخادم. أموالك آمنة ولم يُرسل أي شيء.';

  @override
  String get walletSyncUnverified =>
      'محدّث، لكن هذا الخادم لا يُبلّغ عن إصدار الشبكة';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other: 'لا يزال الإرسال يعمل لنحو $hours ساعة — ثم بدّل الخادم.',
      many: 'لا يزال الإرسال يعمل لنحو $hours ساعة — ثم بدّل الخادم.',
      few: 'لا يزال الإرسال يعمل لنحو $hours ساعات — ثم بدّل الخادم.',
      two: 'لا يزال الإرسال يعمل لنحو ساعتين — ثم بدّل الخادم.',
      one: 'لا يزال الإرسال يعمل لنحو ساعة واحدة — ثم بدّل الخادم.',
      zero: 'لا يزال الإرسال يعمل لأقل من ساعة — ثم بدّل الخادم.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'لا يزال الإرسال يعمل لنحو $blocks كتلة — ثم بدّل الخادم.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'لم يُبلّغ هذا الخادم عن إصدار الشبكة منذ $blocks كتلة، لذا لا يستطيع هذا التطبيق التأكد من أن الإرسال آمن. انتقل إلى خادم آخر.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'لم يُبلّغ هذا الخادم عن إصدار الشبكة منذ يوم، لذا لا يستطيع هذا التطبيق التأكد من أن الإرسال آمن. إذا كان تاريخ هذا الجهاز ووقته غير صحيحين، فصحّحهما أولاً — ثم انتقل إلى خادم يُبلّغ عن إصدار الشبكة.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'لم يُبلّغ هذا الخادم عن إصدار الشبكة قط، لذا لا يستطيع هذا التطبيق التأكد من أن الإرسال آمن. انتقل إلى خادم آخر.';

  @override
  String get walletSyncExplainUnverified =>
      'لا يذكر هذا الخادم على أي إصدار من شبكة Zcash يعمل، لذا لا يستطيع هذا التطبيق التأكد من أن الدفعة التي يوقّعها ستُقبل. رصيدك محدّث. انتقل إلى خادم آخر — هذه ليست مشكلة في الاتصال.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'لا يذكر هذا الخادم على أي إصدار من شبكة Zcash يعمل، لذا لا يستطيع هذا التطبيق التأكد من أن الدفعة التي يوقّعها ستُقبل. كما استمر في تقديم كتل اضطرت المحفظة بعدها إلى التراجع عنها، لذا قد لا يكون رصيدك محدّثًا. انتقل إلى خادم آخر — هذه ليست مشكلة في الاتصال.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'يواصل هذا الخادم أيضًا تقديم كتل تضطر المحفظة بعدها إلى التراجع عنها — انتقل إلى خادم آخر.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'رصيدك لا يزال يلحق بالركب — قد يتوفر المزيد مع مزامنة المحفظة.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return 'لا يزال $pending ZEC في طور الوصول وسيصبح قابلًا للإنفاق بمجرد أن تُنهي المحفظة المزامنة.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'أدخل مبلغًا للإرسال.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'أدخل المبلغ كرقم، على سبيل المثال 0.25.';

  @override
  String get walletSendFaultAmountDecimals => 'لدى ZEC 8 خانات عشرية كحد أقصى.';

  @override
  String get walletSendFaultAmountNotPositive => 'أدخل مبلغًا أكبر من الصفر.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'هذا المبلغ أكبر من إجمالي المعروض من ZEC.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'يحدّ هذا التطبيق حاليًا الإرسال بـ $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'هذا لا يبدو عنوان Zcash صالحًا لهذه الشبكة. تحقق منه وأعد المحاولة.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'لا يمكن لهذا المستلم استلام مذكرة. احذف المذكرة، أو أرسل إلى عنوان محمي (خاص).';

  @override
  String get walletSendFaultMemoTooLong =>
      'مذكرتك طويلة جدًا. اختصرها وأعد المحاولة.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'لا يمكن إرسال هذه المذكرة. احذفها وأعد المحاولة.';

  @override
  String get walletSendFaultMemoConflict =>
      'تعذّر إرسال هذه الدفعة — أرفق التطبيق ملاحظتين بها. لم يُرسل أي شيء.';

  @override
  String get walletSendFaultNetworkMismatch => 'هذا العنوان مخصص لشبكة مختلفة.';

  @override
  String get walletSendFaultUriInvalid =>
      'تعذّر إنشاء هذه الدفعة. تحقق من العنوان والمبلغ.';

  @override
  String get walletSendFaultNotSynced =>
      'لم تتزامن محفظتك بما يكفي بعد. انتظر حتى تلحق المزامنة، أو أضِف هذه الدفعة إلى قائمة الانتظار لإرسالها لاحقًا.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'لم تتزامن محفظتك بما يكفي بعد. انتظر حتى تلحق المزامنة.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'لم تتزامن محفظتك بما يكفي بعد، والمزامنة لا تعمل الآن. تحقّق من حالة المزامنة في شاشة المحفظة.';

  @override
  String get walletSendFaultAmountsExpired =>
      'انتهت صلاحية المبالغ أثناء مراجعتك. يُرجى مراجعة الدفعة مرة أخرى.';

  @override
  String get walletSendFaultQueueFull =>
      'هناك عدد كبير جدًا من العمليات بانتظار الإرسال. دعها تُرسل أولًا، ثم أعد المحاولة.';

  @override
  String get walletSendFaultWalletBusy =>
      'المحفظة مشغولة الآن. أعد المحاولة بعد قليل.';

  @override
  String get walletSendFaultStorageFull =>
      'لا توجد مساحة كافية لإتمام هذا الإرسال. حرّر بعض المساحة وأعد المحاولة.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'هناك عدد كبير جدًا من العناوين لمرة واحدة قيد الاستخدام الآن. قد يتحرر بعضها مع تأكّد التحويلات، لكن قد لا يُحل هذا من تلقاء نفسه. أموالك آمنة.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'تعذّر تحضير هذه الدفعة. تحقق من التفاصيل وأعد المحاولة.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'تعذّر تجهيز هذه الدفعة الآن. حاول مرة أخرى بعد لحظات.';

  @override
  String get walletSwapButton => 'مبادلة';

  @override
  String get walletSwapTitle => 'مبادلة ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'محفظتك غير جاهزة الآن. ارجع وأعد المحاولة.';

  @override
  String get walletSwapUnavailableOff => 'المبادلة غير متاحة حاليًا.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'هذه محفظة للمشاهدة فقط — لا يمكنها المبادلة.';

  @override
  String get walletSwapDone => 'تم';

  @override
  String get walletSwapBackToWallet => 'العودة إلى المحفظة';

  @override
  String walletSwapAvailable(String amount) {
    return 'المتاح للمبادلة: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'المتاح للمبادلة: $amount ZEC — رصيدك لا يزال يلحق بالركب';
  }

  @override
  String get walletSwapAssetLabel => 'الأصل المُستلَم';

  @override
  String get walletSwapAmountLabel => 'المبلغ المراد مبادلته (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'عنوان الوجهة';

  @override
  String get walletSwapDestinationHint =>
      'عنوان الاستلام الخاص بك على سلسلة الوجهة';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'عنوان استلام $chain الخاص بك';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'عنوان $chain — حيث يُرسل الأصل الذي حصلت عليه من المبادلة. تحقق جيدًا من صحة السلسلة.';
  }

  @override
  String get walletSwapDestinationScanTooltip => 'مسح رمز QR لعنوان الوجهة';

  @override
  String get walletSwapTargetAssetHint => 'اختر أصلًا لاستلامه';

  @override
  String get walletSwapQuoteButton => 'الحصول على عرض سعر';

  @override
  String get walletSwapQuoting => 'جارٍ الحصول على عرض السعر…';

  @override
  String get walletSwapExecuting => 'جارٍ بدء المبادلة…';

  @override
  String get walletSwapExecuteStillWorking =>
      'لا تزال العملية جارية — المبادلة قيد البدء. قد يستغرق هذا حتى دقيقة واحدة.';

  @override
  String get walletSwapReviewTitle => 'تأكيد المبادلة';

  @override
  String get walletSwapYouSendLabel => 'ترسل';

  @override
  String get walletSwapYouReceiveLabel => 'تستلم على الأقل';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'رسوم الشبكة';

  @override
  String get walletSwapNetworkFeeValue => 'تُضاف عند إرسال الإيداع';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'عرض السعر صالح لمدة $time تقريبًا — أكّد قبل انتهاء صلاحيته.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'عرض السعر صالح لمدة أقل من دقيقة — أكّد قبل انتهاء صلاحيته.';

  @override
  String get walletSwapQuoteExpired =>
      'انتهت صلاحية عرض السعر هذا. ارجع واحصل على عرض جديد — لم يعد السعر مضمونًا، والإرسال الآن قد يؤدي إلى استرداد الأموال.';

  @override
  String get walletCountdownUnderMinute => 'أقل من دقيقة';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes د';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds ث';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '$hours س $minutes د';
  }

  @override
  String get walletSwapDeshieldTitle => 'هذه المبادلة غير خاصة';

  @override
  String get walletSwapDeshieldBody =>
      'مبادلة ZEC للخارج تُزيل الحماية عنه — فالإيداع معاملة علنية، وجانب المزوّد علني على شبكته.';

  @override
  String get walletSwapDiscloseTitle => 'ما الذي سيراه مزوّد المبادلة';

  @override
  String get walletSwapDiscloseAmounts => 'المبالغ على الجانبين';

  @override
  String get walletSwapDiscloseCrossLink =>
      'أن هذا الـZEC والأصل الذي تستلمه جزء من مبادلة واحدة';

  @override
  String get walletSwapDiscloseDestination => 'عنوان وجهتك';

  @override
  String get walletSwapDiscloseSource => 'عنوان مصدرك';

  @override
  String get walletSwapDiscloseIp =>
      'عنوان IP الخاص بك (ما لم تمرّر الاتصال عبر Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'تفاصيل أخرى لهذه المبادلة';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'معاملات المزوّد نفسه علنية على شبكته';

  @override
  String get walletSwapAckLabel =>
      'أفهم أن المزوّد سيرى المعلومات المذكورة أعلاه.';

  @override
  String get walletSwapConfirmButton => 'بدء المبادلة';

  @override
  String get walletSwapBackButton => 'رجوع';

  @override
  String get walletSwapStatusPendingTitle => 'بدأت المبادلة';

  @override
  String get walletSwapStatusCheckingTitle => 'جارٍ التحقق من حالة المبادلة…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'محفظتك ترسل إيداع ZEC إلى المزوّد. إذا انقطع اتصالك بالإنترنت لفترة وجيزة، سيُرسَل تلقائيًا بمجرد عودة الاتصال — لكن نافذة الإرسال قصيرة، وإذا أُغلقت قبل ذلك، تنتهي المبادلة ببساطة ولا يتم تبادل أي شيء. يبقى الـZEC الخاص بك ملكًا لك، وقد يستغرق الأمر حتى ساعة قبل أن يظهر مجددًا كرصيد قابل للإنفاق.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'بانتظار وصول إيداعك. إذا لم ترسل الأموال بعد من محفظتك الأخرى، فأرسلها قبل انتهاء صلاحية عرض السعر.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'لا تزال هذه المبادلة بانتظار إيداعها. لم تعد تعليمات الإيداع متاحة على هذا الجهاز — إذا كنت قد أرسلت الأموال بالفعل، فسيتم رصدها؛ وإذا لم ترسلها بعد، فدع هذه المبادلة تنتهي صلاحيتها وابدأ مبادلة جديدة.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'تنتهي مهلة الإيداع في $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'انتهت مهلة الإيداع. إذا لم يُرسَل الإيداع في الوقت المحدد، تنتهي المبادلة ويبقى ZEC الخاص بك في محفظتك.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'انتهت مهلة الإيداع. إذا لم ترسل إيداعك بعد، تنتهي هذه المبادلة ببساطة — احصل على عرض سعر جديد عندما تكون مستعدًا.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'مبادلات قيد التنفيذ',
      one: 'مبادلة قيد التنفيذ',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'ZEC الخاص بك في طريقه إلى مزوّد المبادلة.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'بانتظار وصول إيداعك إلى مزوّد المبادلة.';

  @override
  String get walletSwapInFlightRowGeneric => 'هناك مبادلة قيد التنفيذ.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'انتهت مهلة الإيداع — تحقق من حالة هذه المبادلة.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'لم تصل هذه المبادلة إلى نتيجة مؤكدة هنا بعد — افتحها للتحقق. أي ZEC يعود إلى هذه المحفظة يظهر في رصيدك بعد المزامنة.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'لم تصل هذه المبادلة إلى نتيجة مؤكدة هنا بعد — افتحها للتحقق. أي ZEC تسلّمه هذه المبادلة إلى هذه المحفظة يظهر في رصيدك بعد المزامنة.';

  @override
  String get walletSwapRowOutcomeSuccess => 'اكتملت المبادلة.';

  @override
  String get walletSwapRowOutcomeRefunded => 'استُردت المبادلة.';

  @override
  String get walletSwapRowOutcomeFailed => 'لم تكتمل المبادلة.';

  @override
  String get walletSwapRemove => 'إزالة';

  @override
  String get walletSwapRemoveTitle => 'إزالة هذه المبادلة من القائمة؟';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'هذا يزيل المبادلة من هذه القائمة فقط — ولا يلغي المبادلة نفسها، وستتوقف هذه المحفظة عن تتبّع استردادها. الـZEC المسترد لاحقًا لا يزال ملكًا لهذه المحفظة؛ يمكن لإعادة الفحص الكاملة أن تجده.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'هذا يزيل المبادلة من هذه القائمة فقط — ولا يلغي المبادلة نفسها، وستتوقف هذه المحفظة عن تتبّع الـZEC الوارد إليها. الـZEC الذي يصل لاحقًا لا يزال ملكًا لهذه المحفظة؛ يمكن لإعادة الفحص الكاملة أن تجده. أما إذا تم استرداد المبادلة بدلاً من ذلك، فيعود الاسترداد بالعملة التي أرسلتها، خارج هذه المحفظة.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'هذا يزيل المبادلة من هذه القائمة فقط — ولا يلغي المبادلة نفسها، وستتوقف هذه المحفظة عن تتبّع أي ZEC ما زال في طريقه إليها منها. الـZEC الذي يصل لاحقًا لا يزال ملكًا لهذه المحفظة؛ يمكن لإعادة الفحص الكاملة أن تجده.';

  @override
  String get walletSwapRemoveBodyDone =>
      'هذا يزيل المبادلة المكتملة من القائمة.';

  @override
  String get walletSwapRemoveCancel => 'إلغاء';

  @override
  String get walletSwapRemoveConfirm => 'إزالة';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'بدأت في $time';
  }

  @override
  String get walletSwapViewSwap => 'إظهار المبادلة';

  @override
  String get walletSwapsInFlightError =>
      'تعذّر تحميل مبادلاتك قيد التنفيذ الآن.';

  @override
  String get walletSwapsInFlightRetry => 'إعادة المحاولة';

  @override
  String get walletSwapsInFlightRetryInProgress => 'جارٍ المحاولة…';

  @override
  String get walletSwapStartAnother => 'بدء مبادلة أخرى';

  @override
  String get walletSwapStatusUnderTitle => 'بانتظار الإيداع الكامل';

  @override
  String get walletSwapStatusUnderBody =>
      'وصل جزء من الإيداع. الباقي قيد الاكتمال، أو سيقوم المزوّد بالاسترداد.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'وصل جزء من إيداعك. أرسل المبلغ المتبقي قبل الموعد النهائي، وإلا فسيسترد المزوّد ما وصل بالفعل.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'تم استلام $received؛ لا يزال $missing ناقصًا. تنتهي مهلة الإيداع في $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'تم استلام الإيداع';

  @override
  String get walletSwapStatusDetectedBody =>
      'استلم المزوّد إيداعك وسيعالج المبادلة.';

  @override
  String get walletSwapStatusProcessingTitle => 'جارٍ معالجة مبادلتك';

  @override
  String get walletSwapStatusProcessingBody => 'يقوم المزوّد بإتمام مبادلتك.';

  @override
  String get walletSwapStatusSuccessTitle => 'اكتملت المبادلة';

  @override
  String get walletSwapStatusSuccessBody => 'انتهت مبادلتك بنجاح.';

  @override
  String get walletSwapStatusRefundedTitle => 'تم استرداد المبادلة';

  @override
  String get walletSwapStatusRefundedBody =>
      'لم تكتمل المبادلة، لذا أعاد المزوّد الأموال إلى عنوان الاسترداد الخاص بك.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'لم تكتمل المبادلة، لذا أرسل المزوّد الـZEC الخاص بك إلى هذه المحفظة. يصل كأموال غير محمية ويظهر في رصيدك بعد المزامنة التالية للمحفظة — وقد يستغرق ذلك بعض الوقت.';

  @override
  String get walletSwapStatusFailedTitle => 'فشلت المبادلة';

  @override
  String get walletSwapStatusFailedBody =>
      'تعذّر إتمام المبادلة. أي أموال تم إيداعها ستُسوَّى أو تُسترد من جانب المزوّد.';

  @override
  String get walletSwapStatusNotFoundTitle => 'المبادلة غير موجودة';

  @override
  String get walletSwapStatusNotFoundBody =>
      'لم يعد لدى المزوّد سجلّ لهذه المبادلة — على الأرجح انتهت صلاحيتها. إذا تم إجراء إيداع، فينبغي أن يعيده المزوّد إلى عنوان الاسترداد. المبادلة تبقى في قائمتك، وتستمر هذه المحفظة في تتبّع الـZEC الخاص بها في حال وصوله لاحقًا؛ يمكنك إزالتها من القائمة في أي وقت.';

  @override
  String get walletSwapStatusUnknownTitle => 'الحالة غير متاحة';

  @override
  String get walletSwapStatusUnknownBody =>
      'يتعذّر قراءة حالة هذه المبادلة الآن.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'التتبّع غير متاح';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'المبادلة معطّلة، لذا لا يمكننا تتبّع هذا هنا. أي أموال ستُسوَّى أو تُسترد من جانب المزوّد.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'المبادلة معطّلة هنا، لذا لا يمكن تتبّع هذه المبادلة الآن. إذا استُردت، فإن الـZEC يعود إلى هذه المحفظة — ويظهر في رصيدك بعد إعادة تفعيل المبادلة ومزامنة المحفظة.';

  @override
  String get walletSwapTrackingError => 'تعذّر تتبّع هذه المبادلة.';

  @override
  String get walletSwapTrackingErrorBody =>
      'تعذّر فتح تتبّع هذه المبادلة. قد تكون المبادلة نفسها لا تزال جارية — أي أموال تم إيداعها ستُسوَّى أو تُسترد من جانب المزوّد.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'أدخل العنوان الذي تريد استلام الأصل المُبادل عليه.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'عنوان الوجهة هذا غير صالح لهذا الأصل. تحقق منه وأعد المحاولة.';

  @override
  String get walletSwapFaultExpired =>
      'انتهت صلاحية عرض السعر هذا. احصل على عرض جديد للمتابعة.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'تحرّك سعر المزوّد خارج الحد الذي وضعته، لذا أُوقفت المبادلة قبل أن تنتقل أي أموال. أعد المحاولة.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'حد الانزلاق السعري مرتفع جدًا لمبادلة آمنة. أعد المحاولة.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'مزوّد المبادلة غير متاح الآن. أعد المحاولة بعد قليل.';

  @override
  String get walletSwapFaultConnection =>
      'تعذّر الوصول إلى خدمة المبادلة. يُرجى التحقق من اتصالك بالإنترنت وإعادة المحاولة.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'أعاد مزوّد المبادلة استجابة غير متوقعة، لذا أُوقفت المبادلة. أعد المحاولة.';

  @override
  String get walletSwapFaultSwapOff => 'المبادلة معطّلة حاليًا.';

  @override
  String get walletSwapFaultDepositFailed =>
      'تعذّر إرسال إيداعك، فلم تُخصم أي أموال من محفظتك. احصل على عرض سعر جديد لإعادة المحاولة.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'هناك مبادلة قيد التنفيذ بالفعل. يمكنك بدء مبادلة جديدة بعد أن تُسوَّى بالكامل أو تنتهي صلاحية عرض سعرها — وقد يستغرق ذلك بعض الوقت.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'لا يمكن لهذه المحفظة إعداد عنوان استرداد بعد — عادةً ما يعني ذلك ببساطة أن المزامنة الأولى لم تكتمل بعد. انتظر اكتمال المزامنة، ثم أعد المحاولة.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'لا يمكن لهذه المحفظة إعداد عنوان استلام لهذه المبادلة بعد — عادةً ما يعني ذلك ببساطة أن المزامنة الأولى لم تكتمل بعد. انتظر اكتمال المزامنة، ثم أعد المحاولة.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'تعذّر بدء المبادلة في الوقت المحدد — قد يكون الاتصال بطيئًا، أو كانت المحفظة مشغولة. احصل على عرض سعر جديد وأعد المحاولة.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'المحفظة مشغولة للحظة. أعد المحاولة.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'عرض السعر هذا لا يطابق العرض الذي أصدرته محفظتك، لذا لم يُرسل شيء. احصل على عرض سعر جديد وحاول مرة أخرى.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'تحتاج هذه المبادلة إلى ما يقارب $needed ZEC شاملة رسوم الشبكة، لكن المتاح للإنفاق حاليًا هو $spendable ZEC فقط.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'يحدّ هذا التطبيق حاليًا المبادلة بـ $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'تحتاج هذه المبادلة إلى ما يقارب $needed ZEC شاملة رسوم الشبكة، لكن المتاح للإنفاق حاليًا هو $spendable ZEC فقط. رصيدك لا يزال يلحق بالركب — قد يصبح المزيد متاحًا قريبًا.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'تعذّر على المحفظة تسجيل هذه المبادلة بأمان، فلم تنتقل أي أموال. أعد المحاولة.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'تعذّرت معالجة طلب المبادلة هذا. احصل على عرض سعر جديد وأعد المحاولة.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'تعذّر الحصول على عرض سعر للمبادلة. تحقق من التفاصيل وأعد المحاولة.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'محفظتك غير جاهزة الآن. ارجع وأعد المحاولة.';

  @override
  String get walletSwapDirectionBuy => 'شراء ZEC';

  @override
  String get walletSwapDirectionSell => 'بيع ZEC';

  @override
  String get walletSwapRefundLabel => 'عنوان الاسترداد الخاص بك';

  @override
  String get walletSwapRefundHint => 'أين تعود عملاتك إذا فشلت المبادلة';

  @override
  String get walletSwapRefundHelper =>
      'على السلسلة التي ترسل منها — وليس عنوان Zcash.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'عنوان استرداد $chain الخاص بك';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'عنوان $chain — حيث تعود عملاتك إذا فشلت المبادلة. وليس عنوان Zcash.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'حول عنوان الاسترداد الخاص بك';

  @override
  String get walletSwapRefundInfoBody =>
      'إذا تعذّر إتمام المبادلة، يرسل المزوّد عملاتك مرة أخرى إلى هذا العنوان على السلسلة التي دفعت منها. أدخل عنوانًا تتحكم فيه — لا يمكن للمحفظة التحقق من عنوان خارجي نيابة عنك، لذا تحقّق منه بعناية.';

  @override
  String get walletSwapRefundScanTooltip => 'مسح رمز QR لعنوان الاسترداد';

  @override
  String get walletSwapScanTitle => 'مسح العنوان';

  @override
  String get walletSwapScanInstruction =>
      'وجّه الكاميرا نحو رمز QR الخاص بالعنوان.';

  @override
  String get walletSwapScanManualEntry => 'إدخال يدوي';

  @override
  String get walletSwapScanCancel => 'إلغاء';

  @override
  String get walletSwapScanCameraUnavailable =>
      'الكاميرا غير متاحة. أدخل العنوان يدويًا أدناه.';

  @override
  String get walletSwapSourceAssetLabel => 'الأصل المراد المبادلة منه';

  @override
  String get walletSwapSourceAssetHint => 'اختر أصلًا';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'المبلغ المراد إرساله ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'المبلغ المراد إرساله';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol على $chain';
  }

  @override
  String get walletSwapPickerTitle => 'اختر أصلًا للمبادلة منه';

  @override
  String get walletSwapPickerTitleReceive => 'اختر أصلًا لاستلامه';

  @override
  String get walletSwapPickerStale =>
      'تعذّر تحديث قائمة الأصول — تُعرض آخر قائمة معروفة.';

  @override
  String get walletSwapPickerEmpty =>
      'لا توجد أصول متاحة للمبادلة الآن. أعد المحاولة لاحقًا.';

  @override
  String get walletSwapPickerSearchHint => 'ابحث بالاسم أو السلسلة';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'لا توجد أصول مطابقة لـ \"$query\".';
  }

  @override
  String get walletSwapPickerError =>
      'تعذّر تحميل قائمة الأصول. تحقق من اتصالك وأعد المحاولة.';

  @override
  String get walletSwapPickerRetry => 'إعادة المحاولة';

  @override
  String get walletSwapSlippageLabel => 'نسبة تحمّل الانزلاق السعري';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'مخصّص';

  @override
  String get walletSwapSlippageCustomLabel => 'انزلاق سعري مخصّص';

  @override
  String get walletSwapSlippageMayFail =>
      'منخفضة جدًا — قد تفشل المبادلة إذا تحرّك السعر.';

  @override
  String get walletSwapSlippageNormal => 'نسبة تحمّل آمنة.';

  @override
  String get walletSwapSlippageRisky =>
      'مرتفعة — قد تستلم أقل بشكل ملحوظ مما هو مذكور في عرض السعر.';

  @override
  String get walletSwapSlippageTooHigh =>
      'مرتفعة جدًا — سترفض المبادلة. خفّضها إلى 10% أو أقل.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'ستستلم على الأقل $zec ZEC — وهو الحد الأدنى بنسبة انزلاق $slippage%. لن ينخفض المبلغ النهائي عن هذا الحد.';
  }

  @override
  String get walletSwapIntoZecShieldTitle => 'تستلم ZEC إلى عنوانك الخاص';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'إلى أن تحمي هذا المبلغ — بلمسة واحدة، ستُذكَّر بذلك عند الوصول — يكون المبلغ المستلم علنيًا ومرئيًا على السلسلة لفترة وجيزة. قد يبقى المبلغ الصغير علنيًا إلى أن يتراكم.';

  @override
  String get walletSwapRefundVerifyTitle => 'تحقّق من عنوان الاسترداد الخاص بك';

  @override
  String get walletSwapRefundVerifyBody =>
      'تحقّق منه حرفًا بحرف — فهذا هو المكان الذي تعود إليه عملاتك إذا فشلت المبادلة. لا يمكن للمحفظة التحقق من عنوان خارجي نيابة عنك.';

  @override
  String get walletSwapRefundVerifyAck =>
      'لقد تحقّقت من صحة عنوان الاسترداد الخاص بي.';

  @override
  String get walletSwapPayoutVerifyTitle => 'تحقّق من عنوان الاستلام الخاص بك';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'تحقّق منه حرفًا بحرف — فهذا هو المكان الذي ستستلم فيه $asset. لا يمكن للمحفظة التحقق من عنوان خارجي نيابة عنك.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'لقد تحقّقت من صحة عنوان الاستلام الخاص بي.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'المبادلة معطّلة هنا. أي ZEC في طريقه بالفعل سيظهر في محفظتك بعد مزامنتك التالية.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'أدخل المبلغ الذي تريد مبادلته.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'أدخل عنوان الاسترداد الخاص بك على سلسلة المصدر.';

  @override
  String get walletSwapDepositTitle => 'أرسل دفعتك';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'أرسل بالضبط $amount $asset على $chain إلى العنوان أدناه.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'أرسل المبلغ بالضبط. إرسال مبلغ أقل، أو الإرسال بعد إغلاق المهلة، يعني أن المزوّد سيُعيد إليك المبلغ عبر عنوان الاسترداد الخاص بك.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'مهلة الإيداع: تبقّى $time';
  }

  @override
  String get walletSwapDepositExpired =>
      'أُغلقت مهلة الإيداع هذه. لا ترسل أموالًا الآن — ابدأ مبادلة جديدة. إذا كنت قد أرسلت بالفعل، ينبغي أن يُعيد المزوّد المبلغ إلى عنوان الاسترداد الخاص بك.';

  @override
  String get walletSwapDepositQrLabel => 'رمز QR لعنوان الإيداع';

  @override
  String get walletSwapDepositAddressLabel => 'عنوان الإيداع';

  @override
  String get walletSwapDepositCopy => 'نسخ عنوان الإيداع';

  @override
  String get walletSwapDepositCopied => 'تم نسخ عنوان الإيداع';

  @override
  String get walletSwapDepositMemoRequired =>
      'يتطلّب هذا الإيداع مذكرة / علامة';

  @override
  String get walletSwapDepositMemoWarning =>
      'يجب عليك تضمين هذه المذكرة بالضبط مع إيداعك. الإرسال دونها — أو بمذكرة خاطئة — قد يؤدي إلى فقدان أموالك نهائيًا.';

  @override
  String get walletSwapDepositMemoLabel => 'مذكرة / علامة الإيداع';

  @override
  String get walletSwapDepositMemoCopy => 'نسخ المذكرة';

  @override
  String get walletSwapDepositMemoCopied => 'تم نسخ المذكرة';

  @override
  String get walletSwapDepositSent => 'لقد أرسلت الأموال';

  @override
  String get walletSwapDepositBackTitle => 'مغادرة هذه الشاشة؟';

  @override
  String get walletSwapDepositBackBody =>
      'هذا لن يُلغي مبادلتك — فهي تستمر في الخلفية. لكنك ستحتاج عنوان الإيداع للدفع، لذا انسخه أولًا إن لم تفعل ذلك بعد.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'هذا لن يُلغي مبادلتك — فهي تستمر في الخلفية. أُغلقت مهلة الإيداع. لا ترسل الأموال إلى عنوان الإيداع الآن. إذا كنت قد أرسلت بالفعل، ينبغي أن يُعيد المزوّد المبلغ إلى عنوان الاسترداد الخاص بك.';

  @override
  String get walletSwapDepositBackStay => 'البقاء';

  @override
  String get walletSwapDepositBackLeave => 'مغادرة';

  @override
  String get walletReceive => 'استلام';

  @override
  String get walletReceiveSubtitle =>
      'شارك هذا العنوان لاستلام ZEC. من الآمن مشاركته علنًا.';

  @override
  String get walletReceiveCopy => 'نسخ العنوان';

  @override
  String get walletReceiveCopied => 'تم نسخ العنوان';

  @override
  String get walletReceiveUnavailable => 'محفظتك ليست جاهزة بعد.';

  @override
  String get walletReceiveError => 'تعذّر تحميل عنوانك. يُرجى إعادة المحاولة.';

  @override
  String get walletReceivePreparing => 'جارٍ تحضير عنوانك…';

  @override
  String get walletReceivePreparingHint =>
      'محفظتك تُحضّر هذا العنوان على جهازك — قد يستغرق هذا لحظة إذا كانت المحفظة مشغولة بعمل آخر.';

  @override
  String get walletReceiveRetry => 'إعادة المحاولة';

  @override
  String get walletReceiveQrLabel => 'رمز QR لعنوان الاستلام الخاص بك';

  @override
  String get walletReceiveTypeShielded => 'محمي';

  @override
  String get walletReceiveTypeTransparent => 'علني';

  @override
  String get walletReceiveSubtitleTransparent =>
      'شارك هذا العنوان العلني لاستلام ZEC من مُرسل لا يستطيع الدفع إلى عنوان محمي.';

  @override
  String get walletReceiveTransparentWarning =>
      'هذا عنوان علني: فهو مرئي على السلسلة ويربط بين دفعاتك في حال إعادة استخدامه. يُفضَّل استخدام عنوانك المحمي؛ واحمِ هذه الأموال بعد استلامها.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'رمز QR لعنوان الاستلام العلني الخاص بك';

  @override
  String get walletReceiveFreshAddress => 'استخدام عنوان جديد';

  @override
  String get walletReceiveFreshCaption =>
      'عنوان جديد — لا يمكن ربطه بعناوينك الأخرى. المدفوعات إليه لا تزال تصل إلى هذه المحفظة، وتستمر عناوينك السابقة في العمل. لن يُعرض هنا مرة أخرى — انسخه الآن.';

  @override
  String get walletReceiveFreshError =>
      'تعذّر إنشاء عنوان جديد. حاول مرة أخرى.';

  @override
  String get walletReceiveFreshBusy =>
      'المحفظة مشغولة الآن. حاول الحصول على العنوان الجديد مرة أخرى بعد قليل.';

  @override
  String get walletReceiveShare => 'مشاركة';

  @override
  String get walletReceiveRequestAmount => 'طلب مبلغ';

  @override
  String get walletReceiveRequestAmountLabel => 'المبلغ (اختياري)';

  @override
  String get walletReceiveFreshCopyNow => 'لن يُعرض هنا مرة أخرى — انسخه الآن.';

  @override
  String get walletSecurityMenuItem => 'الأمان…';

  @override
  String get securityTitle => 'الأمان';

  @override
  String get securityUnavailableBody =>
      'تُدار إعدادات أمان المحفظة بواسطة هذا التطبيق، وليس بواسطة المحفظة نفسها.';

  @override
  String get securityCustodySectionTitle => 'حفظ المفاتيح';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (عتاد)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (عتاد)';

  @override
  String get securityCustodyTierTee => 'مخزن مفاتيح عتادي (TEE)';

  @override
  String get securityCustodyTierSoftware => 'مخزن مفاتيح برمجي';

  @override
  String get securityCustodyTierKeychain => 'Keychain (مُشفّر برمجيًا)';

  @override
  String get securityCustodyTierNone => 'لا يوجد مخزن مفاتيح عتادي';

  @override
  String get securityCustodyTierUnknown => 'غير معروف';

  @override
  String get securityCustodyHardwareKey =>
      'المفتاح الذي يقفل هذه المحفظة محفوظ في العتاد الآمن لهذا الجهاز، ويُحذف مع المحفظة.';

  @override
  String get securityCustodyBestEffort =>
      'يؤدي الحذف إلى إزالة مفاتيحك على أساس أفضل جهد؛ وقد تبقى نافذة قصيرة لإمكانية استرداد جنائي إلى أن يستعيد الجهاز مساحة التخزين. للحصول على ضمان كامل، استخدم أيضًا خيار مسح كل المحتوى في جهازك.';

  @override
  String get securityCustodyProbeError =>
      'تعذّرت قراءة حالة الحفظ. ارجع وأعد المحاولة.';

  @override
  String get securityDeleteWalletButton => 'حذف المحفظة';

  @override
  String get securityDeleteWalletSubtitle =>
      'احذف هذه المحفظة ومفتاحها من هذا الجهاز. تبقى أموالك على السلسلة ويمكن استعادتها من عبارة الاسترداد.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'احذف هذه المحفظة ومفتاحها من هذا الجهاز. لا تملك أي مفاتيح إنفاق، لذا لا يوجد شيء لنسخه احتياطيًا — يمكن إعادة إضافتها في أي وقت باستخدام مفتاح المشاهدة الخاص بها.';

  @override
  String get securityDeleteDialogTitle => 'حذف هذه المحفظة؟';

  @override
  String get securityDeleteDialogBody =>
      'سيؤدي هذا إلى إزالة المحفظة ومفتاحها من هذا الجهاز. تأكّد من أنك نسخت عبارة الاسترداد احتياطيًا — فهي الوسيلة الوحيدة لاستعادة أموالك.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'سيؤدي هذا إلى إزالة المحفظة ومفتاحها من هذا الجهاز. لا تملك أي مفاتيح إنفاق، لذا لا حاجة لنسخ أي شيء احتياطيًا — يمكنك إعادة إضافتها لاحقًا باستخدام مفتاح المشاهدة الخاص بها.';

  @override
  String get securityDeleteDialogConfirm => 'حذف';

  @override
  String get securityDeleteDialogCancel => 'إلغاء';

  @override
  String get securityDeleteFailedSnack =>
      'تعذّر حذف المحفظة — محفظتك دون تغيير. أعد المحاولة.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'أكمل تبديل الخادم أولًا — يكتمل أو يتوقف خلال $seconds ثانية. ثم حاول حذف المحفظة مرة أخرى.';
  }

  @override
  String get walletParkedTitle => 'محفوظة وقيد الانتظار';

  @override
  String get walletParkedSubtitle =>
      'هذه الدفعات لم تُرسل بعد. لا تزال مبالغها جزءًا من رصيدك.';

  @override
  String get walletParkedSubtitlePreparing =>
      'هذه الدفعات لم تُرسل بعد. لا تزال مبالغها جزءًا من رصيدك — باستثناء أي دفعة تُرسلها محفظتك حاليًا، فقد يكون مبلغها مخصّصًا بالفعل.';

  @override
  String get walletParkedCancel => 'إلغاء';

  @override
  String get walletParkedPausedHint =>
      'متوقفة مؤقتًا — لن تُرسل هذه الدفعة من تلقاء نفسها. أموالك آمنة. أرسلها الآن، أو ألغِها.';

  @override
  String get walletParkedRetryStale =>
      'لم تعد هذه الدفعة قيد الانتظار. تحقّق من دفعاتك قيد الانتظار ونشاطك.';

  @override
  String get walletParkedAlreadyInProgress =>
      'هذه الدفعة لم تعد قيد الانتظار — قد تكون محفظتك بصدد إرسالها بالفعل. تحقّق من \"محفوظة وقيد الانتظار\" ونشاطك.';

  @override
  String get walletReclaimExplainer =>
      'الإرسال عبر العناوين لمرة واحدة متوقف الآن. يمكنك إعادة فتحه — إذ ينقل ذلك مبلغًا صغيرًا بين عناوينك الخاصة ثم يعيده إليك.';

  @override
  String get walletReclaimButton => 'إعادة فتح الإرسال';

  @override
  String get walletReclaimInProgress => 'جارٍ إعادة الفتح…';

  @override
  String get walletReclaimConfirmTitle =>
      'إعادة فتح الإرسال عبر العناوين لمرة واحدة؟';

  @override
  String get walletReclaimConfirmBody =>
      'سينقل هذا مبلغًا صغيرًا بين عناوينك الخاصة لتحرير الإرسال عبر العناوين لمرة واحدة، ثم يعيده إليك. يكلّف هذا بضعة رسوم شبكة. بمجرد تأكيد ذلك، استرد المبلغ المنقول عبر \"استرداد الآن\".';

  @override
  String get walletReclaimConfirmCancel => 'ليس الآن';

  @override
  String get walletReclaimConfirmAction => 'إعادة الفتح';

  @override
  String get walletReclaimStarted =>
      'بدأت إعادة الفتح. بمجرد تأكيد ذلك، أرسل الدفعة المتوقفة مؤقتًا، ثم استخدم \"استرداد الآن\" لاسترداد المبلغ المنقول.';

  @override
  String get walletReclaimNothing => 'لا يوجد ما يمكن إعادة فتحه الآن.';

  @override
  String get walletReclaimNotBroadcast =>
      'تعذّر التأكد من وصول البث إلى الشبكة. قد يكون قد وصل رغم ذلك. أعد المحاولة بعد قليل.';

  @override
  String get walletReclaimNeedsFunds =>
      'تحتاج إلى بعض ZEC المحمي لإعادة فتح الإرسال.';

  @override
  String get walletReclaimFailed =>
      'تعذّر إعادة فتح الإرسال الآن. أموالك دون تغيير. أعد المحاولة.';

  @override
  String get walletReclaimUnknown =>
      'انتهت إعادة الفتح. تحقّق من إرسالاتك عبر العناوين لمرة واحدة، واستخدم \"استرداد الآن\" لاسترداد أي مبلغ منقول.';

  @override
  String get walletParkedError => 'تعذّر تحميل دفعاتك قيد الانتظار الآن.';

  @override
  String get walletParkedErrorRetry => 'إعادة المحاولة';

  @override
  String get walletParkedErrorRetryInProgress => 'جارٍ المحاولة…';

  @override
  String get walletParkedCancelConfirmTitle => 'إلغاء هذه الدفعة قيد الانتظار؟';

  @override
  String get walletParkedCancelConfirmBody =>
      'سيؤدي هذا إلى التخلص من الدفعة المحفوظة. لم تُرسل بعد، لذا لن تُخصم أي أموال من محفظتك — لكن لا يمكن التراجع عن هذا الإجراء.';

  @override
  String get walletParkedCancelConfirmKeep => 'الاحتفاظ بها';

  @override
  String get walletParkedCancelConfirmDiscard => 'التخلص من الدفعة';

  @override
  String get walletParkedCancelDone => 'تم إلغاء الدفعة قيد الانتظار.';

  @override
  String get walletParkedCancelAlreadySending =>
      'قد تكون هذه الدفعة في طريقها بالفعل — تحقّق من نشاطك.';

  @override
  String get walletParkedCancelFailed =>
      'تعذّر الإلغاء الآن. دفعتك دون تغيير. أعد المحاولة.';

  @override
  String get walletRecoverNow => 'استرداد الآن';

  @override
  String get walletRecoverConfirmTitle => 'الاسترداد إلى رصيدك المحمي؟';

  @override
  String get walletRecoverConfirmBody =>
      'سيتحقّق هذا من عناوينك لمرة واحدة وينقل أي أموال يعثر عليها إلى رصيدك المحمي والخاص. من الآمن تشغيله مرة أخرى في أي وقت.';

  @override
  String get walletRecoverConfirmCancel => 'ليس الآن';

  @override
  String get walletRecoverConfirmAction => 'استرداد';

  @override
  String get walletRecoverInProgress => 'جارٍ الاسترداد…';

  @override
  String walletRecoverDone(String amount) {
    return 'جارٍ استرداد $amount إلى رصيدك المحمي.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'جارٍ استرداد $amount — لا تزال بعض الأموال بحاجة إلى محاولة أخرى.';
  }

  @override
  String get walletRecoverRetry =>
      'بعض الأموال بحاجة إلى محاولة أخرى — شغّل الاسترداد مرة أخرى.';

  @override
  String get walletRecoverTruncated =>
      'لم يُتحقق من كل العناوين لمرة واحدة بعد — شغّله مرة أخرى للتحقق من الباقي.';

  @override
  String get walletRecoverNothing => 'لا يوجد ما يمكن استرداده الآن.';

  @override
  String get walletRecoverFailed =>
      'تعذّر الاسترداد الآن. أموالك دون تغيير. أعد المحاولة.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount محفوظ وقيد الانتظار · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'إلغاء دفعة $amount المحفوظة $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount متوقف مؤقتًا · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount قيد التحضير للإرسال · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'محفظتك تُحضّر هذه الدفعة للإرسال — وقد يكون مبلغها مخصّصًا بالفعل. أموالك آمنة. إذا لم تكتمل، ستعود إلى القائمة من تلقاء نفسها.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'محفظتك تُحضّر هذه الدفعة للإرسال — وقد يكون مبلغها مخصّصًا بالفعل. أموالك آمنة، لكن لا يمكن إتمامها إلا بعد أن تتزامن محفظتك من جديد.';

  @override
  String get walletParkedSendNow => 'إرسال الآن';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'جارٍ إرسال دفعة $amount المحفوظة $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'إرسال دفعة $amount المحفوظة $time الآن';
  }

  @override
  String get walletParkedSendNowInProgress => 'جارٍ الإرسال…';

  @override
  String get walletParkedAuthorizeSent => 'جارٍ إرسال دفعتك الآن.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'جارٍ إرسال دفعتك الآن. إذا لم تصل، فلن تتمكن محفظتك من إتمام الدفعة إلا بعد أن تتزامن من جديد.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'لم يحن وقت الإرسال بعد. دفعتك محفوظة ولم تتغيّر.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'لم يحن وقت الإرسال بعد. دفعتك محفوظة ولم تعد متوقفة مؤقتًا — حاول \"إرسال الآن\" مرة أخرى لاحقًا، أو ألغِها.';

  @override
  String get walletParkedAuthorizeFailed =>
      'تعذّر الإرسال الآن. دفعتك دون تغيير. أعد المحاولة.';

  @override
  String get walletTransparentFundsMenuItem => 'الأموال العلنية…';

  @override
  String get walletTransparentFundsTitle => 'الأموال العلنية';

  @override
  String get walletTransparentFundsIntro =>
      'الأموال العلنية مرئية علنًا على سلسلة الكتل — المبلغ والعناوين وتاريخ العملات.';

  @override
  String get walletExpertToggleLabel => 'متقدم: الأموال العلنية';

  @override
  String get walletExpertToggleDescription =>
      'إظهار أدوات تحكم متقدمة للاحتفاظ بالأموال العلنية وإيقاف الحماية التلقائية.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'إظهار أدوات تحكم متقدمة للاحتفاظ بالأموال العلنية.';

  @override
  String get walletAutoShieldToggleLabel => 'الحماية التلقائية';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'عندما يصل رصيدك العلني إلى $minZec ZEC، يُنقل تلقائيًا إلى رصيدك المحمي. عند إيقاف هذا الخيار، تبقى الأموال العلنية مرئية علنًا إلى أن تحميها بنفسك.';
  }

  @override
  String get walletSettingsSaveFailed => 'تعذّر حفظ الإعداد. أعد المحاولة.';

  @override
  String get walletAutoShieldIncomplete =>
      'لم تكتمل الحماية التلقائية — لا تزال هذه الأموال مرئية علنًا. يمكنك حمايتها الآن.';

  @override
  String get walletSendPrivacyShielded =>
      'دفعة محمية — يبقى المبلغ والمستلم خاصَّين على السلسلة.';

  @override
  String get walletSendPrivacyTransparent =>
      'دفعة علنية — المبلغ والعناوين مرئية على سلسلة الكتل.';

  @override
  String get walletActivityPublicBadge => 'مرئية علنًا على السلسلة';

  @override
  String get walletShieldWalletEnded =>
      'انتهت جلسة المحفظة. أغلقها وأعد فتحها للمحاولة مرة أخرى.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'تُنقل الأموال العلنية الجديدة تلقائيًا إلى رصيدك المحمي بمجرد وصولها إلى $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'الحماية التلقائية متوقفة — تبقى الأموال العلنية مرئية علنًا إلى أن تحميها.';

  @override
  String get walletMoveAutoShieldNote =>
      'الحماية التلقائية مفعّلة: بعد وصول هذه الأموال، ستُحمى تلقائيًا مرة أخرى (مقابل رسوم إضافية). للاحتفاظ بها علنية، أوقف أولاً الحماية التلقائية ضمن الأموال العلنية.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'بعد هذا النقل سيصبح رصيدك العلني $amount ZEC — أقل من $floor ZEC اللازمة لحمايته مرة أخرى. سيبقى علنيًا حتى تصل أموال إضافية.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'أنت تنقل الأموال إلى عنوانك العلني الخاص. هذا النقل يبقى في السجل العلني بشكل دائم.';

  @override
  String get walletTxDetailVisibility => 'الرؤية';

  @override
  String get walletTransparentFundsAutoDenied =>
      'الحماية التلقائية متوقفة مؤقتًا لهذه الجلسة — لم تتم الموافقة عليها. لا يزال بإمكانك حماية أموالك يدويًا.';

  @override
  String get walletDeepScanMenuItem => 'تحقّق من عناوين المبادلة الأقدم…';

  @override
  String get walletMenuSyncNotRunningHint => 'المزامنة لا تعمل الآن.';

  @override
  String get walletDeepScanTitle => 'تحقّق من عناوين المبادلة الأقدم';

  @override
  String get walletDeepScanBody =>
      'إذا استعدت هذه المحفظة وكانت تستخدم المبادلة كثيرًا في السابق، فقد تحتاج الأموال من أقدم عمليات المبادلة فيها إلى خطوة إضافية للعثور عليها. هذا الفحص يتحقق من ذلك — وأي أموال يُعثر عليها تظهر في رصيدك أثناء مزامنة محفظتك.';

  @override
  String get walletDeepScanCoverage =>
      'تم التحقق من عناوين المبادلة الأقدم حتى هذا الحد. إذا كانت أموال من مبادلة قديمة لا تزال مفقودة، تحقّق حتى عناوين أقدم.';

  @override
  String get walletDeepScanCoveragePending =>
      'لا يزال التحقق من النطاق الحالي جاريًا — وأي أموال يُعثر عليها ستظهر في رصيدك. قد يستغرق ذلك بعض الوقت.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'يتحقق من وجود أموال من أقدم عمليات المبادلة في محفظتك.';

  @override
  String get walletDeepScanCheckButton => 'تحقّق من العناوين الأقدم';

  @override
  String get walletDeepScanCheckDeeperButton => 'تحقّق من عناوين أقدم حتى';

  @override
  String get walletDeepScanChecking => 'جارٍ التحقق…';

  @override
  String get walletDeepScanClose => 'إغلاق';

  @override
  String get walletDeepScanTorHint =>
      'أنت غير متصل عبر Tor حاليًا. لمزيد من الخصوصية، ننصحك بالانتظار حتى يصبح Tor نشطًا قبل التحقق.';

  @override
  String get walletDeepScanRescanBusy =>
      'يمكنك التحقق من عناوين المبادلة الأقدم بمجرد انتهاء إعادة الفحص.';

  @override
  String get walletDeepScanRan =>
      'جارٍ التحقق من عناوين المبادلة الأقدم — وأي أموال يُعثر عليها ستظهر في رصيدك.';

  @override
  String get walletDeepScanFailed =>
      'تعذّر بدء التحقق. لم يتغير شيء — يُرجى المحاولة مرة أخرى.';

  @override
  String get walletDeepScanSlow =>
      'يستغرق هذا وقتًا أطول من المعتاد. إذا تم التحقق من عناوين المبادلة الأقدم لديك، فأي أموال يُعثر عليها ستظهر في رصيدك — تحقّق مرة أخرى بعد قليل.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'المبادلة مُعطّلة حاليًا، لذا لا يمكن تشغيل هذا. يُرجى المحاولة مرة أخرى عندما تصبح المبادلة متاحة.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'لا يزال التحقق من النطاق الأخير جاريًا — قد يستغرق ذلك حتى يومين، لكن عادةً أقل من ذلك بكثير. ينتهي ذلك تلقائيًا؛ تحقّق مرة أخرى لاحقًا.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'لا يمكننا التحقق من خصوصية اتصالك حتى الآن. لمزيد من الخصوصية، ننصحك بالتحقق بمجرد أن يصبح Tor نشطًا.';

  @override
  String get walletDeepScanBannerChecking =>
      'لا يزال التحقق من عناوين المبادلة الأقدم جاريًا — وأي أموال يُعثر عليها ستظهر في رصيدك.';

  @override
  String get walletRescanSwapPointer =>
      'هل تبحث عن أموال من مبادلة قديمة؟ إعادة الفحص لن تجدها — استخدم ميزة “تحقّق من عناوين المبادلة الأقدم” بدلًا من ذلك.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'هل استعدت محفظة كانت تستخدم المبادلة؟';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'إذا كان لهذه المحفظة سجل مبادلة طويل جدًا، فقد تحتاج الأموال من أقدم عمليات المبادلة فيها إلى خطوة إضافية للعثور عليها. معظم المحافظ لا تحتاج إلى أي إجراء.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'تحقّق الآن';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'تجاهل';

  @override
  String walletTorHostPath(String transport) {
    return 'عبر المسار الخاص لتطبيقك ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'عبر المسار الخاص لتطبيقك ($transport)؛ يمكن للوكيل ربط الاتصالات ببعضها';
  }

  @override
  String get walletTorHostOtherTransport => 'مسار خاص';

  @override
  String get walletTorHostDirect => 'غير خاص (اتصال تطبيقك المباشر)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'يستخدم الخادم المحفوظ عنوانًا غير مشفّر لا يمكن للمسار الخاص لتطبيقك نقله. يتم استخدام $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'المزيد عن $label';
  }

  @override
  String get walletSendPaste => 'لصق';

  @override
  String get walletSendScanQr => 'مسح رمز QR';

  @override
  String get walletSendRecipientGetsLabel => 'يستلم المستلم';

  @override
  String get walletSwapDepositCopyAmount => 'نسخ المبلغ';

  @override
  String get walletSwapDepositAmountCopied => 'تم نسخ المبلغ';

  @override
  String get walletScanOpenSettings => 'فتح الإعدادات';

  @override
  String get walletScanOpenSettingsFailed => 'تعذّر فتح الإعدادات.';

  @override
  String get walletSendLeaveTitle => 'ما زال الإرسال جارياً';

  @override
  String get walletSendLeaveBody =>
      'تستمر دفعتك إذا غادرت. سترى كيف انتهت في نشاطك.';

  @override
  String get walletSendLeaveStay => 'البقاء';

  @override
  String get walletSendLeaveConfirm => 'المغادرة';

  @override
  String get walletSheetLeaveBody =>
      'تستمر هذه العملية إذا غادرت. سترى كيف انتهت في نشاطك.';

  @override
  String get walletLoadingLabel => 'جارٍ التحميل';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'تحقّق قبل الحماية مرة أخرى';

  @override
  String get walletShieldUnknownBody =>
      'تعذّر علينا تأكيد عملية الحماية هذه. راجع «النشاط» قبل المحاولة مرة أخرى.';

  @override
  String get walletMoveUnknownTitle => 'تحقّق قبل النقل مرة أخرى';

  @override
  String get walletMoveUnknownBody =>
      'تعذّر علينا تأكيد عملية النقل هذه. راجع «النشاط» قبل المحاولة مرة أخرى.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
