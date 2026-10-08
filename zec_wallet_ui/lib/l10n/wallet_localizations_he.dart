// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Hebrew (`he`).
class WalletLocalizationsHe extends WalletLocalizations {
  WalletLocalizationsHe([String locale = 'he']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'הגדרות';

  @override
  String get walletTitle => 'ארנק';

  @override
  String get walletNotSetUpTitle => 'הארנק עדיין לא הוגדר';

  @override
  String get walletNotSetUpBody =>
      'הגדרת הארנק תגיע בגרסה עתידית. ההגדרה תדריך אותך לכתוב את ביטוי השחזור שלך לפני שניתן לקבל כספים — כך שדבר אינו בסיכון ללא גיבוי.';

  @override
  String get walletStartupFailedTitle => 'לא ניתן היה להפעיל את הארנק';

  @override
  String get walletStartupFailedBody =>
      'משהו מנע מהארנק להיטען במכשיר זה. אם כבר יש לך ארנק, הכספים שבו אינם נפגעים — הם נמצאים ברשת Zcash וניתן לשחזר אותם באמצעות ביטוי השחזור שלך. נסה שוב; אם זה ממשיך לקרות, סגור את האפליקציה ופתח אותה מחדש.';

  @override
  String get walletBalanceLabel => 'יתרה';

  @override
  String get walletHideBalance => 'הסתרת היתרה';

  @override
  String get walletShowBalance => 'הצגת היתרה';

  @override
  String get walletBalanceHiddenAmount => 'היתרה מוסתרת';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'ניתן לשימוש כעת';

  @override
  String get walletArrivingLabel => 'נכנס';

  @override
  String get walletNotSpendableYetLabel => 'עדיין לא ניתן להוציא';

  @override
  String get walletActivityTitle => 'פעילות';

  @override
  String get walletActivityEmpty => 'אין עדיין פעילות';

  @override
  String get walletActivityError => 'טעינת הפעילות נכשלה';

  @override
  String get walletActivityReceived => 'התקבל';

  @override
  String get walletActivitySent => 'נשלח';

  @override
  String get walletActivityPending => 'בהמתנה';

  @override
  String get walletActivityQueued => 'בתור';

  @override
  String get walletActivityRetrying => 'מנסה שוב';

  @override
  String get walletActivitySaved => 'נשמר';

  @override
  String get walletActivityExpired => 'פג תוקף';

  @override
  String get walletActivityFailed => 'נכשל';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count אישורים',
      one: 'אישור אחד',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count תשלומים התקבלו',
      one: 'תשלום התקבל',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'הצג פרטי עסקה';

  @override
  String get walletTxDetailStatus => 'סטטוס';

  @override
  String get walletTxDetailFee => 'עמלת רשת';

  @override
  String get walletTxDetailDate => 'תאריך';

  @override
  String get walletTxDetailHeight => 'גובה הבלוק';

  @override
  String get walletTxDetailMemo => 'הערה';

  @override
  String get walletTxDetailMemoAttached => 'מצורפת';

  @override
  String get walletTxDetailTxid => 'מזהה עסקה';

  @override
  String get walletTxDetailCopyTxid => 'העתק מזהה עסקה';

  @override
  String get walletTxDetailCopied => 'מזהה העסקה הועתק';

  @override
  String get walletTxDetailClose => 'סגור';

  @override
  String get walletTxFundsKept => 'שום כסף לא יצא מהארנק שלך';

  @override
  String get walletTxExplainQueued =>
      'נשמר במכשיר זה, תחת \"נשמר וממתין\" — תוכל לשלוח אותו או לבטל אותו משם.';

  @override
  String get walletTxExplainPending =>
      'נשלח לרשת Zcash — ממתין לאישור בתוך בלוק.';

  @override
  String get walletTxExplainRetrying =>
      'הארנק שלך עדיין לא הצליח לשלוח זאת לרשת Zcash. הוא שומר את העסקה החתומה ומנסה שוב בכל סנכרון עד שהיא עוברת או שתוקפה פג.';

  @override
  String get walletTxExplainSaved =>
      'הארנק שלך שמר את העסקה החתומה הזו, אך אינו שולח אותה בעצמו כרגע.';

  @override
  String get walletTxExplainConfirmed => 'אושר ברשת Zcash.';

  @override
  String get walletTxExplainExpired =>
      'תוקף העסקה פג לפני שאושרה על ידי הרשת, ולכן היא בוטלה. הסכום עדיין שלך וזמין לשימוש.';

  @override
  String get walletTxExplainFailed =>
      'הרשת דחתה את העסקה, ולכן היא לא בוצעה. הסכום עדיין שלך וזמין לשימוש.';

  @override
  String get walletTxExplainUnknown =>
      'לא ניתן לקבוע כרגע את סטטוס העסקה. הוא יתעדכן לאחר הסנכרון הבא.';

  @override
  String get walletMenuTooltip => 'עוד אפשרויות';

  @override
  String get walletRescanMenuItem => 'סריקה מחדש של ההיסטוריה…';

  @override
  String get walletCheckOneTimeMenuItem => 'בדיקת כתובות חד-פעמיות…';

  @override
  String get walletRescanTitle => 'סריקה מחדש של ההיסטוריה שלך';

  @override
  String get walletRescanBody =>
      'חסרים כספים ישנים? סרוק מחדש את הבלוקצ\'יין מנקודה מוקדמת יותר כדי לשחזר הפקדות שדולגו בשל תאריך התחלה מאוחר מדי. הכספים וביטוי השחזור שלך לעולם אינם בסיכון.';

  @override
  String get walletRescanRangeTitle => 'עד כמה רחוק לסרוק אחורה';

  @override
  String get walletRescanRangeAll =>
      'סרוק את כל ההיסטוריה שלך — האפשרות האיטית ביותר, אך משחזרת הכול.';

  @override
  String get walletRescanRangeDefault =>
      'סורק החל מתחילת הארנק שלך. עדיין חסרים כספים ישנים יותר? בחר תאריך מוקדם יותר, או סרוק את כל ההיסטוריה.';

  @override
  String get walletRescanRangeResolving => 'מכין את הטווח המומלץ…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'כ-$blocks בלוקים לסריקה.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'סורק החל מ-$date. עדיין חסרים כספים ישנים יותר? בחר תאריך מוקדם יותר, או סרוק את כל ההיסטוריה.';
  }

  @override
  String get walletRescanPick => 'בחר תאריך';

  @override
  String get walletRescanChange => 'שנה תאריך';

  @override
  String get walletRescanScanAll => 'סרוק את כל ההיסטוריה';

  @override
  String get walletRescanDatePick => 'התאריך המוקדם ביותר לסריקה';

  @override
  String get walletRescanWarning =>
      'פעולה זו סורקת מחדש את הבלוקצ\'יין. תאריכים קרובים ייקחו דקות; סריקה רחוקה אחורה עשויה לקחת שעות. הסנכרון פועל ברקע — ניתן להמשיך להשתמש בארנק.';

  @override
  String get walletRescanSettlingAdvisory =>
      'תשלום מארנק זה עדיין נמצא בתהליך אישור. הארנק בדרך כלל מסרב לבצע סריקה מחדש עד שהתהליך יושלם — אפשר לנסות, אך יש לצפות לסירוב.';

  @override
  String get walletRescanConfirm => 'התחל סריקה מחדש';

  @override
  String get walletRescanCancel => 'ביטול';

  @override
  String get walletRescanRunning => 'בונה מחדש…';

  @override
  String get walletRescanRebuildingAll =>
      'בונה מחדש את ההיסטוריה שלך — סורק את כל השרשרת. היתרה והפעילות שלך יתמלאו בהדרגה עם התקדמות התהליך.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'בונה מחדש את ההיסטוריה שלך החל מ-$date — היתרה והפעילות שלך יתמלאו בהדרגה עם התקדמות התהליך.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'בונה מחדש את ההיסטוריה שלך החל מתחילת הארנק שלך — היתרה והפעילות שלך יתמלאו בהדרגה עם התקדמות התהליך.';

  @override
  String get walletCatchUpBanner =>
      'משלים פערים — היתרה והפעילות שלך יתמלאו בהדרגה תוך כדי סנכרון הארנק. כל מה שקיבלת בטוח.';

  @override
  String get walletCatchUpRescanBanner =>
      'בונה מחדש את ההיסטוריה שלך אחרי סריקה מחדש — היתרה והפעילות שלך יתמלאו בהדרגה עם התקדמות התהליך. כל מה שקיבלת בטוח.';

  @override
  String get walletRescanFailedNotice =>
      'לא ניתן היה לסרוק מחדש כרגע — הכספים שלך בטוחים, אך ייתכן שהיתרה וההיסטוריה שלך יזדקקו לזמן מה כדי להתעדכן. נסה שוב בעוד רגע.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'תשלום עדיין נמצא בתהליך אישור, ולכן הסריקה מחדש הושהתה כדי להגן על הכספים שלך. הארנק שלך לא השתנה — נסה שוב בעוד כשעתיים והשאר את האפליקציה פתוחה ומחוברת לאינטרנט.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'הסריקה מחדש בונה מחדש את ההיסטוריה שלך תוך כדי סנכרון הארנק שלך, והסנכרון לא פועל כרגע. הארנק שלך לא השתנה — נסה שוב ברגע שהסנכרון יפעל.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'אין מספיק מקום פנוי כדי לבנות מחדש את היסטוריית הארנק שלך — הכספים שלך בטוחים, אך ייתכן שהיתרה וההיסטוריה שלך יזדקקו לזמן מה כדי להתעדכן. פנה מקום ונסה שוב.';

  @override
  String get walletRescanFailedDismiss => 'סגור';

  @override
  String get walletActivityRebuilding => 'בונה מחדש את ההיסטוריה שלך…';

  @override
  String get walletActivityCatchingUp =>
      'עדיין משלים פערים — כל מה שקיבלת יופיע כאן.';

  @override
  String get walletActivitySyncNotRunning =>
      'טעינת היתרה וההיסטוריה שלך תושלם ברגע שהסנכרון יפעל.';

  @override
  String get walletActivityLoadMore => 'טען עוד';

  @override
  String get walletPendingChangeLabel => 'עודף בהמתנה';

  @override
  String get walletTransparentLabel => 'לא מוגן (ציבורי)';

  @override
  String get walletTransparentNote =>
      'לא כלול ב-\"ניתן לשימוש כעת\" — הגן על כספים אלה כדי שתוכל להשתמש בהם. עד אז הם נשארים גלויים לציבור בבלוקצ\'יין.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'כספים אלה גלויים לציבור בבלוקצ\'יין.';

  @override
  String walletPoolShielded(String amount) {
    return 'מוגן $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'ציבורי $amount';
  }

  @override
  String get walletPoolAllShielded => 'הכול מוגן · פרטי';

  @override
  String get walletPoolTapHint => 'הצג כספים ציבוריים';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount מהיתרה שלך נמצא בכתובת חד-פעמית (ניתן לשחזור).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount מהיתרה שלך נמצא בכתובת חד-פעמית.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'תשלומים בסך כולל של $amount משוריינים ועדיין מושלמים דרך כתובות חד-פעמיות שבשליטת הארנק שלך. אל תשלח אותם שוב.',
      one:
          '$amount משוריין לתשלום שהארנק שלך עדיין משלים דרך כתובת חד-פעמית שבשליטתו. אל תשלח אותו שוב.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'תשלומים בסך כולל של $amount משוריינים ונמצאים באמצע התהליך דרך כתובות חד-פעמיות שבשליטת הארנק שלך. הם מושהים עד שהארנק שלך יסתנכרן שוב. אל תשלח אותם שוב.',
      one:
          '$amount משוריין לתשלום שנמצא באמצע התהליך דרך כתובת חד-פעמית שבשליטת הארנק שלך. הוא מושהה עד שהארנק שלך יסתנכרן שוב. אל תשלח אותו שוב.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'לא ניתן היה לבדוק אם תשלום עדיין מושלם. מנסה שוב — עד אז, חפש תשלום ממתין בפעילות שלך לפני שתשלח שוב.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount מהיתרה שלך נמצא בכתובת חד-פעמית (עדיין באישור).';
  }

  @override
  String get walletShieldButton => 'הגן';

  @override
  String get walletShieldSheetTitle => 'הגנה על כספים ציבוריים';

  @override
  String get walletShieldNote =>
      'פעולה זו מעבירה כספים מהיתרה הציבורית והגלויה שלך בבלוקצ\'יין אל היתרה המוגנת והפרטית שלך.';

  @override
  String get walletShieldPreparing => 'מכין…';

  @override
  String get walletShieldAmountLabel => 'בהגנה';

  @override
  String get walletShieldFeeLabel => 'עמלת רשת';

  @override
  String get walletShieldNetLabel => 'מגיע מוגן';

  @override
  String get walletShieldConfirmButton => 'הגן עכשיו';

  @override
  String get walletShieldSubmitting => 'מגן…';

  @override
  String get walletShieldNothingTitle => 'אין עדיין מה להגן';

  @override
  String get walletShieldNothingBody =>
      'כספים אלה נמוכים מהסכום שכדאי להגן עליו כרגע — עמלת הרשת תעלה על התועלת. הם יהיו ניתנים להגנה ברגע שיצטרף אליהם עוד קצת.';

  @override
  String get walletShieldDoneTitle => 'בקשת ההגנה נשלחה';

  @override
  String get walletShieldDoneBody =>
      'הכספים שלך עוברים אל היתרה המוגנת שלך. האישור בשרשרת יתקבל בקרוב.';

  @override
  String get walletShieldSavedTitle => 'נשמר — נשלים את ההגנה';

  @override
  String get walletShieldSavedBody =>
      'לא הצלחנו להגיע לרשת כרגע. פעולת ההגנה נשמרה והארנק שלך ישלים אותה בסנכרון מאוחר יותר. שום דבר לא אבד.';

  @override
  String get walletShieldAlreadyTitle => 'כבר נשלח';

  @override
  String get walletShieldFailedTitle => 'לא ניתן להגן כרגע';

  @override
  String get walletShieldStaleBody =>
      'הארנק עדיין מסתנכרן. נסה להגן שוב בעוד רגע.';

  @override
  String get walletShieldTransientBody =>
      'לא ניתן היה להכין את המיגון כרגע. נסו שוב בעוד רגע.';

  @override
  String get walletShieldStorageFullBody =>
      'אין מספיק מקום פנוי כדי להגן כרגע. פנה מקום ונסה שוב. הכספים שלך בטוחים.';

  @override
  String get walletShieldClose => 'סגור';

  @override
  String get walletShieldRetry => 'נסה שוב';

  @override
  String get walletMoveMenuItem => 'העברה לציבורי…';

  @override
  String get walletMoveSheetTitle => 'העברה לציבורי';

  @override
  String get walletMoveSheetSubtitle =>
      'שלח ZEC מוגן לכתובת הציבורית שלך — שימושי עבור בורסה שאינה מקבלת הפקדה מוגנת.';

  @override
  String get walletMoveDestinationLabel => 'הכתובת הציבורית שלך';

  @override
  String walletMoveAvailable(String amount) {
    return 'זמין להעברה: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'זמין להעברה: $amount ZEC — היתרה שלך עדיין משלימה פערים';
  }

  @override
  String get walletMoveDeshieldTitle => 'העברה זו הופכת את הכספים שלך לפומביים';

  @override
  String get walletMoveDeshieldBody =>
      'העברה לכתובת ציבורית מוציאה כספים אלה מהיתרה המוגנת שלך — הסכום והכתובת הציבורית שלך יהיו גלויים לציבור בבלוקצ\'יין של Zcash.';

  @override
  String get walletMoveWalletEnded =>
      'פעילות הארנק הסתיימה. סגור ופתח מחדש כדי לנסות שוב.';

  @override
  String get walletMoveLoading => 'מכין…';

  @override
  String get walletMovePreparing => 'בודק את הסכום…';

  @override
  String get walletMoveSubmitting => 'מעביר…';

  @override
  String get walletMoveReviewButton => 'סקירה';

  @override
  String get walletMoveCancel => 'ביטול';

  @override
  String get walletMoveReviewTitle => 'סקירת ההעברה';

  @override
  String get walletMoveOwnAddressNote =>
      'אתה מעביר לכתובת הציבורית שלך עצמך. תוכל להגן על כספים אלה שוב בעתיד, אך ההעברה הזו נשארת ברישום הפומבי לצמיתות.';

  @override
  String get walletMoveConfirmButton => 'העבר לציבורי';

  @override
  String get walletMoveBackButton => 'חזרה';

  @override
  String get walletMoveDoneTitle => 'הועבר לציבורי';

  @override
  String get walletMoveDoneBody =>
      'הכספים שלך עוברים לכתובת הציבורית שלך. האישור בשרשרת יתקבל בקרוב.';

  @override
  String get walletMoveSavedTitle => 'נשמר — נשלים את ההעברה';

  @override
  String get walletMoveSavedBody =>
      'ההעברה הזו נשמרה והארנק שלך ישלח אותה בסנכרון מאוחר יותר. שום דבר לא אבד.';

  @override
  String get walletMoveAlreadyTitle => 'כבר נשלח';

  @override
  String get walletMoveAlreadyBody =>
      'כספים אלה כבר נשלחו והם בדרכם לכתובת הציבורית שלך.';

  @override
  String get walletMoveFailedTitle => 'לא ניתן להשלים העברה זו';

  @override
  String get walletMoveNothingTitle => 'אין עדיין מה להעביר';

  @override
  String get walletMoveNothingBody =>
      'אין לך כרגע יתרה מוגנת זמינה להעברה. ברגע שכספים יאושרו, תוכל להעביר אותם לכתובת הציבורית שלך.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'הארנק שלך עדיין משלים פערים — כל מה שקיבלת יהפוך לזמין להעברה לאחר השלמת הסנכרון.';

  @override
  String get walletMoveCouldNotLoad =>
      'לא ניתן היה לטעון את הכתובת הציבורית שלך. נסה שוב.';

  @override
  String get walletMoveRetry => 'נסה שוב';

  @override
  String get walletMoveClose => 'סגור';

  @override
  String get walletSnapshotUnavailable =>
      'לא ניתן היה לקרוא את הארנק כרגע. הוא יתרענן באופן עצמאי.';

  @override
  String get walletBalanceStale =>
      'לא ניתן היה לרענן — מוצגת היתרה האחרונה הידועה.';

  @override
  String get walletSyncStartFailed =>
      'לא ניתן היה להתחיל בסנכרון. נמשיך לנסות.';

  @override
  String get walletSyncRetry => 'נסה שוב';

  @override
  String get walletSyncTryNow => 'נסה עכשיו';

  @override
  String get walletSyncIdle => 'טרם החל סנכרון';

  @override
  String get walletSyncIdleDetail => 'הסנכרון מתחיל באופן אוטומטי.';

  @override
  String get walletSyncDisabled => 'הסנכרון כבוי';

  @override
  String get walletSyncDisabledDetail =>
      'הפעל את הסנכרון בהגדרות של אפליקציה זו כדי לעדכן את היתרה שלך.';

  @override
  String get walletSyncExplainDisabled =>
      'הסנכרון כבוי בהגדרות של אפליקציה זו. הכספים שלך בטוחים. היתרה והפעילות שלך מוצגות לפי המצב הסינכרוני האחרון ולא יתעדכנו עד שהסנכרון יופעל.';

  @override
  String get walletParkedSyncPausedNote =>
      'הארנק שלך לא מסתנכרן, כך שאלה לא יישלחו מעצמם. השתמש ב\"שלח עכשיו\" כדי לשלוח אחד בעצמך.';

  @override
  String get walletSyncPausedMoneyNote => 'מושהה עד שהארנק שלך יסתנכרן שוב.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'מתחבר…';

  @override
  String get walletSyncStartingDetail => 'מתחבר לרשת Zcash ומתכונן לסריקה.';

  @override
  String get walletSyncConnecting => 'מתחבר…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'מתחבר… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'סורק $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'סורק…';

  @override
  String get walletSyncSpendableReady => 'הכספים מוכנים לשימוש.';

  @override
  String get walletSyncCatchingUp =>
      'משלים פערים מול הרשת — סנכרון ראשוני מעמיק עשוי לקחת זמן. אפשר להמשיך להשתמש באפליקציה עד לסיום';

  @override
  String walletSyncScanRemaining(String count) {
    return 'נותרו $count בלוקים';
  }

  @override
  String get walletSyncUpToDate => 'מעודכן';

  @override
  String get walletSyncOffline => 'לא מקוון';

  @override
  String get walletSyncOfflineDetail =>
      'שליחות בתור נשארות שמורות תחת \"נשמר וממתין\".';

  @override
  String get walletSyncUnknown => 'מסתנכרן…';

  @override
  String get walletSyncStalled => 'הסנכרון מושהה';

  @override
  String get walletStallEndpoint =>
      'לא ניתן להגיע לרשת Zcash כרגע. נמשיך לנסות באופן אוטומטי — בדוק את החיבור שלך לאינטרנט, או שהשרת עשוי להיות לא זמין באופן זמני.';

  @override
  String get walletStallTor =>
      'הנתיב הפרטי של האפליקציה שלך אינו זמין, ולכן הארנק לא מתחבר. בדוק את הגדרות הרשת באפליקציה שלך, או כבה את הנתיב הפרטי. הסנכרון יתחדש ברגע שהנתיב יחזור.';

  @override
  String get walletStallStorage => 'אחסון המכשיר מלא. פנה מקום והסנכרון יתחדש.';

  @override
  String get walletStallReorg =>
      'השרשרת עברה ארגון מחדש; בודק שוב את הבלוקים האחרונים.';

  @override
  String get walletStallInternal =>
      'בעיה מקומית עצרה את הסנכרון. אם זה נמשך, שחזר מביטוי השחזור שלך.';

  @override
  String get walletStallEndpointMisbehaving =>
      'השרת הזה שלח נתונים שלא יכולים להיות נכונים, ולכן הסנכרון נעצר. זו לא בעיית חיבור — עברו לשרת אחר. אם כל השרתים נדחים, סרקו מחדש את ההיסטוריה: ייתכן שהארנק שומר רשומה שגויה משרת קודם.';

  @override
  String get walletStallBirthdayInFuture =>
      'הארנק הזה מוגדר להתחיל מבלוק שהשרת הזה עדיין לא הגיע אליו. בדוק את בלוק ההתחלה שהארנק הזה מוגדר אליו, או נסה שרת אחר.';

  @override
  String get walletStallStorageUnavailable =>
      'הסנכרון הושהה במכשיר הזה. מנסה שוב.';

  @override
  String get walletStallUnknown => 'הסנכרון נעצר מסיבה לא ידועה.';

  @override
  String get walletSyncBadgeHint => 'הצג פרטי סנכרון';

  @override
  String get walletSyncSheetClose => 'סגור';

  @override
  String get walletSyncSheetProgress => 'התקדמות';

  @override
  String get walletSyncSheetBlocksLeft => 'בלוקים שנותרו';

  @override
  String get walletSyncSheetSyncedTo => 'מסונכרן עד בלוק';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'מאחר ב-$blocks בלוקים לפחות',
      many: 'מאחר ב-$blocks בלוקים לפחות',
      two: 'מאחר בשני בלוקים לפחות',
      one: 'מאחר בבלוק אחד לפחות',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'הסנכרון עדיין לא התחיל — הוא מתחיל באופן אוטומטי. אין צורך בפעולה.';

  @override
  String get walletSyncExplainStartFailed =>
      'הסנכרון לא הצליח להתחיל. הכספים שלך בטוחים — הארנק פשוט לא בודק כרגע פעילות חדשה. נסה שוב למטה, או פתח מחדש את האפליקציה.';

  @override
  String get walletSyncExplainStarting =>
      'הארנק פונה לרשת Zcash ומתכונן לסריקה. זה בדרך כלל לוקח כמה שניות.';

  @override
  String get walletSyncExplainConnecting => 'מתבצעת התחברות לרשת Zcash.';

  @override
  String get walletSyncExplainScanning =>
      'הארנק בודק בלוקים בבלוקצ\'יין בחיפוש אחר הכספים שלך. היתרה והפעילות שלך מתעדכנות עם איתור עסקאות חדשות — ניתן להמשיך להשתמש באפליקציה עד לסיום.';

  @override
  String get walletSyncExplainUpToDate =>
      'מסונכרן במלואו עם רשת Zcash. היתרה והפעילות שלך מעודכנות.';

  @override
  String get walletSyncExplainStalled =>
      'הסנכרון נתקל בבעיה והוא מושהה. הוא ינסה שוב באופן אוטומטי.';

  @override
  String get walletSyncExplainStalledOffline =>
      'לא ניתן להגיע לרשת Zcash — זה נורמלי אם אתה במצב לא מקוון, או שהשרת עשוי להיות לא זמין באופן זמני. הכספים שלך בטוחים: היתרה מוצגת לפי המצב הסינכרוני האחרון, ושליחות בתור נשארות שמורות תחת \"נשמר וממתין\". החיבור מנסה שוב באופן אוטומטי.';

  @override
  String get walletSyncExplainOffline =>
      'אין חיבור לרשת. הכספים שלך בטוחים — היתרה מוצגת לפי המצב הסינכרוני האחרון, ושליחות בתור נשארות שמורות תחת \"נשמר וממתין\".';

  @override
  String get walletSyncExplainUnknown =>
      'הארנק מסתנכרן. היתרה והפעילות שלך מתעדכנות עם ההתקדמות.';

  @override
  String get walletTorOff => 'Tor כבוי';

  @override
  String get walletTorBootstrapping => 'הנתיב הפרטי מופעל…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport מופעל…';
  }

  @override
  String get walletTorActive => 'Tor פעיל';

  @override
  String get walletTorActiveUnverified => 'Tor פעיל (סביבת הרצה לא מאומתת)';

  @override
  String get walletTorActiveUnattested => 'נתיב פרטי בשימוש (הפרטיות לא אומתה)';

  @override
  String get walletTorFellBack => 'Tor אינו זמין — משתמש בחיבור ישיר';

  @override
  String get walletTorUnavailable => 'הנתיב הפרטי אינו זמין — לא מחובר';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport אינו זמין — לא מחובר';
  }

  @override
  String get walletTorUnanswered => 'הנתיב הפרטי מחובר — שום דבר לא חוזר';

  @override
  String get walletTorUnansweredUnattested =>
      'הנתיב הפרטי מחובר — שום דבר לא חוזר (הפרטיות לא אומתה)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport מחובר — שום דבר לא חוזר';
  }

  @override
  String get walletTorUnansweredDirect =>
      'לא פרטי (החיבור הישיר של האפליקציה שלך) — שום דבר לא חוזר';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'מחובר דרך $transport — שום דבר לא חוזר; הפרוקסי יכול לקשר בין החיבורים';
  }

  @override
  String get walletTorUnknown =>
      'סטטוס Tor אינו ידוע — יש להתייחס אליו כלא מוגן';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'יתרה (נכון לבלוק $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'יתרה · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'יתרה (נכון לבלוק $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'חיבור';

  @override
  String get walletSyncSheetServer => 'שרת';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'שרת, $host, פותח את בחירת השרת';
  }

  @override
  String get walletSyncServerSheetTitle => 'שרת סנכרון';

  @override
  String get walletSyncServerInUse => 'בשימוש';

  @override
  String get walletSyncServerAppDefault => 'ברירת המחדל של האפליקציה';

  @override
  String get walletSyncServerCustom => 'שרת מותאם אישית…';

  @override
  String get walletSyncServerCustomHint => 'https://מארח:פורט';

  @override
  String get walletSyncServerCheck => 'בדיקת שרת';

  @override
  String get walletSyncServerChecking => 'בודק…';

  @override
  String get walletSyncServerUse => 'שימוש בשרת זה';

  @override
  String get walletSyncServerSwitching => 'מחליף…';

  @override
  String get walletSyncServerContinue => 'המשך';

  @override
  String get walletSyncServerCancel => 'ביטול';

  @override
  String get walletSyncServerTrustTitle => 'לסמוך על שרת זה?';

  @override
  String get walletSyncServerTrustNotice =>
      'אתם סומכים על שרת זה שידווח על היתרה וההיסטוריה שלכם ויעביר את התשלומים שלכם. הוא יראה את כתובת ה-IP שלכם אלא אם Tor פעיל, בערך מתי נוצר הארנק, את הכתובות הציבוריות שהארנק בודק, את העסקאות שהוא מחפש, ואת העסקאות שאתם שולחים.';

  @override
  String get walletSyncServerKeyLabel => 'מפתח גישה (רשות)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'כותרת המפתח';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'הזינו את הכותרת שהשרת שלכם מצפה לה';

  @override
  String get walletSyncServerKeyInvalid =>
      'לא ניתן להשתמש במפתח או בכותרת האלה';

  @override
  String get walletSyncServerKeySaved => 'המפתח נשמר';

  @override
  String get walletSyncServerKeyShow => 'הצגה';

  @override
  String get walletSyncServerKeyHide => 'הסתרה';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'המפתח שלכם מזהה אתכם מול השרת הזה. הוא יכול לקשר את התשלומים שלכם לארנק שלכם, גם דרך Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'ההחלפה תפעיל מחדש את הסנכרון שבעיצומו. היתרה וההיסטוריה נשמרות. הכספים עשויים להופיע כנכנסים עד שהסריקה של השרת החדש תתעדכן.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'ההחלפה תתחבר מחדש לשרת החדש. היתרה וההיסטוריה נשמרות.';

  @override
  String get walletSyncServerUnreachable =>
      'לא ניתן להגיע לשרת זה. בדקו את הכתובת — ואם היא נכונה, או שהשרת הזה לא עונה או שהאפליקציה שלכם לא מצליחה להגיע אליו כרגע. נסו שוב או בחרו שרת אחר.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'לא ניתן להגיע לשרת זה. הארנק לא יכול להבחין בין שרת שלא עונה לבין אפליקציה שלא מצליחה להגיע אליו כרגע. בחרו שרת אחר או נסו שוב מאוחר יותר.';

  @override
  String get walletSyncServerWrongNetwork => 'שרת זה נמצא ברשת Zcash אחרת.';

  @override
  String get walletSyncServerInvalidUrl =>
      'זה לא נראה כמו כתובת שרת. השתמשו בתבנית https://מארח:פורט.';

  @override
  String get walletSyncServerNotOffered => 'אפליקציה זו אינה מציעה שרת זה.';

  @override
  String get walletSyncServerBusy => 'הארנק עסוק כרגע. נסו שוב בעוד רגע.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'השרת שבחרתם אינו מוצע עוד באפליקציה זו. נעשה שימוש ב-$host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'לא ניתן היה לקרוא את בחירת השרת השמורה. נעשה שימוש ב-$host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'ההחלפה נכשלה — עדיין נעשה שימוש ב-$host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'תעבורת הארנק מתחברת ישירות לשרת. השרת יכול לראות את כתובת ה-IP שלך.';

  @override
  String get walletTransportExplainTor =>
      'תעבורת הארנק מנותבת דרך רשת Tor, המסתירה את כתובת ה-IP שלך מהשרת.';

  @override
  String get walletTransportExplainBootstrapping =>
      'הנתיב הפרטי של האפליקציה שלך מופעל כעת. תעבורת הארנק ממתינה לו לפני ההתחברות.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport מופעל כעת. תעבורת הארנק ממתינה לו לפני ההתחברות.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'לא ניתן היה להגיע ל-Tor, ולכן התעבורה חזרה לחיבור ישיר. השרת יכול לראות את כתובת ה-IP שלך.';

  @override
  String get walletTransportExplainUnavailable =>
      'הנתיב הפרטי של האפליקציה שלך אינו זמין, ולכן הארנק לא מתחבר. כבה את הנתיב הפרטי, או בדוק את הגדרות הרשת באפליקציה שלך.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport אינו זמין, ולכן הארנק לא מתחבר. כבה אותו, או בדוק את הגדרות הרשת באפליקציה שלך.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'הנתיב הפרטי קיבל את החיבור, אבל כבר דקה ששום דבר לא חוזר. זה יכול להיות הנתיב או שרת הארנק — הארנק לא יכול להבחין ביניהם. הוא ממשיך לנסות; אם זה לא נפתר, נסה שרת אחר או בדוק את הגדרות הרשת באפליקציה שלך.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport קיבל את החיבור, אבל כבר דקה ששום דבר לא חוזר. זה יכול להיות הנתיב או שרת הארנק — הארנק לא יכול להבחין ביניהם. הוא ממשיך לנסות; אם זה לא נפתר, נסה שרת אחר או בדוק את הגדרות הרשת באפליקציה שלך.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'תעבורת הארנק מתחברת ישירות לשרת. השרת יכול לראות את כתובת ה-IP שלך. החיבור התקבל, אבל כבר דקה ששום דבר לא חוזר. זה יכול להיות הנתיב או שרת הארנק — הארנק לא יכול להבחין ביניהם. הוא ממשיך לנסות; אם זה לא נפתר, נסה שרת אחר או בדוק את הגדרות הרשת באפליקציה שלך.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'לא ניתן לאמת את פרטיות החיבור הזה — יש להתייחס אליו כלא פרטי. החיבור התקבל, אבל כבר דקה ששום דבר לא חוזר. זה יכול להיות הנתיב או שרת הארנק — הארנק לא יכול להבחין ביניהם. הוא ממשיך לנסות; אם זה לא נפתר, נסה שרת אחר או בדוק את הגדרות הרשת באפליקציה שלך.';

  @override
  String get walletTransportExplainUnverified =>
      'לא ניתן לאמת את פרטיות החיבור הזה — יש להתייחס אליו כלא פרטי.';

  @override
  String get walletTransportExplainHostProxy =>
      'תעבורת הארנק מנותבת דרך תעבורת הפרטיות של אפליקציה זו, המסתירה את כתובת ה-IP שלך מהשרת.';

  @override
  String get walletOnboardingWelcomeTitle => 'הגדר את הארנק שלך';

  @override
  String get walletOnboardingWelcomeBody =>
      'צור ארנק חדש כדי לקבל ולהחזיק ZEC. ניצור עבורך ביטוי שחזור ונדריך אותך לגבות אותו לפני שיוכלו להגיע כספים כלשהם — כך שדבר אינו בסיכון ללא גיבוי.';

  @override
  String get walletCreateButton => 'צור ארנק חדש';

  @override
  String get walletRestoreButton => 'שחזר מביטוי שחזור';

  @override
  String get walletWatchOnlyButton => 'צפה בארנק (צפייה בלבד)';

  @override
  String get walletWatchOnlyTitle => 'צפייה בארנק';

  @override
  String get walletWatchOnlyBody =>
      'הדבק מפתח צפייה כדי לצפות בארנק ללא מפתחות ההוצאה שלו. תוכל לראות את היתרה וההיסטוריה שלו, אך לא תוכל לשלוח כספים. בחר את תאריך ההתחלה המשוער של הארנק כדי שנדע עד כמה רחוק לסרוק אחורה.';

  @override
  String get walletWatchOnlyKeyLabel => 'מפתח צפייה';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => 'סרוק קוד QR של מפתח הצפייה';

  @override
  String get walletWatchOnlyScanTitle => 'סריקת מפתח צפייה';

  @override
  String get walletWatchOnlyScanInstruction =>
      'כוון את המצלמה לעבר קוד ה-QR של מפתח הצפייה.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'המצלמה אינה זמינה. הדבק את המפתח ידנית במקום זאת.';

  @override
  String get walletWatchOnlyScanManualEntry => 'הדבק במקום זאת';

  @override
  String get walletWatchOnlyScanHint =>
      'או הקש על כפתור הסריקה כדי לקרוא קוד QR של מפתח צפייה.';

  @override
  String get walletWatchOnlyScanFilled => 'מפתח הצפייה נסרק.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'תאריך ההתחלה של הארנק';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'סורק החל מ-$date — כספים שהתקבלו לפני כן לא יופיעו. ארנק ישן יותר? בחר תאריך מוקדם יותר.';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'בחר את תאריך ההתחלה של הארנק';

  @override
  String get walletWatchOnlyBirthdayChange => 'שנה תאריך';

  @override
  String get walletWatchOnlySubmit => 'צפה בארנק זה';

  @override
  String get walletWatchOnlyBack => 'חזרה';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'זה לא נראה כמו מפתח צפייה תקין. בדוק אותו ונסה שוב.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'מפתח הצפייה הזה מיועד לרשת אחרת. אי אפשר להשתמש בו כאן.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'כבר קיים ארנק במכשיר הזה. חזור אחורה ופתח אותו במקום זאת.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'תאריך ההתחלה הזה מאוחר מדי. בחר תאריך מוקדם יותר.';

  @override
  String get walletRestoreTitle => 'שחזור הארנק שלך';

  @override
  String get walletRestoreBody =>
      'הזן את ביטוי השחזור שלך כדי לשחזר את הארנק — הקלד או הדבק את המילים לפי הסדר, מופרדות ברווחים. ביטויים סטנדרטיים בלבד: אם הארנק שלך השתמש בביטוי סיסמה נוסף (\"המילה ה-25\"), אפליקציה זו אינה יכולה לשחזר אותו עדיין — תראה ארנק ריק, לא שגיאה.';

  @override
  String get walletRestorePhraseHint =>
      'מילה ראשונה  מילה שנייה  מילה שלישית  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count מילים',
      one: 'מילה אחת',
      zero: 'עדיין אין מילים',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'לביטויי שחזור יש 12, 15, 18, 21 או 24 מילים';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count מילים אינן מילות שחזור — תקן את המילים המסומנות',
      one: 'מילה אחת אינה מילת שחזור — תקן את המילה המסומנת',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'מילה $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'מילה $index: אינה מילת שחזור';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'הסר מילה $index';
  }

  @override
  String get walletRestoreSubmit => 'שחזר ארנק';

  @override
  String get walletRestoreBack => 'חזרה';

  @override
  String get walletRestoreBirthdayTitle => 'עד כמה רחוק לסרוק אחורה';

  @override
  String get walletRestoreBirthdayNone =>
      'נסרוק את כל ההיסטוריה שלך — איטי יותר, אך שום דבר לא יוחמץ.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'סורק החל מ-$date — כספים שהתקבלו לפני כן לא יופיעו. ארנק ישן יותר? בחר תאריך מוקדם יותר, או סרוק את כל ההיסטוריה.';
  }

  @override
  String get walletRestoreBirthdayPick => 'בחר תאריך';

  @override
  String get walletRestoreBirthdayChange => 'שנה תאריך';

  @override
  String get walletRestoreBirthdayClear => 'סרוק את כל ההיסטוריה';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'מילה $index אינה מילת שחזור. בדוק את הביטוי שלך לשגיאות הקלדה, ונסה שוב.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'ביטוי השחזור אינו תקין. בדוק את המילים ואת סדרן, ונסה שוב.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'הביטוי אינו תואם לארנק שבמכשיר זה. בדוק אותו שוב ונסה מחדש.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'כבר קיים ארנק במכשיר זה. חזור אחורה כדי לפתוח אותו.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'התאריך הזה קרוב מדי. בחר תאריך מוקדם יותר, או סרוק הכול.';

  @override
  String get walletGeneratingLabel => 'יוצר את הארנק שלך…';

  @override
  String get walletOpeningLabel => 'פותח את הארנק שלך…';

  @override
  String get walletBackupTitle => 'גבה את ביטוי השחזור שלך';

  @override
  String get walletBackupBody =>
      'מילים אלה הן הדרך היחידה לשחזר את הארנק והכספים שלך. רשום אותן לפי הסדר ושמור אותן במקום בטוח ופרטי. לעולם אל תשתף אותן ואל תאחסן אותן באינטרנט — כל מי שמחזיק במילים אלה יכול להשתלט על הכספים שלך.';

  @override
  String get walletBackupSecureNoteAndroid => 'צילומי מסך מושבתים במסך זה.';

  @override
  String get walletBackupSecureNoteOther =>
      'ודא שאף אחד אינו יכול לראות את המסך שלך.';

  @override
  String get walletBackupReveal => 'הצג את ביטוי השחזור';

  @override
  String get walletBackupRevealing => 'מכין את ביטוי השחזור שלך…';

  @override
  String get walletBackupRevealFailed =>
      'לא ניתן היה להציג את ביטוי השחזור כרגע. ודא שהמכשיר שלך אינו נעול, ונסה שוב.';

  @override
  String get walletBackupRetryReveal => 'נסה שוב';

  @override
  String get walletBackupReauthFailed => 'לא ניתן היה לאמת את זהותך. נסה שוב.';

  @override
  String get walletBackupConfirmCheckbox =>
      'רשמתי את ביטוי השחזור שלי ושמרתי אותו במקום בטוח.';

  @override
  String get walletBackupContinue => 'המשך';

  @override
  String get walletBackupSaveFailed =>
      'לא ניתן היה לשמור את האישור שלך. נסה שוב.';

  @override
  String get walletBackupStartOver => 'התחל מחדש';

  @override
  String get walletBackupStartOverConfirmTitle => 'להתחיל מחדש בלי הארנק הזה?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'פעולה זו מוחקת ארנק זה מהמכשיר ומחזירה אותך להתחלה. לא ניתן להפקיד דבר דרך אפליקציה זו לפני שההגדרה הושלמה.\n\nאם אי פעם היו בארנק זה כספים — או שהוא שוחזר מביטוי שחזור — רק אותו ביטוי יכול לשחזר אותו.';

  @override
  String get walletBackupStartOverConfirm => 'מחק והתחל מחדש';

  @override
  String get walletBackupStartOverKeep => 'השאר את הארנק הזה';

  @override
  String get walletBackupSectionTitle => 'ביטוי השחזור';

  @override
  String get walletBackupTileTitle => 'גבה את ביטוי השחזור שלך';

  @override
  String get walletBackupTileSubtitle =>
      'הצג את המילים שיכולות לשחזר את הארנק והכספים שלך.';

  @override
  String get walletBackupScreenTitle => 'ביטוי השחזור';

  @override
  String get walletBackupDone => 'סיום';

  @override
  String get walletBackupManagedTitle => 'אין ביטוי שחזור נפרד';

  @override
  String get walletBackupManagedBody =>
      'הארנק הזה הוגדר באמצעות החשבון שלך מהאפליקציה שהתקינה אותו, ולכן אין לו ביטוי שחזור משלו. הכספים שלך משוחזרים יחד עם החשבון הזה — השתמש בגיבוי שלו כדי לשמור עליהם.';

  @override
  String get walletExportViewingKeyTitle => 'ייצוא מפתח צפייה';

  @override
  String get walletExportViewingKeyTileTitle => 'ייצוא מפתח צפייה';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'שתף עותק לצפייה בלבד של הארנק שלך — הוא יכול לראות את ההיסטוריה שלך, אך אינו יכול להוציא כספים.';

  @override
  String get walletExportViewingKeyWarning =>
      'מפתח זה מאפשר לכל מי שמחזיק בו לראות את כל מה שהארנק הזה קיבל ושלח אי פעם — וכל מה שיקבל וישלח בעתיד. הוא אינו יכול להוציא את הכספים שלך או לשחזר את הארנק שלך. שתף אותו רק עם מי שאתה סומך עליו לראות את ההיסטוריה המלאה שלך, כמו רואה החשבון שלך או המכשיר השני שלך. הדרך היחידה לבטל את השיתוף בהמשך היא להעביר את הכספים שלך לארנק חדש.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'מפתח זה מאפשר לכל מי שמחזיק בו לראות את כל מה שהארנק הזה קיבל ושלח אי פעם — וכל מה שיקבל וישלח בעתיד. הוא אינו יכול להוציא את הכספים שלך או לשחזר את הארנק שלך. שתף אותו רק עם מי שאתה סומך עליו לראות את ההיסטוריה המלאה שלך, כמו רואה החשבון שלך או המכשיר השני שלך. ברגע ששיתפת אותו, לא ניתן לבטל את השיתוף.';

  @override
  String get walletExportViewingKeyReveal => 'הצג את מפתח הצפייה';

  @override
  String get walletExportViewingKeyRetry => 'נסה שוב';

  @override
  String get walletExportViewingKeyRevealing => 'מכין את מפתח הצפייה שלך…';

  @override
  String get walletExportViewingKeyFailed =>
      'לא ניתן היה להציג את מפתח הצפייה כרגע. נסה שוב בעוד רגע.';

  @override
  String get walletExportViewingKeyQrLabel => 'קוד QR של מפתח הצפייה';

  @override
  String get walletExportViewingKeyCopy => 'העתק מפתח צפייה';

  @override
  String get walletExportViewingKeyCopied => 'מפתח הצפייה הועתק';

  @override
  String get walletExportViewingKeyDone => 'סיום';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'צילומי מסך מושבתים במסך זה.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'ודא שאף אחד אינו יכול לראות את המסך שלך.';

  @override
  String get walletWatchOnlySectionTitle => 'על אודות הארנק לצפייה בלבד הזה';

  @override
  String get walletWatchOnlyAboutBody =>
      'זהו ארנק לצפייה בלבד. הוא הוגדר מתוך מפתח צפייה, ולכן הוא יכול לראות את היתרה וההיסטוריה שלך, אך אינו מחזיק במפתחות הוצאה — אין כאן דבר לגבות, והוא אינו יכול לשלוח כספים.';

  @override
  String get walletWatchOnlyBadge => 'צפייה בלבד';

  @override
  String get walletOnboardingFailedTitle => 'הגדרת הארנק לא הושלמה';

  @override
  String get walletOnboardingRetry => 'נסה שוב';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'האחסון המאובטח בטלפון שלך אינו מגיב. שחרר את נעילת המכשיר ונסה שוב. אם זה ממשיך לקרות, הפעל מחדש את הטלפון.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'הארנק הזה פתוח בחלון או באפליקציה אחרת, או שהוא עדיין מסיים פעולה קודמת. סגור כל חלון אחר שמשתמש בו — או המתן רגע — ואז נסה שוב.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'המפתח המאובטח של הארנק הזה כבר אינו זמין, ולכן לא ניתן לפתוח אותו במכשיר זה. הכספים שלך בטוחים — שחזר מביטוי השחזור שלך כדי לשחזר אותם.';

  @override
  String get walletOnboardingFailedRestoreAction => 'שחזר מביטוי שחזור';

  @override
  String get walletOnboardingRecoverConfirmTitle => 'לשחזר ארנק זה?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'ודא שביטוי השחזור שלך ברשותך לפני שתמשיך — תזדקק לו במסך הבא כדי לשחזר את הכספים שלך. הכספים שלך בטוחים בבלוקצ\'יין ונשלטים על ידי הביטוי הזה. פעולה זו תסיר את נתוני הארנק שאינם ניתנים לקריאה מהמכשיר הזה כדי שניתן יהיה לבנות אותו מחדש.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'ביטול';

  @override
  String get walletOnboardingFailedStorageFull =>
      'אין מספיק מקום פנוי כדי להגדיר את הארנק שלך. פנה מקום ונסה שוב.';

  @override
  String get walletOnboardingFailedNoVault =>
      'למכשיר זה אין מחסן מפתחות מאובטח, ולכן הארנק אינו יכול להגן על ביטוי השחזור שלך כאן.';

  @override
  String get walletOnboardingFailedNetwork =>
      'לא ניתן היה להגיע לרשת במהלך ההגדרה. בדוק את החיבור שלך ונסה שוב.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'הגדרת הארנק לא הושלמה. נסה שוב כדי להשלים אותה — שום דבר לא אבד.';

  @override
  String get walletOnboardingFailedUnknown =>
      'משהו השתבש בהגדרת הארנק שלך. נסה שוב.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'הגדרת הארנק באפליקציה זו שגויה, ולכן הארנק אינו יכול להתחיל לפעול. ניסיון חוזר לא יעזור — אנא דווח על כך למפתח האפליקציה. הכספים שלך אינם נפגעים.';

  @override
  String get walletSendButton => 'שלח';

  @override
  String get walletSendSyncNotRunning =>
      'הסנכרון לא פועל — היתרה הזמינה לשימוש שלך לא תוכל להתעדכן';

  @override
  String get walletSendWaitingForFunds =>
      'הסנכרון עדיין מתבצע — תוכל לשלוח ברגע שתהיה לך יתרה זמינה לשימוש';

  @override
  String get walletSendNoSpendableYet => 'אין עדיין יתרה זמינה לשימוש';

  @override
  String get walletSendSyncUnavailable => 'תוכל לשלוח ברגע שהסנכרון יתחדש';

  @override
  String get walletSendTitle => 'שליחה';

  @override
  String get walletSendUnavailable =>
      'הארנק שלך אינו מוכן כרגע. חזור אחורה ונסה שוב.';

  @override
  String get walletSendWatchOnly =>
      'זהו ארנק לצפייה בלבד. הוא יכול להציג יתרות ולקבל תשלומים, אך אין בו מפתחות הוצאה — ולכן הוא לא יכול לשלוח.';

  @override
  String get walletSendExpiredTitle => 'תוקף בקשת התשלום הזו פג';

  @override
  String get walletSendExpiredBody =>
      'מסך השליחה נפתח אחרי יותר מחמש שניות, ולכן האפליקציה קיבלה הודעה ששום דבר לא נשלח. התשובה הזו סופית: לא ניתן לשלם את הבקשה הזו מכאן. כדי לשלם, התחל מחדש מהאפליקציה.';

  @override
  String get walletSendFaultWatchOnly =>
      'זהו ארנק לצפייה בלבד — אין בו מפתחות הוצאה, ולכן הוא לא יכול לשלוח.';

  @override
  String walletSendAvailable(String amount) {
    return 'זמין לשליחה: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'זמין לשליחה: $amount ZEC — היתרה שלך עדיין משלימה פערים';
  }

  @override
  String get walletSendRecipientLabel => 'כתובת הנמען';

  @override
  String get walletSendRecipientHint => 'כתובת Zcash (מתחילה ב-u, z או t)';

  @override
  String get walletSendRecipientLocked => 'לא ניתן לשנות את הנמען כאן';

  @override
  String get walletSendAmountLabel => 'סכום (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'הערה (אופציונלי)';

  @override
  String get walletSendMemoHint => 'נמסרת רק לנמענים מוגנים (פרטיים)';

  @override
  String get walletSendMemoTransparentDisabled =>
      'הערות דורשות נמען מוגן. כתובת ציבורית זו אינה יכולה לקבל הערה.';

  @override
  String get walletSendMemoMachineDisabled =>
      'התשלום הזה כבר נושא מזהה מהאפליקציה, ולכן אינו יכול לשאת גם הערה כתובה.';

  @override
  String get walletSendMachineMemoTitle => 'האפליקציה מצרפת מזהה';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'לפי דבריה זה נועד ל: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'הוא נשאר עם העסקה ולא ניתן להסירו מאוחר יותר. הארנק אינו יכול לבדוק מה הוא מכיל.';

  @override
  String get walletSendRecipientShielded => 'מוגן · פרטי';

  @override
  String get walletSendRecipientTransparent => 'ציבורי';

  @override
  String get walletSendRecipientInvalid => 'זו אינה נראית ככתובת Zcash תקינה.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'כתובת זו מיועדת לרשת Zcash אחרת.';

  @override
  String get walletSendReviewButton => 'סקירת התשלום';

  @override
  String get walletSendQueueButton => 'העבר לתור לשליחה מאוחר יותר';

  @override
  String get walletSendQueueHint =>
      'תשלום שממתין בתור נשאר תחת \"נשמר וממתין\", שם תוכל לשלוח אותו או לבטל אותו. עמלת הרשת מחושבת בזמן השליחה.';

  @override
  String get walletSendPreparing => 'מכין את התשלום שלך…';

  @override
  String get walletSendSubmitting => 'שולח…';

  @override
  String get walletSendQueuing => 'מעביר לתור…';

  @override
  String get walletSendReviewTitle => 'אישור התשלום';

  @override
  String get walletSendTotalLabel => 'סך הכול';

  @override
  String get walletSendFeeLabel => 'עמלת רשת';

  @override
  String get walletSendChangeLabel => 'עודף שהוחזר';

  @override
  String get walletSendDeshieldTitle => 'תשלום זה אינו פרטי';

  @override
  String get walletSendDeshieldBody =>
      'הוא נשלח לכתובת ציבורית, כך שהסכום והנמען יהיו גלויים לציבור בבלוקצ\'יין של Zcash.';

  @override
  String get walletSendPublicAckLabel => 'ידוע לי שהתשלום הזה יהיה ציבורי.';

  @override
  String get walletSendConfirmButton => 'שלח עכשיו';

  @override
  String get walletSendBackButton => 'חזרה';

  @override
  String get walletSendSelfSendNote =>
      'אתה שולח לארנק שלך עצמך. עמלת הרשת עדיין חלה.';

  @override
  String get walletSendLargeConfirmTitle => 'לשלוח סכום גדול?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'זהו כמעט כל היתרה שלך. לא ניתן לבטל תשלום שנשלח.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'זהו תשלום גדול. לא ניתן לבטל תשלום שנשלח.';

  @override
  String get walletSendLargeConfirmBoth =>
      'זהו תשלום גדול — כמעט כל היתרה שלך. לא ניתן לבטל תשלום שנשלח.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'שלח $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'חזור';

  @override
  String get walletSendSentTitle => 'התשלום נשלח';

  @override
  String get walletSendSentBody => 'התשלום שלך שודר לרשת.';

  @override
  String get walletSendSavedTitle => 'נשמר — נשלים את השליחה';

  @override
  String get walletSendSavedBody =>
      'לא ניתן היה לשלוח את התשלום שלך כרגע, ולכן הוא נשמר והארנק שלך ישלח אותו בסנכרון מאוחר יותר. שום דבר לא אבד.';

  @override
  String get walletSendKeptTitle => 'נשמר';

  @override
  String get walletSendKeptBody =>
      'הארנק שלך שמר את העסקה הזו אך לא התחייב לשלוח אותה בעצמו. בדוק בפעילות כדי לראות את מצבה.';

  @override
  String get walletSendPartialBody =>
      'חלק מהתשלום שלך יצא לדרך; הארנק שלך ישלים את השאר בסנכרון מאוחר יותר. שום דבר לא אבד.';

  @override
  String get walletSendInMotionTitle => 'התשלום בתהליך';

  @override
  String get walletSendInMotionBody =>
      'התשלום שלך התחיל והוא עובר דרך כתובת חד-פעמית שבשליטת הארנק שלך. אל תשלח אותו שוב. אם הוא לא יושלם, תוכל לשחזר את הכספים ממסך הארנק שלך.';

  @override
  String get walletSendAlreadyTitle => 'כבר נשלח';

  @override
  String get walletSendAlreadyBody =>
      'תשלום זה כבר נשלח — הוא לא יישלח פעמיים.';

  @override
  String get walletSendFailedTitle => 'לא ניתן להשלים את התשלום';

  @override
  String get walletSendFailedBody =>
      'משהו השתבש בהשלמת התשלום ושום דבר לא נשלח. תוכל לנסות שוב.';

  @override
  String get walletSendTryAgain => 'נסה שוב';

  @override
  String get walletSendDone => 'סיום';

  @override
  String get walletSendAnother => 'שלח תשלום נוסף';

  @override
  String get walletSendQueuedTitle => 'הועבר לתור לשליחה';

  @override
  String get walletSendQueuedBody =>
      'התשלום הזה שמור. תמצא אותו תחת \"נשמר וממתין\", שם תוכל לשלוח אותו עכשיו או לבטל אותו.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'אין מספיק יתרה זמינה לשימוש — יש לך $available ZEC וזה דורש $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'רשת Zcash שודרגה והאפליקציה הזו זקוקה לעדכון לפני שתוכל לשלוח. הכספים שלך בטוחים.';

  @override
  String get walletSyncUpToDateLimited =>
      'מסונכרן עד כמה שהגרסה הזו יכולה לקרוא';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'רשת Zcash שודרגה. הגרסה הזו סרקה את כל מה שהיא יכולה לקרוא, אבל בלוקים חדשים יותר עשויים להכיל כספים שהיא עדיין לא יכולה להציג, והערות על תשלומים אחרונים אינן זמינות. עדכן את האפליקציה כדי לראות הכול.';

  @override
  String get walletSyncUpToDateDegraded =>
      'מעודכן, אבל השרת הזה לא משרת את כל המאגרים';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'השרת הזה מסרב, מסתיר או מדווח באופן שגוי על אחד ממאגרי ה-Zcash המוגנים. לא ניתן להוציא כספים שהתקבלו במאגר הזה דרך השרת, והיתרה המוצגת היא ערך מינימלי. עברו לשרת אחר כדי להשתמש בהם — זו לא בעיית חיבור.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: השרת הזה מסרב לשרת אותו';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: השרת הזה מעכב חלק ממנו';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: השרת הזה מדווח עליו באופן שגוי';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: לא ידוע אם השרת הזה משרת אותו';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'מעודכן מול השרת הזה, אבל השרת מפגר אחרי הרשת';

  @override
  String get walletSyncExplainEndpointBehind =>
      'השרשרת של השרת הזה נעצרת בבלוק שהרשת כבר עברה לפני שגרסה זו של האפליקציה נבנתה, ולכן היתרה שלכם מעודכנת רק עד אותו בלוק. תשלומים חדשים אליכם אולי עדיין לא מוצגים, ותשלום שנשלח מכאן עלול לא להגיע. עברו לשרת אחר כדי להתעדכן — זו לא בעיית חיבור.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'ממתין לעדכון האפליקציה — הכספים שלך בטוחים ולא נשלח דבר.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'ממתין לשרת שמדווח על גרסת הרשת — החליפו שרת. הכספים שלכם בטוחים ולא נשלח דבר.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'ממתין לשרת שמדווח על גרסת הרשת. אם התאריך והשעה במכשיר זה שגויים, תקנו אותם קודם — ואז החליפו שרת. הכספים שלכם בטוחים ולא נשלח דבר.';

  @override
  String get walletSyncUnverified =>
      'מעודכן, אבל השרת הזה לא מדווח על גרסת הרשת';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'השליחה עדיין פועלת למשך כ-$hours שעות נוספות — לאחר מכן החליפו שרת.',
      two: 'השליחה עדיין פועלת למשך כשעתיים נוספות — לאחר מכן החליפו שרת.',
      one: 'השליחה עדיין פועלת למשך כשעה נוספת — לאחר מכן החליפו שרת.',
      zero: 'השליחה עדיין פועלת למשך פחות משעה — לאחר מכן החליפו שרת.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'השליחה עדיין פועלת למשך כ-$blocks בלוקים נוספים — לאחר מכן החליפו שרת.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'השרת הזה לא דיווח על גרסת הרשת במשך $blocks בלוקים, ולכן האפליקציה לא יכולה לאשר שבטוח לשלוח. עברו לשרת אחר.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'השרת הזה לא דיווח על גרסת הרשת במשך יום, ולכן האפליקציה לא יכולה לאשר שבטוח לשלוח. אם התאריך והשעה במכשיר זה שגויים, תקנו אותם קודם — ואז עברו לשרת שמדווח על גרסת הרשת.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'השרת הזה מעולם לא דיווח על גרסת הרשת, ולכן האפליקציה לא יכולה לאשר שבטוח לשלוח. עברו לשרת אחר.';

  @override
  String get walletSyncExplainUnverified =>
      'השרת הזה לא אומר באיזו גרסה של רשת Zcash הוא נמצא, ולכן האפליקציה לא יכולה לאשר שתשלום שהיא חותמת עליו יתקבל. היתרה שלכם מעודכנת. עברו לשרת אחר — זו לא בעיית חיבור.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'השרת הזה לא אומר באיזו גרסה של רשת Zcash הוא נמצא, ולכן האפליקציה לא יכולה לאשר שתשלום שהיא חותמת עליו יתקבל. הוא גם המשיך לספק בלוקים שהארנק נאלץ אחר כך לבטל, ולכן היתרה שלכם אולי אינה מעודכנת. עברו לשרת אחר — זו לא בעיית חיבור.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'השרת הזה גם ממשיך לספק בלוקים שהארנק נאלץ אחר כך לבטל — החליפו שרת.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'היתרה שלך עדיין משלימה פערים — ייתכן שיהיה זמין יותר תוך כדי סנכרון הארנק.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC עדיין בדרך ויהיו זמינים לשימוש לאחר שהארנק יתעדכן.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'הזן סכום לשליחה.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'הזן את הסכום כמספר, לדוגמה 0.25.';

  @override
  String get walletSendFaultAmountDecimals => 'ל-ZEC יש עד 8 ספרות עשרוניות.';

  @override
  String get walletSendFaultAmountNotPositive => 'הזן סכום גדול מאפס.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'סכום זה גדול מסך היצע ה-ZEC הכולל.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'אפליקציה זו מגבילה כרגע שליחות ל-$limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'זו אינה נראית ככתובת Zcash תקינה עבור רשת זו. בדוק אותה ונסה שוב.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'נמען זה אינו יכול לקבל הערה. הסר את ההערה, או שלח לכתובת מוגנת (פרטית).';

  @override
  String get walletSendFaultMemoTooLong =>
      'ההערה שלך ארוכה מדי. קצר אותה ונסה שוב.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'לא ניתן לשלוח הערה זו. הסר אותה ונסה שוב.';

  @override
  String get walletSendFaultMemoConflict =>
      'לא ניתן היה לשלוח את התשלום הזה — האפליקציה צירפה אליו שתי הערות. שום דבר לא נשלח.';

  @override
  String get walletSendFaultNetworkMismatch => 'כתובת זו מיועדת לרשת אחרת.';

  @override
  String get walletSendFaultUriInvalid =>
      'לא ניתן היה ליצור תשלום זה. בדוק את הכתובת ואת הסכום.';

  @override
  String get walletSendFaultNotSynced =>
      'הארנק שלך עדיין לא מסונכרן מספיק. המתן שהסנכרון ישלים את הפער, או העבר את זה לתור לשליחה מאוחר יותר.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'הארנק שלך עדיין לא מסונכרן מספיק. המתן שהסנכרון ישלים את הפער.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'הארנק שלך עדיין לא מסונכרן מספיק, והסנכרון לא פועל כרגע. בדוק את מצב הסנכרון במסך הארנק.';

  @override
  String get walletSendFaultAmountsExpired =>
      'תוקף הסכומים פג בזמן שסקרת אותם. סקור את התשלום שוב.';

  @override
  String get walletSendFaultQueueFull =>
      'יותר מדי שליחות ממתינות לצאת. תן להן להישלח קודם, ואז נסה שוב.';

  @override
  String get walletSendFaultWalletBusy => 'הארנק עסוק כרגע. נסה שוב בעוד רגע.';

  @override
  String get walletSendFaultStorageFull =>
      'אין מספיק מקום פנוי כדי להשלים שליחה זו. פנה מקום ונסה שוב.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'יותר מדי כתובות חד-פעמיות בשימוש כרגע. חלקן עשויות להתפנות ככל שההעברות יאושרו, אך ייתכן שהמצב לא ייפתר מעצמו. הכספים שלך בטוחים.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'לא ניתן היה להכין תשלום זה. בדוק את הפרטים ונסה שוב.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'לא ניתן היה להכין את התשלום הזה כרגע. נסו שוב בעוד רגע.';

  @override
  String get walletSwapButton => 'החלף';

  @override
  String get walletSwapTitle => 'החלפת ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'הארנק שלך אינו מוכן כרגע. חזור אחורה ונסה שוב.';

  @override
  String get walletSwapUnavailableOff => 'ההחלפה אינה זמינה כרגע.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'זהו ארנק לצפייה בלבד — הוא לא יכול להחליף.';

  @override
  String get walletSwapDone => 'סיום';

  @override
  String get walletSwapBackToWallet => 'חזרה לארנק';

  @override
  String walletSwapAvailable(String amount) {
    return 'זמין להחלפה: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'זמין להחלפה: $amount ZEC — היתרה שלך עדיין משלימה פערים';
  }

  @override
  String get walletSwapAssetLabel => 'נכס לקבלה';

  @override
  String get walletSwapAmountLabel => 'סכום להחלפה (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'כתובת יעד';

  @override
  String get walletSwapDestinationHint => 'כתובת הקבלה שלך בשרשרת היעד';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'כתובת הקבלה שלך ב-$chain';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'כתובת $chain — לשם נשלח הנכס שקיבלת מההחלפה. ודא שהשרשרת נכונה.';
  }

  @override
  String get walletSwapDestinationScanTooltip => 'סרוק קוד QR של כתובת יעד';

  @override
  String get walletSwapTargetAssetHint => 'בחר נכס לקבלה';

  @override
  String get walletSwapQuoteButton => 'קבל הצעת מחיר';

  @override
  String get walletSwapQuoting => 'מקבל הצעת מחיר…';

  @override
  String get walletSwapExecuting => 'מתחיל את ההחלפה שלך…';

  @override
  String get walletSwapExecuteStillWorking =>
      'עדיין בעבודה — ההחלפה מתחילה. פעולה זו עשויה לקחת עד דקה.';

  @override
  String get walletSwapReviewTitle => 'אישור ההחלפה';

  @override
  String get walletSwapYouSendLabel => 'אתה שולח';

  @override
  String get walletSwapYouReceiveLabel => 'תקבל לפחות';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'עמלת רשת';

  @override
  String get walletSwapNetworkFeeValue => 'מתווספת בעת שליחת ההפקדה';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'הצעת המחיר בתוקף לעוד כ-$time — אשר לפני שתפוג.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'הצעת המחיר בתוקף לעוד פחות מדקה — אשר לפני שתפוג.';

  @override
  String get walletSwapQuoteExpired =>
      'תוקף הצעת המחיר הזו פג. חזור וקבל הצעה חדשה — השער כבר אינו מובטח, ושליחה כעת עלולה להוביל להחזר כספי.';

  @override
  String get walletCountdownUnderMinute => 'פחות מדקה';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes דק\'';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds שנ\'';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '$hours שע\' $minutes דק\'';
  }

  @override
  String get walletSwapDeshieldTitle => 'החלפה זו אינה פרטית';

  @override
  String get walletSwapDeshieldBody =>
      'החלפה החוצה מסירה את ההגנה מה-ZEC שלך — ההפקדה היא עסקה ציבורית, וצד הספק פומבי ברשת שלו.';

  @override
  String get walletSwapDiscloseTitle => 'מה ספק ההחלפה יראה';

  @override
  String get walletSwapDiscloseAmounts => 'הסכומים בשני הצדדים';

  @override
  String get walletSwapDiscloseCrossLink =>
      'שה-ZEC הזה והנכס שתקבל הם חלק מהחלפה אחת';

  @override
  String get walletSwapDiscloseDestination => 'כתובת היעד שלך';

  @override
  String get walletSwapDiscloseSource => 'כתובת המקור שלך';

  @override
  String get walletSwapDiscloseIp => 'כתובת ה-IP שלך (אלא אם כן תנתב דרך Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'פרטים נוספים של החלפה זו';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'העסקאות של הספק עצמו פומביות ברשת שלו';

  @override
  String get walletSwapAckLabel => 'אני מבין שהספק יראה את המידע שלמעלה.';

  @override
  String get walletSwapConfirmButton => 'התחל החלפה';

  @override
  String get walletSwapBackButton => 'חזרה';

  @override
  String get walletSwapStatusPendingTitle => 'ההחלפה החלה';

  @override
  String get walletSwapStatusCheckingTitle => 'בודק את מצב ההחלפה…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'הארנק שלך שולח את הפקדת ה-ZEC אל הספק. אם אתה במצב לא מקוון לזמן קצר, ההפקדה תישלח אוטומטית ברגע שתתחבר מחדש — אך חלון השליחה קצר, ואם הוא נסגר לפני כן, ההחלפה פשוט מסתיימת ולא מתבצעת שום החלפה. ה-ZEC שלך נשאר שלך, וייתכן שייקח עד שעה עד שהוא יוצג שוב כזמין לשימוש.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'ממתינים להגעת ההפקדה שלך. אם עדיין לא שלחת את הכספים מהארנק האחר שלך, שלח אותם לפני שתוקף הצעת המחיר יפוג.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'החלפה זו עדיין ממתינה להפקדה שלה. הוראות ההפקדה אינן זמינות עוד במכשיר זה — אם כבר שלחת את הכספים, הם יזוהו; אם לא, תן להחלפה הזו לפוג והתחל אחת חדשה.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'חלון ההפקדה מסתיים ב-$time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'חלון ההפקדה חלף. אם ההפקדה לא נשלחה בזמן, ההחלפה מסתיימת וה-ZEC שלך נשאר בארנק שלך.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'חלון ההפקדה חלף. אם עדיין לא שלחת את ההפקדה שלך, ההחלפה הזו פשוט מסתיימת — קבל הצעת מחיר חדשה כשתהיה מוכן.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'החלפות בתהליך',
      one: 'החלפה בתהליך',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec => 'ה-ZEC שלך בדרך לספק ההחלפה.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'ממתינים להגעת ההפקדה שלך לספק ההחלפה.';

  @override
  String get walletSwapInFlightRowGeneric => 'יש החלפה בתהליך.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'חלון ההפקדה חלף — בדוק את סטטוס ההחלפה הזו.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'ההחלפה הזו עדיין לא הגיעה לתוצאה מאושרת כאן — פתח אותה כדי לבדוק. כל ZEC שחוזר לארנק זה יופיע ביתרה שלך לאחר סנכרון.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'ההחלפה הזו עדיין לא הגיעה לתוצאה מאושרת כאן — פתח אותה כדי לבדוק. כל ZEC שההחלפה מעבירה לארנק זה יופיע ביתרה שלך לאחר סנכרון.';

  @override
  String get walletSwapRowOutcomeSuccess => 'ההחלפה הושלמה.';

  @override
  String get walletSwapRowOutcomeRefunded => 'ההחלפה הוחזרה.';

  @override
  String get walletSwapRowOutcomeFailed => 'ההחלפה לא הושלמה.';

  @override
  String get walletSwapRemove => 'הסר';

  @override
  String get walletSwapRemoveTitle => 'להסיר את ההחלפה הזו מהרשימה?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'פעולה זו רק מסירה את ההחלפה מהרשימה הזו — היא אינה מבטלת את ההחלפה, וארנק זה יפסיק לעקוב אחר ההחזר שלה. ZEC שיוחזר מאוחר יותר עדיין שייך לארנק זה; סריקה מחדש מלאה יכולה למצוא אותו.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'פעולה זו רק מסירה את ההחלפה מהרשימה הזו — היא אינה מבטלת את ההחלפה, וארנק זה יפסיק לעקוב אחר ה-ZEC הנכנס. ZEC שיגיע מאוחר יותר עדיין שייך לארנק זה; סריקה מחדש מלאה יכולה למצוא אותו. אם ההחלפה תוחזר במקום זאת, ההחזר יחזור בנכס ששלחת, מחוץ לארנק זה.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'פעולה זו רק מסירה את ההחלפה מהרשימה הזו — היא אינה מבטלת את ההחלפה, וארנק זה יפסיק לעקוב אחר ZEC שעדיין מגיע ממנה. ZEC שיגיע מאוחר יותר עדיין שייך לארנק זה; סריקה מחדש מלאה יכולה למצוא אותו.';

  @override
  String get walletSwapRemoveBodyDone =>
      'פעולה זו מסירה את ההחלפה שהסתיימה מהרשימה.';

  @override
  String get walletSwapRemoveCancel => 'ביטול';

  @override
  String get walletSwapRemoveConfirm => 'הסר';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'החלה ב-$time';
  }

  @override
  String get walletSwapViewSwap => 'הצג את ההחלפה';

  @override
  String get walletSwapsInFlightError =>
      'לא ניתן היה לטעון כרגע את ההחלפות שלך שבתהליך.';

  @override
  String get walletSwapsInFlightRetry => 'נסה שוב';

  @override
  String get walletSwapsInFlightRetryInProgress => 'מנסה…';

  @override
  String get walletSwapStartAnother => 'התחל החלפה נוספת';

  @override
  String get walletSwapStatusUnderTitle => 'ממתין להפקדה המלאה';

  @override
  String get walletSwapStatusUnderBody =>
      'חלק מההפקדה הגיע. השאר בתהליך השלמה, או שהספק יחזיר את הכסף.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'חלק מההפקדה שלך הגיע. שלח את הסכום החסר לפני המועד האחרון, או שהספק יחזיר את מה שהגיע.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'התקבל $received; עדיין חסר $missing. חלון ההפקדה מסתיים: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'ההפקדה התקבלה';

  @override
  String get walletSwapStatusDetectedBody =>
      'הספק קיבל את ההפקדה שלך ויעבד את ההחלפה.';

  @override
  String get walletSwapStatusProcessingTitle => 'מעבד את ההחלפה שלך';

  @override
  String get walletSwapStatusProcessingBody => 'הספק משלים את ההחלפה שלך.';

  @override
  String get walletSwapStatusSuccessTitle => 'ההחלפה הושלמה';

  @override
  String get walletSwapStatusSuccessBody => 'ההחלפה שלך הסתיימה בהצלחה.';

  @override
  String get walletSwapStatusRefundedTitle => 'ההחלפה הוחזרה';

  @override
  String get walletSwapStatusRefundedBody =>
      'ההחלפה לא הושלמה, ולכן הספק שלח את הכספים בחזרה לכתובת ההחזר שלך.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'ההחלפה לא הושלמה, ולכן הספק שלח את ה-ZEC שלך בחזרה לארנק זה. הוא מגיע ככספים לא מוגנים ומופיע ביתרה שלך לאחר הסנכרון הבא של הארנק — זה עשוי לקחת קצת זמן.';

  @override
  String get walletSwapStatusFailedTitle => 'ההחלפה נכשלה';

  @override
  String get walletSwapStatusFailedBody =>
      'לא ניתן היה להשלים את ההחלפה. כספים שהופקדו יסולקו או יוחזרו בצד הספק.';

  @override
  String get walletSwapStatusNotFoundTitle => 'ההחלפה לא נמצאה';

  @override
  String get walletSwapStatusNotFoundBody =>
      'לספק אין עוד תיעוד של החלפה זו — סביר להניח שפג תוקפה. אם בוצעה הפקדה, הספק אמור להחזיר אותה לכתובת ההחזר. ההחלפה נשארת ברשימה שלך, וארנק זה ממשיך לעקוב אחר ה-ZEC שלה למקרה שעדיין יגיע; תוכל להסיר אותה מהרשימה בכל עת.';

  @override
  String get walletSwapStatusUnknownTitle => 'הסטטוס אינו זמין';

  @override
  String get walletSwapStatusUnknownBody =>
      'לא ניתן לקרוא כרגע את סטטוס ההחלפה הזו.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'המעקב אינו זמין';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'ההחלפה כבויה, ולכן לא ניתן לעקוב אחר זה כאן. כספים כלשהם יסולקו או יוחזרו בצד הספק.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'ההחלפה כבויה כאן, ולכן לא ניתן לעקוב אחר ההחלפה הזו כרגע. אם היא הוחזרה, ה-ZEC יחזור לארנק זה — הוא יופיע ביתרה שלך לאחר שההחלפה תופעל מחדש והארנק יסתנכרן.';

  @override
  String get walletSwapTrackingError => 'לא ניתן היה לעקוב אחר החלפה זו.';

  @override
  String get walletSwapTrackingErrorBody =>
      'לא ניתן היה לפתוח מעקב עבור החלפה זו. ייתכן שההחלפה עצמה עדיין מתקדמת — כספים שהופקדו יסולקו או יוחזרו בצד הספק.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'הזן את הכתובת שבה תרצה לקבל את הנכס המוחלף.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'כתובת יעד זו אינה תקינה עבור נכס זה. בדוק אותה ונסה שוב.';

  @override
  String get walletSwapFaultExpired =>
      'תוקף הצעת המחיר הזו פג. קבל הצעת מחיר חדשה כדי להמשיך.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'מחיר הספק נע מחוץ למגבלה שקבעת, ולכן ההחלפה נעצרה לפני שדבר זז. נסה שוב.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'מגבלת ההחלקה גבוהה מדי עבור החלפה בטוחה. נסה שוב.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'ספק ההחלפה אינו זמין כרגע. נסה שוב בעוד רגע.';

  @override
  String get walletSwapFaultConnection =>
      'לא ניתן היה להגיע לשירות ההחלפה. בדוק את החיבור לאינטרנט ונסה שוב.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'ספק ההחלפה החזיר תגובה בלתי צפויה, ולכן ההחלפה נעצרה. נסה שוב.';

  @override
  String get walletSwapFaultSwapOff => 'ההחלפה כבויה כרגע.';

  @override
  String get walletSwapFaultDepositFailed =>
      'לא הצלחנו לשלוח את ההפקדה שלך, כך שדבר לא יצא מהארנק שלך. קבל הצעת מחיר חדשה כדי לנסות שוב.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'יש כבר החלפה בתהליך. תוכל להתחיל חדשה לאחר שזו תיסלק במלואה או שתוקף הצעת המחיר שלה יפוג — זה עשוי לקחת זמן מה.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'ארנק זה עדיין אינו יכול להגדיר כתובת החזר — בדרך כלל זה פשוט אומר שהסנכרון הראשון עוד לא הסתיים. המתן עד שהסנכרון יושלם, ואז נסה שוב.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'ארנק זה עדיין אינו יכול להגדיר כתובת קבלה עבור החלפה זו — בדרך כלל זה פשוט אומר שהסנכרון הראשון עוד לא הסתיים. המתן עד שהסנכרון יושלם, ואז נסה שוב.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'ההחלפה לא הצליחה להתחיל בזמן — ייתכן שהחיבור היה איטי, או שהארנק היה עסוק. קבל הצעת מחיר חדשה ונסה שוב.';

  @override
  String get walletSwapFaultStoreBusyRetry => 'הארנק עסוק לרגע. נסה שוב.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'הצעת המחיר הזו אינה תואמת את זו שהארנק שלך הנפיק, ולכן לא נשלח דבר. קבל הצעת מחיר חדשה ונסה שוב.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'החלפה זו דורשת כ-$needed ZEC כולל עמלת הרשת, אך רק $spendable ZEC זמינים לשימוש כרגע.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'אפליקציה זו מגבילה כרגע החלפות ל-$limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'החלפה זו דורשת כ-$needed ZEC כולל עמלת הרשת, אך רק $spendable ZEC זמינים לשימוש כרגע. היתרה שלך עדיין משלימה פערים — ייתכן שבקרוב יהיה זמין יותר.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'הארנק לא הצליח לתעד החלפה זו באופן בטוח, כך שדבר לא זז. נסה שוב.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'לא ניתן היה לעבד את בקשת ההחלפה הזו. קבל הצעת מחיר חדשה ונסה שוב.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'לא ניתן היה לקבל הצעת מחיר להחלפה. בדוק את הפרטים ונסה שוב.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'הארנק שלך אינו מוכן כרגע. חזור אחורה ונסה שוב.';

  @override
  String get walletSwapDirectionBuy => 'קנה ZEC';

  @override
  String get walletSwapDirectionSell => 'מכור ZEC';

  @override
  String get walletSwapRefundLabel => 'כתובת ההחזר שלך';

  @override
  String get walletSwapRefundHint => 'לאן המטבעות שלך יחזרו אם ההחלפה תיכשל';

  @override
  String get walletSwapRefundHelper =>
      'בשרשרת שממנה אתה שולח — לא כתובת Zcash.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'כתובת ההחזר שלך ב-$chain';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'כתובת $chain — לשם יחזרו המטבעות שלך אם ההחלפה תיכשל. לא כתובת Zcash.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'על אודות כתובת ההחזר שלך';

  @override
  String get walletSwapRefundInfoBody =>
      'אם ההחלפה לא תושלם, הספק ישלח את המטבעות שלך בחזרה לכתובת זו בשרשרת שממנה שילמת. הזן כתובת שבשליטתך — הארנק אינו יכול לבדוק עבורך כתובת זרה, לכן ודא אותה בקפידה.';

  @override
  String get walletSwapRefundScanTooltip => 'סרוק קוד QR של כתובת החזר';

  @override
  String get walletSwapScanTitle => 'סריקת כתובת';

  @override
  String get walletSwapScanInstruction =>
      'כוון את המצלמה לעבר קוד ה-QR של הכתובת.';

  @override
  String get walletSwapScanManualEntry => 'הזנה ידנית';

  @override
  String get walletSwapScanCancel => 'ביטול';

  @override
  String get walletSwapScanCameraUnavailable =>
      'המצלמה אינה זמינה. הזן את הכתובת ידנית למטה.';

  @override
  String get walletSwapSourceAssetLabel => 'נכס להחלפה ממנו';

  @override
  String get walletSwapSourceAssetHint => 'בחר נכס';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'סכום לשליחה ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'סכום לשליחה';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol על $chain';
  }

  @override
  String get walletSwapPickerTitle => 'בחר נכס להחלפה ממנו';

  @override
  String get walletSwapPickerTitleReceive => 'בחר נכס לקבלה';

  @override
  String get walletSwapPickerStale =>
      'לא ניתן היה לרענן את רשימת הנכסים — מוצגת הרשימה האחרונה הידועה.';

  @override
  String get walletSwapPickerEmpty =>
      'אין כרגע נכסים זמינים להחלפה. נסה שוב מאוחר יותר.';

  @override
  String get walletSwapPickerSearchHint => 'חפש לפי שם או שרשרת';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'אין נכסים התואמים ל-\"$query\".';
  }

  @override
  String get walletSwapPickerError =>
      'לא ניתן היה לטעון את רשימת הנכסים. בדוק את החיבור שלך ונסה שוב.';

  @override
  String get walletSwapPickerRetry => 'נסה שוב';

  @override
  String get walletSwapSlippageLabel => 'סבילות להחלקת מחיר';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'מותאם אישית';

  @override
  String get walletSwapSlippageCustomLabel => 'החלקת מחיר מותאמת אישית';

  @override
  String get walletSwapSlippageMayFail =>
      'נמוכה מאוד — ההחלפה עלולה להיכשל אם המחיר ישתנה.';

  @override
  String get walletSwapSlippageNormal => 'סבילות בטוחה.';

  @override
  String get walletSwapSlippageRisky =>
      'גבוהה — ייתכן שתקבל פחות באופן ניכר מהמצוטט.';

  @override
  String get walletSwapSlippageTooHigh =>
      'גבוהה מדי — ההחלפה תידחה. הנמך אותה ל-10% או פחות.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'תקבל לפחות $zec ZEC — רף ההחלקה שלך הוא $slippage%. הסכום הסופי לא ירד מתחת לרף זה.';
  }

  @override
  String get walletSwapIntoZecShieldTitle => 'אתה מקבל ZEC לכתובת שלך עצמך';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'עד שתגן עליו — בהקשה אחת, בתזכורת עם ההגעה — הסכום שהתקבל יהיה ציבורי וגלוי בשרשרת לזמן קצר. הפקדה קטנה עשויה להישאר ציבורית עד שתצטבר.';

  @override
  String get walletSwapRefundVerifyTitle => 'אמת את כתובת ההחזר שלך';

  @override
  String get walletSwapRefundVerifyBody =>
      'בדוק אותה תו אחר תו — לכאן חוזרים המטבעות שלך אם ההחלפה תיכשל. הארנק אינו יכול לאמת עבורך כתובת זרה.';

  @override
  String get walletSwapRefundVerifyAck => 'בדקתי שכתובת ההחזר שלי נכונה.';

  @override
  String get walletSwapPayoutVerifyTitle => 'אמת את כתובת הקבלה שלך';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'בדוק אותה תו אחר תו — לכאן תקבל את $asset. הארנק אינו יכול לאמת עבורך כתובת זרה.';
  }

  @override
  String get walletSwapPayoutVerifyAck => 'בדקתי שכתובת הקבלה שלי נכונה.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'ההחלפה כבויה כאן. כל ZEC שכבר בדרך יופיע בארנק שלך לאחר הסנכרון הבא.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'הזן את הסכום שברצונך להחליף.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'הזן את כתובת ההחזר שלך בשרשרת המקור.';

  @override
  String get walletSwapDepositTitle => 'שלח את התשלום שלך';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'שלח בדיוק $amount $asset ב-$chain לכתובת שלמטה.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'שלח את הסכום המדויק. שליחת פחות, או שליחה לאחר סגירת החלון, פירושה שהספק יחזיר לך את הכסף לכתובת ההחזר שלך.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'חלון ההפקדה: נותרו $time';
  }

  @override
  String get walletSwapDepositExpired =>
      'חלון ההפקדה הזה נסגר. אל תשלח כספים כעת — התחל החלפה חדשה. אם כבר שלחת, הספק אמור להחזיר את הכסף לכתובת ההחזר שלך.';

  @override
  String get walletSwapDepositQrLabel => 'קוד QR של כתובת ההפקדה';

  @override
  String get walletSwapDepositAddressLabel => 'כתובת הפקדה';

  @override
  String get walletSwapDepositCopy => 'העתק כתובת הפקדה';

  @override
  String get walletSwapDepositCopied => 'כתובת ההפקדה הועתקה';

  @override
  String get walletSwapDepositMemoRequired => 'הפקדה זו דורשת הערה / תג';

  @override
  String get walletSwapDepositMemoWarning =>
      'עליך לכלול הערה זו במדויק עם ההפקדה שלך. שליחה בלעדיה — או עם הערה שגויה — עלולה לגרום לאובדן קבוע של הכספים שלך.';

  @override
  String get walletSwapDepositMemoLabel => 'הערה / תג הפקדה';

  @override
  String get walletSwapDepositMemoCopy => 'העתק הערה';

  @override
  String get walletSwapDepositMemoCopied => 'ההערה הועתקה';

  @override
  String get walletSwapDepositSent => 'שלחתי את הכספים';

  @override
  String get walletSwapDepositBackTitle => 'לצאת ממסך זה?';

  @override
  String get walletSwapDepositBackBody =>
      'פעולה זו לא תבטל את ההחלפה שלך — היא ממשיכה ברקע. אך תזדקק לכתובת ההפקדה כדי לשלם, אז העתק אותה קודם אם עדיין לא עשית זאת.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'פעולה זו לא תבטל את ההחלפה שלך — היא ממשיכה ברקע. חלון ההפקדה נסגר. אל תשלח כספים לכתובת ההפקדה כעת. אם כבר שלחת, הספק אמור להחזיר את הכסף לכתובת ההחזר שלך.';

  @override
  String get walletSwapDepositBackStay => 'הישאר';

  @override
  String get walletSwapDepositBackLeave => 'צא';

  @override
  String get walletReceive => 'קבלה';

  @override
  String get walletReceiveSubtitle =>
      'שתף כתובת זו כדי לקבל ZEC. בטוח לשתף אותה בפומבי.';

  @override
  String get walletReceiveCopy => 'העתק כתובת';

  @override
  String get walletReceiveCopied => 'הכתובת הועתקה';

  @override
  String get walletReceiveUnavailable => 'הארנק שלך עדיין לא מוכן.';

  @override
  String get walletReceiveError => 'לא הצלחנו לטעון את הכתובת שלך. נסה שוב.';

  @override
  String get walletReceivePreparing => 'מכין את הכתובת שלך…';

  @override
  String get walletReceivePreparingHint =>
      'הארנק שלך מכין את הכתובת הזו על המכשיר שלך — זה עשוי לקחת רגע אם הארנק עסוק במשימות אחרות.';

  @override
  String get walletReceiveRetry => 'נסה שוב';

  @override
  String get walletReceiveQrLabel => 'קוד QR של כתובת הקבלה שלך';

  @override
  String get walletReceiveTypeShielded => 'מוגן';

  @override
  String get walletReceiveTypeTransparent => 'ציבורי';

  @override
  String get walletReceiveSubtitleTransparent =>
      'שתף כתובת ציבורית זו כדי לקבל ZEC משולח שאינו יכול לשלם לכתובת מוגנת.';

  @override
  String get walletReceiveTransparentWarning =>
      'זוהי כתובת ציבורית: היא גלויה בבלוקצ\'יין ומקשרת בין התשלומים שלך אם תשתמש בה שוב. עדיף להשתמש בכתובת המוגנת שלך; הגן על כספים אלה לאחר קבלתם.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'קוד QR של כתובת הקבלה הציבורית שלך';

  @override
  String get walletReceiveFreshAddress => 'השתמש בכתובת חדשה';

  @override
  String get walletReceiveFreshCaption =>
      'כתובת חדשה — לא ניתן לקשר אותה לכתובות האחרות שלך. תשלומים אליה עדיין מגיעים לארנק הזה, והכתובות הקודמות שלך ממשיכות לעבוד. היא לא תוצג כאן שוב — העתק אותה עכשיו.';

  @override
  String get walletReceiveFreshError => 'לא הצלחנו ליצור כתובת חדשה. נסה שוב.';

  @override
  String get walletReceiveFreshBusy =>
      'הארנק עסוק כרגע. נסה שוב את הכתובת החדשה בעוד רגע.';

  @override
  String get walletReceiveShare => 'שיתוף';

  @override
  String get walletReceiveRequestAmount => 'בקשת סכום';

  @override
  String get walletReceiveRequestAmountLabel => 'סכום (אופציונלי)';

  @override
  String get walletReceiveFreshCopyNow =>
      'היא לא תוצג כאן שוב — העתק אותה עכשיו.';

  @override
  String get walletSecurityMenuItem => 'אבטחה…';

  @override
  String get securityTitle => 'אבטחה';

  @override
  String get securityUnavailableBody =>
      'הגדרות אבטחת הארנק מנוהלות על ידי אפליקציה זו, לא על ידי הארנק עצמו.';

  @override
  String get securityCustodySectionTitle => 'משמורת מפתחות';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (חומרה)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (חומרה)';

  @override
  String get securityCustodyTierTee => 'מחסן מפתחות חומרה (TEE)';

  @override
  String get securityCustodyTierSoftware => 'מחסן מפתחות תוכנה';

  @override
  String get securityCustodyTierKeychain => 'Keychain (מוצפן בתוכנה)';

  @override
  String get securityCustodyTierNone => 'אין מחסן מפתחות חומרה';

  @override
  String get securityCustodyTierUnknown => 'לא ידוע';

  @override
  String get securityCustodyHardwareKey =>
      'המפתח שנועל את הארנק הזה שמור בחומרה המאובטחת של מכשיר זה ונמחק יחד עם הארנק.';

  @override
  String get securityCustodyBestEffort =>
      'המחיקה מסירה את המפתחות שלך במאמץ מיטבי; ייתכן שיישאר חלון קצר לשחזור פורנזי עד שהמכשיר ישתלט מחדש על מקום האחסון. לביטחון מלא, השתמש גם באפשרות מחיקת כל התוכן של המכשיר שלך.';

  @override
  String get securityCustodyProbeError =>
      'לא ניתן היה לקרוא את סטטוס המשמורת. חזור אחורה ונסה שוב.';

  @override
  String get securityDeleteWalletButton => 'מחק ארנק';

  @override
  String get securityDeleteWalletSubtitle =>
      'מחק ארנק זה ואת המפתח שלו מהמכשיר הזה. הכספים שלך נשארים בבלוקצ\'יין וניתנים לשחזור מביטוי השחזור שלך.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'מחק ארנק זה ואת המפתח שלו מהמכשיר הזה. הוא אינו מחזיק במפתחות הוצאה — אין כאן דבר לגבות, ניתן להוסיף אותו מחדש בכל עת עם מפתח הצפייה שלו.';

  @override
  String get securityDeleteDialogTitle => 'למחוק ארנק זה?';

  @override
  String get securityDeleteDialogBody =>
      'פעולה זו מסירה את הארנק ואת המפתח שלו מהמכשיר הזה. ודא שגיבית את ביטוי השחזור שלך — זו הדרך היחידה לשחזר את הכספים שלך.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'פעולה זו מסירה את הארנק ואת המפתח שלו מהמכשיר הזה. הוא אינו מחזיק במפתחות הוצאה — אין כאן דבר לגבות, ניתן להוסיף אותו מחדש בהמשך עם מפתח הצפייה שלו.';

  @override
  String get securityDeleteDialogConfirm => 'מחק';

  @override
  String get securityDeleteDialogCancel => 'ביטול';

  @override
  String get securityDeleteFailedSnack =>
      'לא ניתן היה למחוק את הארנק — הארנק שלך לא השתנה. נסה שוב.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'סיים קודם את החלפת השרת — היא מסתיימת או נעצרת תוך $seconds שניות. לאחר מכן נסה שוב למחוק את הארנק.';
  }

  @override
  String get walletParkedTitle => 'נשמר וממתין';

  @override
  String get walletParkedSubtitle =>
      'תשלומים אלה עדיין לא נשלחו. הסכומים שלהם עדיין חלק מהיתרה שלך.';

  @override
  String get walletParkedSubtitlePreparing =>
      'תשלומים אלה עדיין לא נשלחו. הסכומים שלהם עדיין חלק מהיתרה שלך — למעט תשלום שהארנק שלך שולח כעת, שהסכום שלו עשוי כבר להיות משוריין.';

  @override
  String get walletParkedCancel => 'ביטול';

  @override
  String get walletParkedPausedHint =>
      'מושהה — תשלום זה לא יישלח מעצמו. הכספים שלך בטוחים. שלח אותו עכשיו, או בטל אותו.';

  @override
  String get walletParkedRetryStale =>
      'תשלום זה כבר אינו ממתין. בדוק את התשלומים הממתינים ואת הפעילות שלך.';

  @override
  String get walletParkedAlreadyInProgress =>
      'תשלום זה כבר אינו ממתין — ייתכן שהארנק שלך כבר שולח אותו. בדוק את \"נשמר וממתין\" ואת הפעילות שלך.';

  @override
  String get walletReclaimExplainer =>
      'שליחות בכתובות חד-פעמיות תקועות. אפשר לפתוח אותן מחדש — זה מעביר סכום קטן בין הכתובות שבבעלותך ומחזיר אותו.';

  @override
  String get walletReclaimButton => 'פתח מחדש את השליחה';

  @override
  String get walletReclaimInProgress => 'פותח מחדש…';

  @override
  String get walletReclaimConfirmTitle =>
      'לפתוח מחדש את השליחה בכתובות חד-פעמיות?';

  @override
  String get walletReclaimConfirmBody =>
      'פעולה זו מעבירה סכום קטן בין הכתובות שבבעלותך כדי לשחרר את השליחה בכתובות חד-פעמיות, ואז מחזירה אותו. היא עולה כמה עמלות רשת. לאחר האישור, שחזר את הסכום שהועבר באמצעות \"שחזר עכשיו\".';

  @override
  String get walletReclaimConfirmCancel => 'לא עכשיו';

  @override
  String get walletReclaimConfirmAction => 'פתח מחדש';

  @override
  String get walletReclaimStarted =>
      'פתיחה מחדש החלה. לאחר האישור, שלח את התשלום המושהה, ואז שחזר את הסכום שהועבר באמצעות \"שחזר עכשיו\".';

  @override
  String get walletReclaimNothing => 'אין כרגע מה לפתוח מחדש.';

  @override
  String get walletReclaimNotBroadcast =>
      'לא ניתן היה לאשר שההעברה הגיעה לרשת. ייתכן שהיא בכל זאת עברה. נסה שוב בעוד רגע.';

  @override
  String get walletReclaimNeedsFunds =>
      'אתה צריך קצת ZEC מוגן כדי לפתוח מחדש את השליחה.';

  @override
  String get walletReclaimFailed =>
      'לא ניתן היה לפתוח מחדש את השליחה כרגע. הכספים שלך לא השתנו. נסה שוב.';

  @override
  String get walletReclaimUnknown =>
      'פתיחה מחדש הסתיימה. בדוק את השליחות שלך בכתובות חד-פעמיות, ושחזר סכום שאולי הועבר באמצעות \"שחזר עכשיו\".';

  @override
  String get walletParkedError =>
      'לא ניתן היה לטעון כרגע את התשלומים הממתינים שלך.';

  @override
  String get walletParkedErrorRetry => 'נסה שוב';

  @override
  String get walletParkedErrorRetryInProgress => 'מנסה…';

  @override
  String get walletParkedCancelConfirmTitle => 'לבטל תשלום ממתין זה?';

  @override
  String get walletParkedCancelConfirmBody =>
      'פעולה זו מוחקת את התשלום השמור. הוא לא נשלח, כך שדבר לא יוצא מהארנק שלך — אך לא ניתן לבטל פעולה זו.';

  @override
  String get walletParkedCancelConfirmKeep => 'השאר אותו';

  @override
  String get walletParkedCancelConfirmDiscard => 'מחק את התשלום';

  @override
  String get walletParkedCancelDone => 'התשלום הממתין בוטל.';

  @override
  String get walletParkedCancelAlreadySending =>
      'ייתכן שתשלום זה כבר בדרכו — בדוק את הפעילות שלך.';

  @override
  String get walletParkedCancelFailed =>
      'לא ניתן היה לבטל כרגע. התשלום שלך לא השתנה. נסה שוב.';

  @override
  String get walletRecoverNow => 'שחזר עכשיו';

  @override
  String get walletRecoverConfirmTitle => 'לשחזר אל היתרה המוגנת שלך?';

  @override
  String get walletRecoverConfirmBody =>
      'פעולה זו בודקת את הכתובות החד-פעמיות שלך ומעבירה כל מה שנמצא אל היתרה המוגנת והפרטית שלך. בטוח להריץ אותה שוב בכל עת.';

  @override
  String get walletRecoverConfirmCancel => 'לא עכשיו';

  @override
  String get walletRecoverConfirmAction => 'שחזר';

  @override
  String get walletRecoverInProgress => 'משחזר…';

  @override
  String walletRecoverDone(String amount) {
    return 'משחזר $amount אל היתרה המוגנת שלך.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'משחזר $amount — חלק מהכספים עדיין זקוקים לניסיון נוסף.';
  }

  @override
  String get walletRecoverRetry =>
      'חלק מהכספים זקוקים לניסיון נוסף — הרץ את השחזור שוב.';

  @override
  String get walletRecoverTruncated =>
      'לא כל הכתובות החד-פעמיות נבדקו עדיין — הרץ שוב כדי לבדוק את השאר.';

  @override
  String get walletRecoverNothing => 'אין כרגע מה לשחזר.';

  @override
  String get walletRecoverFailed =>
      'לא ניתן היה לשחזר כרגע. הכספים שלך לא השתנו. נסה שוב.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount נשמר וממתין · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'ביטול תשלום $amount שנשמר $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount מושהה · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount בהכנה לשליחה · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'הארנק שלך מכין את התשלום הזה — ייתכן שהסכום שלו כבר משוריין. הכספים שלך בטוחים. אם התהליך לא יושלם, התשלום יחזור לרשימה מעצמו.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'הארנק שלך מכין את התשלום הזה — ייתכן שהסכום שלו כבר משוריין. הכספים שלך בטוחים, אך הוא יוכל להסתיים רק לאחר שהארנק שלך יסתנכרן שוב.';

  @override
  String get walletParkedSendNow => 'שלח עכשיו';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'שולח את תשלום $amount שנשמר $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'שליחת תשלום $amount שנשמר $time עכשיו';
  }

  @override
  String get walletParkedSendNowInProgress => 'שולח…';

  @override
  String get walletParkedAuthorizeSent => 'שולח את התשלום שלך עכשיו.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'שולח את התשלום שלך עכשיו. אם הוא לא יעבור, הארנק שלך יוכל להשלים את התשלום רק לאחר שיסתנכרן שוב.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'עדיין לא מוכן לשליחה. התשלום שלך שמור ולא השתנה.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'עדיין לא מוכן לשליחה. התשלום שלך שמור וכבר לא מושהה — נסה שוב את \"שלח עכשיו\" מאוחר יותר, או בטל אותו.';

  @override
  String get walletParkedAuthorizeFailed =>
      'לא ניתן היה לשלוח כרגע. התשלום שלך לא השתנה. נסה שוב.';

  @override
  String get walletTransparentFundsMenuItem => 'כספים ציבוריים…';

  @override
  String get walletTransparentFundsTitle => 'כספים ציבוריים';

  @override
  String get walletTransparentFundsIntro =>
      'כספים ציבוריים גלויים לציבור בבלוקצ\'יין — הסכום, הכתובות וההיסטוריה של המטבעות.';

  @override
  String get walletExpertToggleLabel => 'מתקדם: כספים ציבוריים';

  @override
  String get walletExpertToggleDescription =>
      'הצג פקדים מתקדמים להחזקת כספים ציבוריים וכיבוי ההגנה האוטומטית.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'הצג פקדים מתקדמים להחזקת כספים ציבוריים.';

  @override
  String get walletAutoShieldToggleLabel => 'הגן אוטומטית';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'כשהיתרה הציבורית שלך מגיעה ל-$minZec ZEC, היא מועברת אוטומטית אל היתרה המוגנת שלך. כשאפשרות זו כבויה, כספים ציבוריים נשארים גלויים לציבור עד שתגן עליהם בעצמך.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'לא ניתן היה לשמור את ההגדרה. נסה שוב.';

  @override
  String get walletAutoShieldIncomplete =>
      'ההגנה האוטומטית לא הושלמה — כספים אלה עדיין גלויים לציבור. תוכל להגן עליהם עכשיו.';

  @override
  String get walletSendPrivacyShielded =>
      'תשלום מוגן — הסכום והנמען נשארים פרטיים בשרשרת.';

  @override
  String get walletSendPrivacyTransparent =>
      'תשלום פומבי — הסכום והכתובות גלויים בבלוקצ\'יין.';

  @override
  String get walletActivityPublicBadge => 'גלוי לציבור בבלוקצ\'יין';

  @override
  String get walletShieldWalletEnded =>
      'פעילות הארנק הסתיימה. סגור ופתח מחדש כדי לנסות שוב.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'כספים ציבוריים חדשים מועברים אוטומטית אל היתרה המוגנת שלך ברגע שהם מגיעים ל-$minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'הגנה אוטומטית כבויה — כספים ציבוריים נשארים גלויים לציבור עד שתגן עליהם.';

  @override
  String get walletMoveAutoShieldNote =>
      'הגנה אוטומטית פעילה: לאחר שהכספים האלה יגיעו, הם יוגנו שוב אוטומטית (בתשלום עמלה נוספת). כדי לשמור עליהם ציבוריים, כבה תחילה את ההגנה האוטומטית תחת כספים ציבוריים.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'אחרי ההעברה הזו היתרה הציבורית שלך תהיה $amount ZEC — פחות מ-$floor ZEC הנדרשים כדי להגן עליה שוב. היא תישאר ציבורית עד שיגיעו כספים נוספים.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'אתה מעביר לכתובת הציבורית שלך עצמך. ההעברה הזו נשארת ברישום הפומבי לצמיתות.';

  @override
  String get walletTxDetailVisibility => 'נראות';

  @override
  String get walletTransparentFundsAutoDenied =>
      'ההגנה האוטומטית מושהית עבור הפעלה זו — היא לא אושרה. תוכל עדיין להגן ידנית.';

  @override
  String get walletDeepScanMenuItem => 'בדוק כתובות החלפה ישנות יותר…';

  @override
  String get walletMenuSyncNotRunningHint => 'הסנכרון לא פועל כרגע.';

  @override
  String get walletDeepScanTitle => 'בדוק כתובות החלפה ישנות יותר';

  @override
  String get walletDeepScanBody =>
      'אם שחזרת את הארנק הזה והוא השתמש בעבר הרבה בהחלפות, ייתכן שהכספים מההחלפות הישנות ביותר שלו יזדקקו לשלב נוסף כדי להימצא. הבדיקה הזו מחפשת אותם — כל מה שנמצא יופיע ביתרה שלך ככל שהארנק שלך מסתנכרן.';

  @override
  String get walletDeepScanCoverage =>
      'כתובות ההחלפה הישנות יותר שלך נבדקו עד כאן. אם עדיין חסרים לך כספים מהחלפה ישנה, כדאי לבדוק אף עמוק יותר.';

  @override
  String get walletDeepScanCoveragePending =>
      'עדיין בודקים את הטווח הנוכחי — כל מה שיימצא יופיע ביתרה שלך. זה עשוי לקחת קצת זמן.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'בודק אם יש כספים מההחלפות הישנות ביותר של הארנק שלך.';

  @override
  String get walletDeepScanCheckButton => 'בדוק כתובות ישנות יותר';

  @override
  String get walletDeepScanCheckDeeperButton => 'בדוק כתובות ישנות אף יותר';

  @override
  String get walletDeepScanChecking => 'בודק…';

  @override
  String get walletDeepScanClose => 'סגור';

  @override
  String get walletDeepScanTorHint =>
      'כרגע אינך מחובר דרך Tor. לפרטיות רבה יותר, כדאי להמתין עד ש-Tor יהיה פעיל לפני הבדיקה.';

  @override
  String get walletDeepScanRescanBusy =>
      'תוכל לבדוק כתובות החלפה ישנות יותר לאחר שהסריקה מחדש תסתיים.';

  @override
  String get walletDeepScanRan =>
      'בודקים כתובות החלפה ישנות יותר — כל מה שיימצא יופיע ביתרה שלך.';

  @override
  String get walletDeepScanFailed =>
      'לא ניתן היה להתחיל את הבדיקה. שום דבר לא השתנה — נסה שוב.';

  @override
  String get walletDeepScanSlow =>
      'זה לוקח יותר זמן מהרגיל. אם כתובות ההחלפה הישנות יותר שלך נבדקו, כל מה שיימצא יופיע ביתרה שלך — בדוק שוב בקרוב.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'ההחלפה כבויה כרגע, ולכן לא ניתן להריץ זאת. נסה שוב כשההחלפה תהיה זמינה.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'עדיין בודקים את הטווח האחרון — זה עשוי לקחת עד כיומיים, אך בדרך כלל הרבה פחות. זה מסתיים מעצמו; בדוק שוב מאוחר יותר.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'עדיין לא ניתן לאשר את פרטיות החיבור שלך. לפרטיות רבה יותר, כדאי לבדוק שוב ברגע ש-Tor יהיה פעיל.';

  @override
  String get walletDeepScanBannerChecking =>
      'עדיין בודקים כתובות החלפה ישנות יותר — כל מה שיימצא יופיע ביתרה שלך.';

  @override
  String get walletRescanSwapPointer =>
      'מחפש כספים מהחלפה ישנה? סריקה מחדש לא תמצא את זה — השתמש במקום זאת ב“בדוק כתובות החלפה ישנות יותר”.';

  @override
  String get walletDeepScanRestoreNoteTitle => 'שחזרת ארנק שהשתמש בהחלפות?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'אם לארנק הזה הייתה היסטוריית החלפות ארוכה מאוד, ייתכן שהכספים מההחלפות הישנות ביותר שלו יזדקקו לשלב נוסף כדי להימצא. רוב הארנקים אינם זקוקים לשום דבר.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'בדוק עכשיו';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'סגור';

  @override
  String walletTorHostPath(String transport) {
    return 'דרך הנתיב הפרטי של האפליקציה שלך ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'דרך הנתיב הפרטי של האפליקציה שלך ($transport); הפרוקסי יכול לקשר בין החיבורים';
  }

  @override
  String get walletTorHostOtherTransport => 'נתיב פרטי';

  @override
  String get walletTorHostDirect => 'לא פרטי (החיבור הישיר של האפליקציה שלך)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'השרת השמור משתמש בכתובת לא מוצפנת, שהנתיב הפרטי של האפליקציה שלך אינו יכול להעביר. נעשה שימוש ב-$host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'עוד על $label';
  }

  @override
  String get walletSendPaste => 'הדבקה';

  @override
  String get walletSendScanQr => 'סריקת קוד QR';

  @override
  String get walletSendRecipientGetsLabel => 'הנמען מקבל';

  @override
  String get walletSwapDepositCopyAmount => 'העתקת הסכום';

  @override
  String get walletSwapDepositAmountCopied => 'הסכום הועתק';

  @override
  String get walletScanOpenSettings => 'פתיחת ההגדרות';

  @override
  String get walletScanOpenSettingsFailed => 'לא ניתן לפתוח את ההגדרות.';

  @override
  String get walletSendLeaveTitle => 'עדיין בשליחה';

  @override
  String get walletSendLeaveBody =>
      'התשלום ממשיך גם אם תצא. תראה איך הסתיים בפעילות שלך.';

  @override
  String get walletSendLeaveStay => 'להישאר';

  @override
  String get walletSendLeaveConfirm => 'לצאת';

  @override
  String get walletSheetLeaveBody =>
      'הפעולה ממשיכה גם אם תצא. תראה איך הסתיימה בפעילות שלך.';

  @override
  String get walletLoadingLabel => 'טוען';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'בדוק לפני שתגן שוב';

  @override
  String get walletShieldUnknownBody =>
      'לא הצלחנו לאשר את ההגנה הזו. בדוק בפעילות לפני שתנסה שוב.';

  @override
  String get walletMoveUnknownTitle => 'בדוק לפני שתעביר שוב';

  @override
  String get walletMoveUnknownBody =>
      'לא הצלחנו לאשר את ההעברה הזו. בדוק בפעילות לפני שתנסה שוב.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
