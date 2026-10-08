// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class WalletLocalizationsJa extends WalletLocalizations {
  WalletLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get walletAppearanceMenuItem => '設定';

  @override
  String get walletTitle => 'ウォレット';

  @override
  String get walletNotSetUpTitle => 'ウォレットはまだ設定されていません';

  @override
  String get walletNotSetUpBody =>
      'ウォレットのセットアップは今後のビルドで提供されます。セットアップでは、資金を受け取れるようになる前にリカバリーフレーズを控える手順をご案内します。バックアップがない限り、資産が危険にさらされることはありません。';

  @override
  String get walletStartupFailedTitle => 'ウォレットを起動できませんでした';

  @override
  String get walletStartupFailedBody =>
      'この端末でのウォレットの読み込みが、何らかの理由で妨げられました。すでにウォレットをお持ちの場合、その資金への影響はありません — 資金はZcashネットワーク上に存在し、リカバリーフレーズで復元できます。もう一度お試しください。問題が続く場合は、アプリを一度閉じてから開き直してください。';

  @override
  String get walletBalanceLabel => '残高';

  @override
  String get walletHideBalance => '残高を隠す';

  @override
  String get walletShowBalance => '残高を表示';

  @override
  String get walletBalanceHiddenAmount => '残高は非表示';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => '今すぐ使用可能';

  @override
  String get walletArrivingLabel => '入金予定';

  @override
  String get walletNotSpendableYetLabel => 'まだ使えません';

  @override
  String get walletActivityTitle => '履歴';

  @override
  String get walletActivityEmpty => 'まだ履歴がありません';

  @override
  String get walletActivityError => '履歴を読み込めませんでした';

  @override
  String get walletActivityReceived => '受取';

  @override
  String get walletActivitySent => '送金';

  @override
  String get walletActivityPending => '保留中';

  @override
  String get walletActivityQueued => '送信待ち';

  @override
  String get walletActivityRetrying => '再試行中';

  @override
  String get walletActivitySaved => '保存済み';

  @override
  String get walletActivityExpired => '期限切れ';

  @override
  String get walletActivityFailed => '失敗';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '承認$count件',
      one: '承認1件',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '入金が$count件ありました',
      one: '入金がありました',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => '取引の詳細を表示';

  @override
  String get walletTxDetailStatus => 'ステータス';

  @override
  String get walletTxDetailFee => 'ネットワーク手数料';

  @override
  String get walletTxDetailDate => '日付';

  @override
  String get walletTxDetailHeight => 'ブロック高';

  @override
  String get walletTxDetailMemo => 'メモ';

  @override
  String get walletTxDetailMemoAttached => 'あり';

  @override
  String get walletTxDetailTxid => '取引ID';

  @override
  String get walletTxDetailCopyTxid => '取引IDをコピー';

  @override
  String get walletTxDetailCopied => '取引IDをコピーしました';

  @override
  String get walletTxDetailClose => '閉じる';

  @override
  String get walletTxFundsKept => '資金はウォレットから出ていません';

  @override
  String get walletTxExplainQueued =>
      'この端末に保存されており、「保存済み・保留中」にあります — そこから送信またはキャンセルできます。';

  @override
  String get walletTxExplainPending => 'Zcashネットワークに送信済みです。ブロックでの承認をお待ちください。';

  @override
  String get walletTxExplainRetrying =>
      'ウォレットはまだこれをZcashネットワークに送信できていません。署名済みのトランザクションを保持し、送信できるか期限切れになるまで同期のたびに再試行します。';

  @override
  String get walletTxExplainSaved =>
      'ウォレットはこの署名済みトランザクションを保持していますが、現在は自動では送信しません。';

  @override
  String get walletTxExplainConfirmed => 'Zcashネットワーク上で承認されました。';

  @override
  String get walletTxExplainExpired =>
      'この取引はネットワークが承認する前に期限切れとなり、キャンセルされました。金額は引き続きご利用いただけます。';

  @override
  String get walletTxExplainFailed =>
      'ネットワークがこの取引を拒否したため、送信されませんでした。金額は引き続きご利用いただけます。';

  @override
  String get walletTxExplainUnknown => 'この取引の現在のステータスを確認できません。次回の同期後に更新されます。';

  @override
  String get walletMenuTooltip => 'その他のオプション';

  @override
  String get walletRescanMenuItem => '履歴を再スキャン…';

  @override
  String get walletCheckOneTimeMenuItem => '使い捨てアドレスを確認…';

  @override
  String get walletRescanTitle => '履歴を再スキャン';

  @override
  String get walletRescanBody =>
      '古い資金が見つかりませんか?ブロックチェーンをさらにさかのぼって再スキャンすると、開始日の設定によりスキップされた入金を復元できます。資金とリカバリーフレーズが危険にさらされることはありません。';

  @override
  String get walletRescanRangeTitle => 'どこまでさかのぼってスキャンするか';

  @override
  String get walletRescanRangeAll => '履歴全体をスキャンします。最も時間はかかりますが、すべて復元できます。';

  @override
  String get walletRescanRangeDefault =>
      'ウォレットの開始時点からスキャンします。それでも古い資金が見つからない場合は、より前の日付を選択するか、履歴をすべてスキャンしてください。';

  @override
  String get walletRescanRangeResolving => 'おすすめの範囲を準備しています…';

  @override
  String walletRescanEstimate(String blocks) {
    return '約$blocksブロックをスキャンします。';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return '$date以降をスキャンします。それでも古い資金が見つからない場合は、より前の日付を選択するか、履歴をすべてスキャンしてください。';
  }

  @override
  String get walletRescanPick => '日付を選択';

  @override
  String get walletRescanChange => '日付を変更';

  @override
  String get walletRescanScanAll => '履歴をすべてスキャン';

  @override
  String get walletRescanDatePick => 'スキャンする最も古い日付';

  @override
  String get walletRescanWarning =>
      'ブロックチェーンを再スキャンします。最近の日付なら数分ですが、大幅にさかのぼる場合は数時間かかることがあります。同期はバックグラウンドで実行されるため、その間もウォレットを引き続き使用できます。';

  @override
  String get walletRescanSettlingAdvisory =>
      'このウォレットからの支払いがまだ確定していません。通常、完了するまでウォレットは再スキャンを拒否します — 試すことはできますが、拒否されるものとお考えください。';

  @override
  String get walletRescanConfirm => '再スキャンを開始';

  @override
  String get walletRescanCancel => 'キャンセル';

  @override
  String get walletRescanRunning => '再構築中…';

  @override
  String get walletRescanRebuildingAll =>
      '履歴を再構築中です。チェーン全体をスキャンしています。残高と履歴は、追いつくにつれて表示されます。';

  @override
  String walletRescanRebuildingFrom(String date) {
    return '$date以降の履歴を再構築中です。残高と履歴は、追いつくにつれて表示されます。';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'ウォレットの開始時点からの履歴を再構築中です。残高と履歴は、追いつくにつれて表示されます。';

  @override
  String get walletCatchUpBanner =>
      '追いついています。ウォレットの同期が進むにつれて残高と履歴が表示されます。受け取ったものはすべて安全です。';

  @override
  String get walletCatchUpRescanBanner =>
      '再スキャン後、履歴を再構築中です。残高と履歴は、追いつくにつれて表示されます。受け取ったものはすべて安全です。';

  @override
  String get walletRescanFailedNotice =>
      '現在、再スキャンできませんでした。資金は安全ですが、残高と履歴が追いつくまで少し時間がかかる場合があります。しばらくしてからもう一度お試しください。';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'お支払いがまだ確定していないため、資金を保護する目的で再スキャンを一時停止しています。ウォレットに変更はありません。数時間後にもう一度お試しいただき、その間はアプリを開いたままオンラインに保ってください。';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      '再スキャンは、ウォレットの同期に合わせて履歴を再構築します。同期は現在実行されていません。ウォレットに変更はありません。同期が実行されたら、もう一度お試しください。';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'ウォレット履歴を再構築するための空き容量が不足しています。資金は安全ですが、残高と履歴が追いつくまで少し時間がかかる場合があります。空き容量を確保して、もう一度お試しください。';

  @override
  String get walletRescanFailedDismiss => '閉じる';

  @override
  String get walletActivityRebuilding => '履歴を再構築中…';

  @override
  String get walletActivityCatchingUp => 'まだ追いつき中です。受け取ったものはここに表示されます。';

  @override
  String get walletActivitySyncNotRunning => '同期が実行されると、残高と履歴の読み込みが完了します。';

  @override
  String get walletActivityLoadMore => 'さらに読み込む';

  @override
  String get walletPendingChangeLabel => '保留中のおつり';

  @override
  String get walletTransparentLabel => '非シールド(公開)';

  @override
  String get walletTransparentNote =>
      '「今すぐ使用可能」には含まれません — この資金を使用するにはシールドしてください。それまでの間、チェーン上で公開されたままになります。';

  @override
  String get walletTransparentNoteWatchOnly => 'これらの資金はチェーン上で公開されたままになります。';

  @override
  String walletPoolShielded(String amount) {
    return 'シールド済み $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return '公開 $amount';
  }

  @override
  String get walletPoolAllShielded => 'すべてシールド済み · 非公開';

  @override
  String get walletPoolTapHint => '公開の資金を表示';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '残高のうち$amountは使い捨てアドレスにあります(回収可能)。';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '残高のうち$amountは使い捨てアドレスにあります。';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '合計$amountが、ウォレットが管理する使い捨てアドレスを通じて完了中の支払いに充てられています。再送信しないでください。',
      one: '$amountは、ウォレットが管理する使い捨てアドレスを通じて完了中の支払いに充てられています。再送信しないでください。',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '合計$amountが、ウォレットが管理する使い捨てアドレスを通じた支払いの途中で充てられています。ウォレットが再び同期するまで一時停止しています。再送信しないでください。',
      one:
          '$amountは、ウォレットが管理する使い捨てアドレスを通じた支払いの途中で充てられています。ウォレットが再び同期するまで一時停止しています。再送信しないでください。',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      '支払いがまだ完了処理中かどうかを確認できませんでした。再試行しています。それまでは、再送信する前にアクティビティで保留中の支払いがないか確認してください。';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '残高のうち$amountは使い捨てアドレスにあります(承認待ち)。';
  }

  @override
  String get walletShieldButton => 'シールド';

  @override
  String get walletShieldSheetTitle => '公開の資金をシールド';

  @override
  String get walletShieldNote => 'これにより、チェーン上で公開されている残高から、非公開のシールド残高へ資金を移動します。';

  @override
  String get walletShieldPreparing => '準備中…';

  @override
  String get walletShieldAmountLabel => 'シールド対象';

  @override
  String get walletShieldFeeLabel => 'ネットワーク手数料';

  @override
  String get walletShieldNetLabel => 'シールド後の金額';

  @override
  String get walletShieldConfirmButton => '今すぐシールド';

  @override
  String get walletShieldSubmitting => 'シールド中…';

  @override
  String get walletShieldNothingTitle => 'シールドできる資金がありません';

  @override
  String get walletShieldNothingBody =>
      'この資金は現在シールドするには少なすぎます。ネットワーク手数料の方が高くつきます。もう少し資金が増えればシールドできるようになります。';

  @override
  String get walletShieldDoneTitle => 'シールドを送信しました';

  @override
  String get walletShieldDoneBody => '資金はシールド残高へ移動中です。まもなくチェーン上で承認されます。';

  @override
  String get walletShieldSavedTitle => '保存しました — シールドを完了します';

  @override
  String get walletShieldSavedBody =>
      '現在ネットワークに接続できませんでした。シールドの内容は保存されており、後の同期でウォレットが完了させます。資金が失われることはありません。';

  @override
  String get walletShieldAlreadyTitle => '送信済みです';

  @override
  String get walletShieldFailedTitle => '現在シールドできませんでした';

  @override
  String get walletShieldStaleBody => 'ウォレットはまだ同期中です。しばらくしてからもう一度シールドをお試しください。';

  @override
  String get walletShieldTransientBody =>
      'シールドを今すぐ準備できませんでした。しばらくしてからもう一度お試しください。';

  @override
  String get walletShieldStorageFullBody =>
      '今すぐシールドするための空き容量が不足しています。空き容量を確保して、もう一度お試しください。資金は安全です。';

  @override
  String get walletShieldClose => '閉じる';

  @override
  String get walletShieldRetry => 'もう一度試す';

  @override
  String get walletMoveMenuItem => '公開アドレスへ移動…';

  @override
  String get walletMoveSheetTitle => '公開アドレスへ移動';

  @override
  String get walletMoveSheetSubtitle =>
      'シールドされたZECを、ご自身の公開アドレスへ送ります。シールドされた入金を受け付けない取引所への入金などに便利です。';

  @override
  String get walletMoveDestinationLabel => 'ご自身の公開アドレス';

  @override
  String walletMoveAvailable(String amount) {
    return '移動可能な金額: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return '移動可能な金額: $amount ZEC。残高はまだ追いつき中です。';
  }

  @override
  String get walletMoveDeshieldTitle => 'この移動により資金が公開されます';

  @override
  String get walletMoveDeshieldBody =>
      '公開アドレスへ移動すると、この資金はシールド残高から出ます。金額とご自身の公開アドレスは、Zcashブロックチェーン上で公開され、誰でも閲覧できるようになります。';

  @override
  String get walletMoveWalletEnded =>
      'ウォレットのセッションが終了しました。閉じてから開き直し、もう一度お試しください。';

  @override
  String get walletMoveLoading => '準備中…';

  @override
  String get walletMovePreparing => '金額を確認中…';

  @override
  String get walletMoveSubmitting => '移動中…';

  @override
  String get walletMoveReviewButton => '確認';

  @override
  String get walletMoveCancel => 'キャンセル';

  @override
  String get walletMoveReviewTitle => '移動内容を確認';

  @override
  String get walletMoveOwnAddressNote =>
      'ご自身の公開アドレスへ移動します。この資金は後で再度シールドできますが、この移動の記録は公開台帳に永久に残ります。';

  @override
  String get walletMoveConfirmButton => '公開アドレスへ移動';

  @override
  String get walletMoveBackButton => '戻る';

  @override
  String get walletMoveDoneTitle => '公開アドレスへ移動しました';

  @override
  String get walletMoveDoneBody => '資金は公開アドレスへ移動中です。まもなくチェーン上で承認されます。';

  @override
  String get walletMoveSavedTitle => '保存しました — 移動を完了します';

  @override
  String get walletMoveSavedBody =>
      'この移動は保存されており、後の同期でウォレットが送信します。資金は失われませんでした。';

  @override
  String get walletMoveAlreadyTitle => '送信済みです';

  @override
  String get walletMoveAlreadyBody => 'この資金はすでに送信済みで、公開アドレスへ向かっています。';

  @override
  String get walletMoveFailedTitle => 'この移動を完了できませんでした';

  @override
  String get walletMoveNothingTitle => '移動できる資金がありません';

  @override
  String get walletMoveNothingBody =>
      '現在、移動可能なシールド残高がありません。資金が承認されると、公開アドレスへ移動できるようになります。';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'ウォレットはまだ追いつき中です。受け取ったものは、同期が完了すると移動可能になります。';

  @override
  String get walletMoveCouldNotLoad => '公開アドレスを読み込めませんでした。もう一度お試しください。';

  @override
  String get walletMoveRetry => 'もう一度試す';

  @override
  String get walletMoveClose => '閉じる';

  @override
  String get walletSnapshotUnavailable => '現在ウォレットを読み込めませんでした。自動的に更新されます。';

  @override
  String get walletBalanceStale => '更新できませんでした — 最後に確認できた残高を表示しています。';

  @override
  String get walletSyncStartFailed => '同期を開始できませんでした。引き続き自動的に再試行します。';

  @override
  String get walletSyncRetry => 'もう一度試す';

  @override
  String get walletSyncTryNow => '今すぐ試す';

  @override
  String get walletSyncIdle => 'まだ同期していません';

  @override
  String get walletSyncIdleDetail => '同期は自動的に開始されます。';

  @override
  String get walletSyncDisabled => '同期オフ';

  @override
  String get walletSyncDisabledDetail => '残高を更新するには、このアプリの設定で同期をオンにしてください。';

  @override
  String get walletSyncExplainDisabled =>
      '同期はこのアプリの設定でオフになっています。資金は安全です。残高と履歴は最後に同期した状態を表示しており、同期がオンになるまで更新されません。';

  @override
  String get walletParkedSyncPausedNote =>
      'ウォレットが同期していないため、これらは自動的には送信されません。「今すぐ送金」でご自身で送信してください。';

  @override
  String get walletSyncPausedMoneyNote => 'ウォレットが再び同期するまで一時停止しています。';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body$note';
  }

  @override
  String get walletSyncStarting => '接続中…';

  @override
  String get walletSyncStartingDetail => 'Zcashネットワークに接続し、スキャンの準備をしています。';

  @override
  String get walletSyncConnecting => '接続中…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return '接続中… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'スキャン中 $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'スキャン中…';

  @override
  String get walletSyncSpendableReady => '資金は使用可能です。';

  @override
  String get walletSyncCatchingUp =>
      'ネットワークに追いついています — 初回の大規模な同期には時間がかかることがあります。完了するまでの間もアプリを引き続き使用できます';

  @override
  String walletSyncScanRemaining(String count) {
    return '残り$countブロック';
  }

  @override
  String get walletSyncUpToDate => '最新の状態です';

  @override
  String get walletSyncOffline => 'オフライン';

  @override
  String get walletSyncOfflineDetail => '送信待ちの支払いは「保存済み・保留中」に保存されたままになります。';

  @override
  String get walletSyncUnknown => '同期中…';

  @override
  String get walletSyncStalled => '同期が一時停止しています';

  @override
  String get walletStallEndpoint =>
      '現在Zcashネットワークに接続できません。自動的に再試行を続けます — インターネット接続をご確認いただくか、サーバーが一時的に利用できない可能性があります。';

  @override
  String get walletStallTor =>
      'アプリのプライベート経路が利用できないため、ウォレットは接続していません。アプリのネットワーク設定を確認するか、プライベート経路をオフにしてください。経路が戻り次第、同期が再開します。';

  @override
  String get walletStallStorage => '端末のストレージ容量が不足しています。空き容量を確保すると同期が再開されます。';

  @override
  String get walletStallReorg => 'チェーンの再編成が発生したため、最近のブロックを再確認しています。';

  @override
  String get walletStallInternal =>
      '端末側の問題により同期が停止しました。繰り返し発生する場合は、リカバリーフレーズから復元してください。';

  @override
  String get walletStallEndpointMisbehaving =>
      'このサーバーが正しいはずのないデータを送信したため、同期を停止しました。接続の問題ではありません。別のサーバーに切り替えてください。どのサーバーでも拒否される場合は履歴を再スキャンしてください。ウォレットが以前のサーバーの誤った記録を保持している可能性があります。';

  @override
  String get walletStallBirthdayInFuture =>
      'このウォレットは、このサーバーがまだ到達していないブロックから開始するよう設定されています。このウォレットに設定されている開始ブロックを確認するか、別のサーバーをお試しください。';

  @override
  String get walletStallStorageUnavailable => 'この端末で同期が一時停止しています。再試行しています。';

  @override
  String get walletStallUnknown => '不明な理由により同期が停止しました。';

  @override
  String get walletSyncBadgeHint => '同期の詳細を表示';

  @override
  String get walletSyncSheetClose => '閉じる';

  @override
  String get walletSyncSheetProgress => '進捗';

  @override
  String get walletSyncSheetBlocksLeft => '残りブロック数';

  @override
  String get walletSyncSheetSyncedTo => '同期済みブロック';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '少なくとも$blocksブロック遅れています',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle => '同期はまだ開始されていません — 自動的に開始されます。操作は必要ありません。';

  @override
  String get walletSyncExplainStartFailed =>
      '同期を開始できませんでした。資金は安全です — ウォレットが新しいアクティビティを確認していないだけです。下のボタンからもう一度お試しいただくか、アプリを開き直してください。';

  @override
  String get walletSyncExplainStarting =>
      'ウォレットがZcashネットワークに接続し、スキャンの準備をしています。通常は数秒で完了します。';

  @override
  String get walletSyncExplainConnecting => 'Zcashネットワークへの接続を確立しています。';

  @override
  String get walletSyncExplainScanning =>
      'ウォレットがブロックチェーンのブロックを確認し、資金を検出しています。新しい取引が見つかると残高と履歴が更新されます — 完了するまでの間もアプリを引き続き使用できます。';

  @override
  String get walletSyncExplainUpToDate => 'Zcashネットワークと完全に同期しています。残高と履歴は最新です。';

  @override
  String get walletSyncExplainStalled => '同期で問題が発生し、一時停止しています。自動的に再試行されます。';

  @override
  String get walletSyncExplainStalledOffline =>
      'Zcashネットワークに接続できません — オフラインの場合はこれは正常な状態です。または、サーバーが一時的に利用できない可能性があります。資金は安全です。残高は最後に同期した状態を表示しており、送信待ちの支払いは「保存済み・保留中」に保存されたままになります。接続は自動的に再試行されます。';

  @override
  String get walletSyncExplainOffline =>
      'ネットワークに接続されていません。資金は安全です — 残高は最後に同期した状態を表示しており、送信待ちの支払いは「保存済み・保留中」に保存されたままになります。';

  @override
  String get walletSyncExplainUnknown => 'ウォレットは同期中です。進行に伴い残高と履歴が更新されます。';

  @override
  String get walletTorOff => 'Torオフ';

  @override
  String get walletTorBootstrapping => 'プライベート経路を起動中…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport起動中…';
  }

  @override
  String get walletTorActive => 'Tor有効';

  @override
  String get walletTorActiveUnverified => 'Tor有効(未検証の実行環境)';

  @override
  String get walletTorActiveUnattested => 'プライベート経路を使用中（プライバシーは未検証）';

  @override
  String get walletTorFellBack => 'Torが利用できません — 直接接続を使用中';

  @override
  String get walletTorUnavailable => 'プライベート経路が利用できません — 未接続';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transportが利用できません — 未接続';
  }

  @override
  String get walletTorUnanswered => 'プライベート経路に接続済み — 応答がありません';

  @override
  String get walletTorUnansweredUnattested =>
      'プライベート経路に接続済み — 応答がありません（プライバシーは未検証）';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transportに接続済み — 応答がありません';
  }

  @override
  String get walletTorUnansweredDirect => '非プライベート（アプリの直接接続） — 応答がありません';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return '$transport経由で接続済み — 応答がありません。接続がプロキシで関連付けられる可能性があります';
  }

  @override
  String get walletTorUnknown => 'Torの状態が不明です — 保護されていないものとして扱ってください';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return '残高(ブロック$height時点)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return '残高 · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return '残高(ブロック$height、$time時点)';
  }

  @override
  String get walletSyncSheetConnection => '接続';

  @override
  String get walletSyncSheetServer => 'サーバー';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'サーバー、$host、サーバー選択を開きます';
  }

  @override
  String get walletSyncServerSheetTitle => '同期サーバー';

  @override
  String get walletSyncServerInUse => '使用中';

  @override
  String get walletSyncServerAppDefault => 'アプリの既定';

  @override
  String get walletSyncServerCustom => 'カスタムサーバー…';

  @override
  String get walletSyncServerCustomHint => 'https://ホスト:ポート';

  @override
  String get walletSyncServerCheck => 'サーバーを確認';

  @override
  String get walletSyncServerChecking => '確認中…';

  @override
  String get walletSyncServerUse => 'このサーバーを使用';

  @override
  String get walletSyncServerSwitching => '切り替え中…';

  @override
  String get walletSyncServerContinue => '続行';

  @override
  String get walletSyncServerCancel => 'キャンセル';

  @override
  String get walletSyncServerTrustTitle => 'このサーバーを信頼しますか？';

  @override
  String get walletSyncServerTrustNotice =>
      'このサーバーに残高と履歴の報告、および送金の中継を任せることになります。Torがオフの場合はあなたのIPアドレス、ウォレットのおおよその作成時期、ウォレットが確認する公開アドレス、ウォレットが照会する取引、送信する取引がサーバーに見えます。';

  @override
  String get walletSyncServerKeyLabel => 'アクセスキー（任意）';

  @override
  String get walletSyncServerKeyHeaderLabel => 'キーのヘッダー';

  @override
  String get walletSyncServerKeyHeaderNeeded => 'サーバーが求めるヘッダーを入力してください';

  @override
  String get walletSyncServerKeyInvalid => 'このキーまたはヘッダーは使用できません';

  @override
  String get walletSyncServerKeySaved => 'キーを保存しました';

  @override
  String get walletSyncServerKeyShow => '表示';

  @override
  String get walletSyncServerKeyHide => '非表示';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'このキーによって、このサーバーはあなたを識別できます。Tor経由でも、あなたの送金をウォレットと結び付けられる可能性があります。';

  @override
  String get walletSyncServerSwitchNotice =>
      '切り替えると進行中の同期がやり直されます。残高と履歴はそのまま保持されます。新しいサーバーのスキャンが追いつくまで、資金は入金予定と表示されることがあります。';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      '切り替えると新しいサーバーに再接続します。残高と履歴はそのまま保持されます。';

  @override
  String get walletSyncServerUnreachable =>
      'このサーバーに接続できませんでした。アドレスをご確認ください — 正しい場合は、このサーバーが応答していないか、アプリが今このサーバーに到達できていません。もう一度お試しいただくか、別のサーバーをお選びください。';

  @override
  String get walletSyncServerUnreachableOffered =>
      'このサーバーに接続できませんでした。このサーバーが応答していないのか、アプリが今このサーバーに到達できていないのかは、ウォレットには区別できません。別のサーバーをお選びいただくか、しばらくしてからお試しください。';

  @override
  String get walletSyncServerWrongNetwork => 'このサーバーは別のZcashネットワークにあります。';

  @override
  String get walletSyncServerInvalidUrl =>
      'サーバーのアドレスとして正しくないようです。https://ホスト:ポート の形式で入力してください。';

  @override
  String get walletSyncServerNotOffered => 'このサーバーはこのアプリでは提供されていません。';

  @override
  String get walletSyncServerBusy => 'ウォレットは現在処理中です。しばらくしてからお試しください。';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return '選択したサーバーはこのアプリで提供されなくなりました。$host を使用しています。';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return '保存されたサーバーの選択を読み取れませんでした。$host を使用しています。';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return '切り替えできませんでした。引き続き $host を使用しています。';
  }

  @override
  String get walletTransportExplainDirect =>
      'ウォレットの通信はサーバーへ直接接続されます。サーバーはあなたのIPアドレスを把握できます。';

  @override
  String get walletTransportExplainTor =>
      'ウォレットの通信はTorネットワークを経由しており、サーバーからあなたのIPアドレスを隠します。';

  @override
  String get walletTransportExplainBootstrapping =>
      'アプリのプライベート経路が起動しています。ウォレットの通信は、起動が完了するまで接続を待機します。';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transportが起動しています。ウォレットの通信は、起動が完了するまで接続を待機します。';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Torに接続できなかったため、通信は直接接続に切り替わりました。サーバーはあなたのIPアドレスを把握できます。';

  @override
  String get walletTransportExplainUnavailable =>
      'アプリのプライベート経路が利用できないため、ウォレットは接続していません。プライベート経路をオフにするか、アプリのネットワーク設定を確認してください。';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transportが利用できないため、ウォレットは接続していません。オフにするか、アプリのネットワーク設定を確認してください。';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'プライベート経路は接続を受け付けましたが、1分間なにも返ってきていません。経路側かウォレットサーバー側かは、ウォレットには区別できません。再試行を続けます。解消しない場合は、別のサーバーを試すか、アプリのネットワーク設定を確認してください。';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transportは接続を受け付けましたが、1分間なにも返ってきていません。経路側かウォレットサーバー側かは、ウォレットには区別できません。再試行を続けます。解消しない場合は、別のサーバーを試すか、アプリのネットワーク設定を確認してください。';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'ウォレットの通信はサーバーへ直接接続されます。サーバーはあなたのIPアドレスを把握できます。接続は受け付けられましたが、1分間なにも返ってきていません。経路側かウォレットサーバー側かは、ウォレットには区別できません。再試行を続けます。解消しない場合は、別のサーバーを試すか、アプリのネットワーク設定を確認してください。';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'この接続のプライバシーは検証できません — 非公開ではないものとして扱ってください。接続は受け付けられましたが、1分間なにも返ってきていません。経路側かウォレットサーバー側かは、ウォレットには区別できません。再試行を続けます。解消しない場合は、別のサーバーを試すか、アプリのネットワーク設定を確認してください。';

  @override
  String get walletTransportExplainUnverified =>
      'この接続のプライバシーは検証できません — 非公開ではないものとして扱ってください。';

  @override
  String get walletTransportExplainHostProxy =>
      'ウォレットの通信は、このアプリのプライバシー通信経路を経由しており、サーバーからあなたのIPアドレスを隠します。';

  @override
  String get walletOnboardingWelcomeTitle => 'ウォレットをセットアップ';

  @override
  String get walletOnboardingWelcomeBody =>
      '新しいウォレットを作成して、ZECを受け取り保管しましょう。リカバリーフレーズを生成し、資金が届く前にそのバックアップ手順をご案内します。バックアップがない限り、資産が危険にさらされることはありません。';

  @override
  String get walletCreateButton => '新しいウォレットを作成';

  @override
  String get walletRestoreButton => 'リカバリーフレーズから復元';

  @override
  String get walletWatchOnlyButton => 'ウォレットをウォッチする(ウォッチオンリー)';

  @override
  String get walletWatchOnlyTitle => 'ウォレットをウォッチする';

  @override
  String get walletWatchOnlyBody =>
      'ビューイングキーを貼り付けると、送金キーなしでウォレットをウォッチできます。残高や履歴を確認できますが、資金を送ることはできません。どこまで遡って確認すればよいかがわかるよう、ウォレットのおおよその開始日を選択してください。';

  @override
  String get walletWatchOnlyKeyLabel => 'ビューイングキー';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => 'ビューイングキーのQRコードをスキャン';

  @override
  String get walletWatchOnlyScanTitle => 'ビューイングキーをスキャン';

  @override
  String get walletWatchOnlyScanInstruction => 'ビューイングキーのQRコードにカメラを向けてください。';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'カメラを使用できません。代わりにキーを手動で貼り付けてください。';

  @override
  String get walletWatchOnlyScanManualEntry => '代わりに貼り付け';

  @override
  String get walletWatchOnlyScanHint =>
      'またはスキャンボタンをタップして、ビューイングキーのQRコードを読み取ってください。';

  @override
  String get walletWatchOnlyScanFilled => 'ビューイングキーをスキャンしました。';

  @override
  String get walletWatchOnlyBirthdayTitle => 'ウォレットの開始日';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return '$date以降をスキャンします — それより前に受け取った資金は表示されません。それより古いウォレットの場合は、より前の日付を選択してください。';
  }

  @override
  String get walletWatchOnlyBirthdayPick => 'ウォレットの開始日を選択してください';

  @override
  String get walletWatchOnlyBirthdayChange => '日付を変更';

  @override
  String get walletWatchOnlySubmit => 'このウォレットをウォッチする';

  @override
  String get walletWatchOnlyBack => '戻る';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      '有効なビューイングキーではないようです。確認のうえ、もう一度お試しください。';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'そのビューイングキーは別のネットワーク用です。ここでは使用できません。';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'この端末にはすでにウォレットが存在します。戻って開いてください。';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'その開始日は新しすぎます。より前の日付を選択してください。';

  @override
  String get walletRestoreTitle => 'ウォレットを復元';

  @override
  String get walletRestoreBody =>
      'リカバリーフレーズを入力してウォレットを復元してください — 単語を順番どおりに、スペースで区切って入力または貼り付けます。対応するのは標準的なフレーズのみです。追加のパスフレーズ(「25番目の単語」)を使用していたウォレットは、現時点ではこのアプリで復元できません。その場合、エラーではなく空のウォレットが表示されます。';

  @override
  String get walletRestorePhraseHint => '単語1  単語2  単語3  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count単語',
      one: '1単語',
      zero: 'まだ単語がありません',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint => 'リカバリーフレーズは12、15、18、21、24のいずれかの単語数です';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count個の単語がリカバリー単語ではありません — ハイライトされた単語を修正してください',
      one: '1つの単語がリカバリー単語ではありません — ハイライトされた単語を修正してください',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return '単語$index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return '単語$index: リカバリー単語ではありません';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return '単語$indexを削除';
  }

  @override
  String get walletRestoreSubmit => 'ウォレットを復元';

  @override
  String get walletRestoreBack => '戻る';

  @override
  String get walletRestoreBirthdayTitle => 'どこまでさかのぼってスキャンするか';

  @override
  String get walletRestoreBirthdayNone => '履歴全体をスキャンします。時間はかかりますが、見逃しはありません。';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return '$date以降をスキャンします — それより前に受け取った資金は表示されません。それより古いウォレットの場合は、より前の日付を選択するか、履歴をすべてスキャンしてください。';
  }

  @override
  String get walletRestoreBirthdayPick => '日付を選択';

  @override
  String get walletRestoreBirthdayChange => '日付を変更';

  @override
  String get walletRestoreBirthdayClear => '履歴をすべてスキャン';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return '単語$indexはリカバリー単語ではありません。フレーズの入力ミスを確認し、もう一度お試しください。';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'そのリカバリーフレーズは無効です。単語とその順序を確認し、もう一度お試しください。';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'そのフレーズはこの端末上のウォレットと一致しません。よく確認して、もう一度お試しください。';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'この端末にはすでにウォレットが存在します。戻って開いてください。';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'その日付は新しすぎます。より前の日付を選択するか、すべてスキャンしてください。';

  @override
  String get walletGeneratingLabel => 'ウォレットを作成中…';

  @override
  String get walletOpeningLabel => 'ウォレットを開いています…';

  @override
  String get walletBackupTitle => 'リカバリーフレーズをバックアップ';

  @override
  String get walletBackupBody =>
      'この単語列は、ウォレットと資金を復元できる唯一の手段です。順番どおりに書き留め、安全で人目につかない場所に保管してください。他人に教えたり、オンラインに保存したりしないでください — この単語列を知る者は誰でも資金を奪うことができます。';

  @override
  String get walletBackupSecureNoteAndroid => 'この画面ではスクリーンショットが無効になっています。';

  @override
  String get walletBackupSecureNoteOther => '他人に画面を見られないようにご注意ください。';

  @override
  String get walletBackupReveal => 'リカバリーフレーズを表示';

  @override
  String get walletBackupRevealing => 'リカバリーフレーズを準備中…';

  @override
  String get walletBackupRevealFailed =>
      '現在リカバリーフレーズを表示できませんでした。端末のロックが解除されていることを確認し、もう一度お試しください。';

  @override
  String get walletBackupRetryReveal => 'もう一度試す';

  @override
  String get walletBackupReauthFailed => '本人確認ができませんでした。もう一度お試しください。';

  @override
  String get walletBackupConfirmCheckbox => 'リカバリーフレーズを書き留め、安全に保管しました。';

  @override
  String get walletBackupContinue => '続ける';

  @override
  String get walletBackupSaveFailed => '確認内容を保存できませんでした。もう一度お試しください。';

  @override
  String get walletBackupStartOver => '最初からやり直す';

  @override
  String get walletBackupStartOverConfirmTitle => 'このウォレットを使わずに最初からやり直しますか?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'この操作により、このウォレットが端末から削除され、最初の画面に戻ります。セットアップが完了するまで、このアプリを通じて入金することはできません。\n\nこのウォレットに資金が入っていたことがある場合、またはリカバリーフレーズから復元されたものである場合、そのフレーズでしか取り戻せません。';

  @override
  String get walletBackupStartOverConfirm => '削除して最初からやり直す';

  @override
  String get walletBackupStartOverKeep => 'このウォレットを保持する';

  @override
  String get walletBackupSectionTitle => 'リカバリーフレーズ';

  @override
  String get walletBackupTileTitle => 'リカバリーフレーズをバックアップ';

  @override
  String get walletBackupTileSubtitle => 'ウォレットと資金を復元できる単語を表示します。';

  @override
  String get walletBackupScreenTitle => 'リカバリーフレーズ';

  @override
  String get walletBackupDone => '完了';

  @override
  String get walletBackupManagedTitle => '独自のリカバリーフレーズはありません';

  @override
  String get walletBackupManagedBody =>
      'このウォレットは、インストール元のアプリのアカウントを使ってセットアップされているため、独自のリカバリーフレーズはありません。資金はそのアカウントと一緒に復元されます — 資金を守るには、そのアカウントのバックアップをご利用ください。';

  @override
  String get walletExportViewingKeyTitle => 'ビューイングキーをエクスポート';

  @override
  String get walletExportViewingKeyTileTitle => 'ビューイングキーをエクスポート';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'ウォレットの閲覧専用コピーを共有します — 履歴の確認はできますが、送金はできません。';

  @override
  String get walletExportViewingKeyWarning =>
      'このキーを持つ人は、このウォレットがこれまでに受け取ったすべての取引と送ったすべての取引 — そして今後行われるすべての取引を見ることができます。資金を送ることはできず、ウォレットを復元することもできません。共有する相手は、会計士やご自身の別の端末など、全履歴を見せても構わないと信頼できる人に限ってください。共有を取り消す唯一の方法は、資金を新しいウォレットに移すことです。';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'このキーを持つ人は、このウォレットがこれまでに受け取ったすべての取引と送ったすべての取引 — そして今後行われるすべての取引を見ることができます。資金を送ることはできず、ウォレットを復元することもできません。共有する相手は、会計士やご自身の別の端末など、全履歴を見せても構わないと信頼できる人に限ってください。一度共有すると、共有を取り消すことはできません。';

  @override
  String get walletExportViewingKeyReveal => 'ビューイングキーを表示';

  @override
  String get walletExportViewingKeyRetry => 'もう一度試す';

  @override
  String get walletExportViewingKeyRevealing => 'ビューイングキーを準備しています…';

  @override
  String get walletExportViewingKeyFailed =>
      'ビューイングキーを表示できませんでした。しばらくしてからもう一度お試しください。';

  @override
  String get walletExportViewingKeyQrLabel => 'ビューイングキーのQRコード';

  @override
  String get walletExportViewingKeyCopy => 'ビューイングキーをコピー';

  @override
  String get walletExportViewingKeyCopied => 'ビューイングキーをコピーしました';

  @override
  String get walletExportViewingKeyDone => '完了';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'この画面ではスクリーンショットが無効になっています。';

  @override
  String get walletExportViewingKeySecureNoteOther => '他人に画面を見られないようにご注意ください。';

  @override
  String get walletWatchOnlySectionTitle => 'このウォッチオンリーウォレットについて';

  @override
  String get walletWatchOnlyAboutBody =>
      'これはウォッチオンリーウォレットです。ビューイングキーから設定されているため、残高や履歴は確認できますが、送金キーは保持していません — バックアップするものはなく、資金を送ることもできません。';

  @override
  String get walletWatchOnlyBadge => 'ウォッチオンリー';

  @override
  String get walletOnboardingFailedTitle => 'ウォレットのセットアップを完了できませんでした';

  @override
  String get walletOnboardingRetry => 'もう一度試す';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'スマートフォンのセキュアストレージが応答しません。端末のロックを解除して、もう一度お試しください。問題が続く場合は、スマートフォンを再起動してください。';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'このウォレットは別のウィンドウまたはアプリで開かれているか、前の操作をまだ完了していません。使用中の他のウィンドウを閉じるか、しばらく待ってから、もう一度お試しください。';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'このウォレットのセキュアキーが失われたため、この端末では開けません。資金は安全です — リカバリーフレーズから復元してください。';

  @override
  String get walletOnboardingFailedRestoreAction => 'リカバリーフレーズから復元';

  @override
  String get walletOnboardingRecoverConfirmTitle => 'このウォレットを復元しますか?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      '続ける前に、リカバリーフレーズを手元にご用意ください — 次の画面で資金を復元するために必要です。資金はブロックチェーン上で安全に保たれており、そのフレーズによって管理されています。この操作により、この端末上の読み取れなくなったウォレットデータが削除され、再構築できるようになります。';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'キャンセル';

  @override
  String get walletOnboardingFailedStorageFull =>
      'ウォレットをセットアップするための空き容量が不足しています。空き容量を確保して、もう一度お試しください。';

  @override
  String get walletOnboardingFailedNoVault =>
      'この端末にはセキュアキーストアがないため、ここではウォレットがリカバリーフレーズを保護できません。';

  @override
  String get walletOnboardingFailedNetwork =>
      'セットアップ中にネットワークへ接続できませんでした。接続を確認し、もう一度お試しください。';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'ウォレットのセットアップが完了しませんでした。もう一度お試しいただくと完了します — 何も失われていません。';

  @override
  String get walletOnboardingFailedUnknown =>
      'ウォレットのセットアップ中に問題が発生しました。もう一度お試しください。';

  @override
  String get walletOnboardingFailedConfiguration =>
      'このアプリのウォレット設定に誤りがあるため、ウォレットを起動できません。再試行しても解決しません。アプリの開発者にご報告ください。資金への影響はありません。';

  @override
  String get walletSendButton => '送金';

  @override
  String get walletSendSyncNotRunning => '同期は実行されていません — 使用可能な残高を更新できません';

  @override
  String get walletSendWaitingForFunds =>
      '同期はまだ進行中です — 使用可能な残高があると送金できるようになります';

  @override
  String get walletSendNoSpendableYet => '使用可能な残高がまだありません';

  @override
  String get walletSendSyncUnavailable => '同期が再開すると送金できるようになります';

  @override
  String get walletSendTitle => '送金';

  @override
  String get walletSendUnavailable => 'ウォレットの準備が現在できていません。戻って、もう一度お試しください。';

  @override
  String get walletSendWatchOnly =>
      'これはウォッチオンリーのウォレットです。残高の表示や支払いの受け取りはできますが、送金キーを持っていません — そのため送金できません。';

  @override
  String get walletSendExpiredTitle => 'この支払いリクエストは期限切れです';

  @override
  String get walletSendExpiredBody =>
      '送信画面を開くのに5秒以上かかったため、アプリには何も送信されなかったと伝えられました。この回答は確定です。このリクエストはここから支払えません。支払うには、アプリからやり直してください。';

  @override
  String get walletSendFaultWatchOnly =>
      'これはウォッチオンリーのウォレットです — 送金キーを持っていないため、送金できません。';

  @override
  String walletSendAvailable(String amount) {
    return '送金可能額: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return '送金可能額: $amount ZEC。残高はまだ追いつき中です。';
  }

  @override
  String get walletSendRecipientLabel => '送金先アドレス';

  @override
  String get walletSendRecipientHint => 'Zcashアドレス(u、z、tのいずれかで始まります)';

  @override
  String get walletSendRecipientLocked => '送金先はここでは変更できません';

  @override
  String get walletSendAmountLabel => '金額(ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'メモ(任意)';

  @override
  String get walletSendMemoHint => 'シールドされた(非公開の)送金先にのみ届きます';

  @override
  String get walletSendMemoTransparentDisabled =>
      'メモにはシールドされた送金先が必要です。この公開アドレスはメモを受け取れません。';

  @override
  String get walletSendMemoMachineDisabled =>
      'この支払いにはすでにアプリの参照情報が付いているため、書き込みメモは追加できません。';

  @override
  String get walletSendMachineMemoTitle => 'アプリが参照情報を添付します';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return '用途は次のとおりとされています: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      '取引とともに残り、あとから削除できません。ウォレットは内容を確認できません。';

  @override
  String get walletSendRecipientShielded => 'シールド・非公開';

  @override
  String get walletSendRecipientTransparent => '公開';

  @override
  String get walletSendRecipientInvalid => '有効なZcashアドレスではないようです。';

  @override
  String get walletSendRecipientWrongNetwork => 'このアドレスは別のZcashネットワーク用です。';

  @override
  String get walletSendReviewButton => '支払い内容を確認';

  @override
  String get walletSendQueueButton => '後で送信するため待機';

  @override
  String get walletSendQueueHint =>
      '送信待ちの支払いは「保存済み・保留中」で待機しており、そこから送信またはキャンセルできます。ネットワーク手数料は送信時に計算されます。';

  @override
  String get walletSendPreparing => '支払いを準備中…';

  @override
  String get walletSendSubmitting => '送信中…';

  @override
  String get walletSendQueuing => '待機登録中…';

  @override
  String get walletSendReviewTitle => '支払いを確認';

  @override
  String get walletSendTotalLabel => '合計';

  @override
  String get walletSendFeeLabel => 'ネットワーク手数料';

  @override
  String get walletSendChangeLabel => '返却されるおつり';

  @override
  String get walletSendDeshieldTitle => 'この支払いは非公開ではありません';

  @override
  String get walletSendDeshieldBody =>
      '公開アドレスへの送金のため、金額と送金先はZcashブロックチェーン上で公開され、誰でも閲覧できます。';

  @override
  String get walletSendPublicAckLabel => 'この支払いが公開されることを理解しました。';

  @override
  String get walletSendConfirmButton => '今すぐ送金';

  @override
  String get walletSendBackButton => '戻る';

  @override
  String get walletSendSelfSendNote =>
      'ご自身のウォレットへ送金しようとしています。ネットワーク手数料は通常どおり発生します。';

  @override
  String get walletSendLargeConfirmTitle => '高額な送金を行いますか?';

  @override
  String get walletSendLargeConfirmNearTotal => 'これはほぼ残高全額です。送金した支払いは取り消せません。';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'これは高額な支払いです。送金した支払いは取り消せません。';

  @override
  String get walletSendLargeConfirmBoth =>
      'これは高額な支払いです — ほぼ残高全額に相当します。送金した支払いは取り消せません。';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return '$amountを送金';
  }

  @override
  String get walletSendLargeConfirmCancel => '戻る';

  @override
  String get walletSendSentTitle => '支払いを送信しました';

  @override
  String get walletSendSentBody => '支払いはネットワークにブロードキャストされました。';

  @override
  String get walletSendSavedTitle => '保存しました — 送信を完了します';

  @override
  String get walletSendSavedBody =>
      '今回は支払いを送信できませんでしたが、保存されており、後の同期でウォレットが送信します。資金が失われることはありません。';

  @override
  String get walletSendKeptTitle => '保存済み';

  @override
  String get walletSendKeptBody =>
      'ウォレットはこのトランザクションを保持していますが、自動で送信することは約束していません。状況は「履歴」で確認してください。';

  @override
  String get walletSendPartialBody =>
      '支払いの一部は送信されましたが、残りは後の同期でウォレットが完了させます。資金が失われることはありません。';

  @override
  String get walletSendInMotionTitle => '支払い処理中';

  @override
  String get walletSendInMotionBody =>
      '支払いが開始され、ウォレットが管理する使い捨てアドレスを経由して移動中です。再送信しないでください。完了しない場合は、ウォレット画面から資金を回収できます。';

  @override
  String get walletSendAlreadyTitle => '送信済みです';

  @override
  String get walletSendAlreadyBody => 'この支払いはすでに送信されています — 二重に送信されることはありません。';

  @override
  String get walletSendFailedTitle => '支払いを完了できませんでした';

  @override
  String get walletSendFailedBody =>
      'この支払いの完了中に問題が発生し、何も送信されませんでした。もう一度お試しいただけます。';

  @override
  String get walletSendTryAgain => 'もう一度試す';

  @override
  String get walletSendDone => '完了';

  @override
  String get walletSendAnother => 'もう一度送金';

  @override
  String get walletSendQueuedTitle => '送信待ちに登録しました';

  @override
  String get walletSendQueuedBody =>
      'この支払いは保存されています。「保存済み・保留中」で確認でき、そこから今すぐ送金するかキャンセルできます。';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return '使用可能な残高が不足しています — 保有額は$available ZECで、この送金には$required ZECが必要です。';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Zcash ネットワークがアップグレードされたため、送金するにはこのアプリの更新が必要です。資金は安全です。';

  @override
  String get walletSyncUpToDateLimited => 'このバージョンが読み取れる範囲まで同期済み';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Zcash ネットワークがアップグレードされました。このバージョンは読み取れるものをすべてスキャンしましたが、新しいブロックにはまだ表示できない資金が含まれている可能性があり、最近の支払いのメモは利用できません。すべてを表示するにはアプリを更新してください。';

  @override
  String get walletSyncUpToDateDegraded => '同期済みですが、このサーバーはすべてのプールを提供していません';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'このサーバーは、Zcashのシールドプールのひとつを拒否、保留、または誤って報告しています。そのプールで受け取った資金はこのサーバー経由では使えず、表示されている残高は下限です。使用するには別のサーバーに切り替えてください。接続の問題ではありません。';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: このサーバーは提供を拒否しています';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: このサーバーは一部を提供していません';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: このサーバーは誤った情報を報告しています';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: このサーバーの提供状況は不明です';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind => 'このサーバーとは同期済みですが、サーバーがネットワークより遅れています';

  @override
  String get walletSyncExplainEndpointBehind =>
      'このサーバーのチェーンは、このバージョンのアプリがビルドされる前にネットワークがすでに通過したブロックで止まっています。そのため残高はそのブロック時点までしか反映されません。新しい受け取りがまだ表示されないことや、ここから送った支払いが届かないことがあります。別のサーバーに切り替えて最新の状態にしてください。接続の問題ではありません。';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'アプリの更新を待っています — 資金は安全で、まだ何も送金されていません。';

  @override
  String get walletParkedBlockedByServerSilent =>
      'ネットワークのバージョンを報告するサーバーを待っています — サーバーを切り替えてください。資金は安全で、まだ何も送金されていません。';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'ネットワークのバージョンを報告するサーバーを待っています。この端末の日付と時刻が間違っている場合は、まずそれを修正してから、サーバーを切り替えてください。資金は安全で、まだ何も送金されていません。';

  @override
  String get walletSyncUnverified => '同期済みですが、このサーバーはネットワークのバージョンを報告していません';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other: '送金はあと約$hours時間は可能です — その後はサーバーを切り替えてください。',
      zero: '送金はあと1時間未満は可能です — その後はサーバーを切り替えてください。',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return '送金はあと約$blocksブロックの間は可能です — その後はサーバーを切り替えてください。';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'このサーバーは$blocksブロックの間ネットワークのバージョンを報告していないため、このアプリは送金が安全かどうかを確認できません。別のサーバーに切り替えてください。';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'このサーバーは1日間ネットワークのバージョンを報告していないため、このアプリは送金が安全かどうかを確認できません。この端末の日付と時刻が間違っている場合は、まずそれを修正してから、ネットワークのバージョンを報告するサーバーに切り替えてください。';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'このサーバーはネットワークのバージョンを一度も報告していないため、このアプリは送金が安全かどうかを確認できません。別のサーバーに切り替えてください。';

  @override
  String get walletSyncExplainUnverified =>
      'このサーバーはZcashネットワークのどのバージョンにあるかを伝えないため、このアプリは署名した支払いが受け入れられることを確認できません。 残高は最新です。 別のサーバーに切り替えてください — これは接続の問題ではありません。';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'このサーバーはZcashネットワークのどのバージョンにあるかを伝えないため、このアプリは署名した支払いが受け入れられることを確認できません。 さらに、このウォレットが後で取り消さざるを得なかったブロックを繰り返し提供しているため、残高は最新でない可能性があります。 別のサーバーに切り替えてください — これは接続の問題ではありません。';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'このサーバーは、このウォレットが後で取り消さざるを得なくなるブロックを繰り返し提供しています — サーバーを切り替えてください。';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      '残高はまだ追いつき中です。ウォレットの同期が進むにつれて、さらに利用可能になる場合があります。';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZECは届いている途中で、ウォレットの同期が追いつき次第使用可能になります。';
  }

  @override
  String get walletSendFaultAmountEmpty => '送金する金額を入力してください。';

  @override
  String get walletSendFaultAmountNotANumber => '金額は数値で入力してください。例: 0.25';

  @override
  String get walletSendFaultAmountDecimals => 'ZECの小数点以下は最大8桁までです。';

  @override
  String get walletSendFaultAmountNotPositive => '0より大きい金額を入力してください。';

  @override
  String get walletSendFaultAmountOutOfRange => 'その金額はZECの総発行量を超えています。';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'このアプリでは現在、送金額は$limit ZECまでに制限されています。';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'このネットワークで有効なZcashアドレスではないようです。確認のうえ、もう一度お試しください。';

  @override
  String get walletSendFaultMemoToTransparent =>
      'この送金先はメモを受け取れません。メモを削除するか、シールドされた(非公開の)アドレスへ送金してください。';

  @override
  String get walletSendFaultMemoTooLong => 'メモが長すぎます。短くして、もう一度お試しください。';

  @override
  String get walletSendFaultMemoNotSendable => 'そのメモは送信できません。削除して、もう一度お試しください。';

  @override
  String get walletSendFaultMemoConflict =>
      'この支払いを送信できませんでした。アプリが2つのメモを添付しました。何も送信されていません。';

  @override
  String get walletSendFaultNetworkMismatch => 'そのアドレスは別のネットワーク用です。';

  @override
  String get walletSendFaultUriInvalid => 'この支払いを作成できませんでした。アドレスと金額を確認してください。';

  @override
  String get walletSendFaultNotSynced =>
      'ウォレットの同期がまだ十分に進んでいません。同期が追いつくのを待つか、後で送信するために待機に登録してください。';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'ウォレットの同期がまだ十分に進んでいません。同期が追いつくのを待ってください。';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'ウォレットの同期がまだ十分に進んでいません。同期は現在実行されていません。ウォレット画面で同期の状態を確認してください。';

  @override
  String get walletSendFaultAmountsExpired =>
      '確認中に金額の有効期限が切れました。もう一度支払い内容をご確認ください。';

  @override
  String get walletSendFaultQueueFull =>
      '送信待ちの支払いが多すぎます。それらの送信が完了してから、もう一度お試しください。';

  @override
  String get walletSendFaultWalletBusy => 'ウォレットは現在処理中です。しばらくしてからもう一度お試しください。';

  @override
  String get walletSendFaultStorageFull =>
      'この送信を完了するための空き容量が不足しています。空き容量を確保して、もう一度お試しください。';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      '現在使用中の使い捨てアドレスが多すぎます。送金が承認されるにつれていくつか解放される可能性がありますが、自然に解消するとは限りません。資金は安全です。';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'この支払いを準備できませんでした。内容を確認し、もう一度お試しください。';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'この支払いを今すぐ準備できませんでした。しばらくしてからもう一度お試しください。';

  @override
  String get walletSwapButton => 'スワップ';

  @override
  String get walletSwapTitle => 'ZECをスワップ';

  @override
  String get walletSwapUnavailableWallet =>
      'ウォレットの準備が現在できていません。戻って、もう一度お試しください。';

  @override
  String get walletSwapUnavailableOff => '現在スワップは利用できません。';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'これはウォッチオンリーのウォレットです — スワップできません。';

  @override
  String get walletSwapDone => '完了';

  @override
  String get walletSwapBackToWallet => 'ウォレットに戻る';

  @override
  String walletSwapAvailable(String amount) {
    return 'スワップ可能額: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'スワップ可能額: $amount ZEC。残高はまだ追いつき中です。';
  }

  @override
  String get walletSwapAssetLabel => '受け取る資産';

  @override
  String get walletSwapAmountLabel => 'スワップする金額(ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => '送金先アドレス';

  @override
  String get walletSwapDestinationHint => '受け取り先チェーン上のご自身のアドレス';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'ご自身の$chain受け取りアドレス';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return '$chainのアドレスです — スワップ後の資産の送付先です。チェーンが正しいか必ずご確認ください。';
  }

  @override
  String get walletSwapDestinationScanTooltip => '送金先アドレスのQRコードをスキャン';

  @override
  String get walletSwapTargetAssetHint => '受け取る資産を選択';

  @override
  String get walletSwapQuoteButton => '見積もりを取得';

  @override
  String get walletSwapQuoting => '見積もりを取得中…';

  @override
  String get walletSwapExecuting => 'スワップを開始中…';

  @override
  String get walletSwapExecuteStillWorking =>
      'まだ処理中です — スワップを開始しています。最大で1分ほどかかることがあります。';

  @override
  String get walletSwapReviewTitle => 'スワップを確認';

  @override
  String get walletSwapYouSendLabel => '送る金額';

  @override
  String get walletSwapYouReceiveLabel => '受け取る最低金額';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'ネットワーク手数料';

  @override
  String get walletSwapNetworkFeeValue => '入金の送信時に加算されます';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return '見積もりの有効期限まで残り約$timeです — 期限が切れる前にご確認ください。';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      '見積もりの有効期限まで残り1分未満です — 期限が切れる前にご確認ください。';

  @override
  String get walletSwapQuoteExpired =>
      'この見積もりは期限切れです。戻って新しい見積もりを取得してください — このレートはもう保証されておらず、このまま送信すると返金になる可能性があります。';

  @override
  String get walletCountdownUnderMinute => '1分未満';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes分';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds秒';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '$hours時間$minutes分';
  }

  @override
  String get walletSwapDeshieldTitle => 'このスワップは非公開ではありません';

  @override
  String get walletSwapDeshieldBody =>
      'ZECをスワップして出す際、ZECはシールド解除されます — 入金は公開された取引となり、プロバイダー側もそのネットワーク上で公開されます。';

  @override
  String get walletSwapDiscloseTitle => 'スワッププロバイダーに見える情報';

  @override
  String get walletSwapDiscloseAmounts => '両側の金額';

  @override
  String get walletSwapDiscloseCrossLink => 'このZECと受け取る資産が同一のスワップであること';

  @override
  String get walletSwapDiscloseDestination => '送金先アドレス';

  @override
  String get walletSwapDiscloseSource => '送金元アドレス';

  @override
  String get walletSwapDiscloseIp => 'IPアドレス(Tor経由の場合を除く)';

  @override
  String get walletSwapDiscloseGeneric => 'このスワップのその他の詳細';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'プロバイダー自身の取引は、そのネットワーク上で公開されます';

  @override
  String get walletSwapAckLabel => '上記の情報がプロバイダーに見えることを理解しました。';

  @override
  String get walletSwapConfirmButton => 'スワップを開始';

  @override
  String get walletSwapBackButton => '戻る';

  @override
  String get walletSwapStatusPendingTitle => 'スワップを開始しました';

  @override
  String get walletSwapStatusCheckingTitle => 'スワップの状態を確認中…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'ウォレットがZECの入金をプロバイダーへ送信しています。一時的にオフラインの場合は、オンラインに戻り次第自動的に送信されますが、送信可能な時間は短く、それより先に終了すると、スワップはそのまま終了し、何も交換されません。ZECはそのままお客様のものですが、再び使用可能と表示されるまで最大1時間かかることがあります。';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      '入金の到着をお待ちしています。別のウォレットからまだ資金を送金していない場合は、見積もりの有効期限が切れる前に送金してください。';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'このスワップはまだ入金を待っています。入金手順はこの端末ではもう確認できません — すでに送金済みの場合は検出されます。まだ送金していない場合は、このスワップを期限切れにして新しいスワップを開始してください。';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return '入金期限は$timeまでです。';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      '入金期限を過ぎました。期限内に入金が送信されなかった場合、スワップは終了し、ZECはウォレット内に残ります。';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      '入金期限を過ぎました。まだ入金を送信していない場合、このスワップはそのまま終了します — 準備ができたら新しい見積もりを取得できます。';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'スワップ進行中',
      one: 'スワップ進行中',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec => 'ZECはスワッププロバイダーへ送信中です。';

  @override
  String get walletSwapInFlightRowIntoZec => '入金がスワッププロバイダーに届くのをお待ちしています。';

  @override
  String get walletSwapInFlightRowGeneric => 'スワップが進行中です。';

  @override
  String get walletSwapInFlightRowPastWindow =>
      '入金期限を過ぎました — このスワップのステータスをご確認ください。';

  @override
  String get walletSwapInFlightRowOverdue =>
      'このスワップはここではまだ確定した結果に至っていません — 開いて確認してください。このウォレットに戻ってくるZECは、同期後に残高に反映されます。';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'このスワップはここではまだ確定した結果に至っていません — 開いて確認してください。このスワップがこのウォレットに届けるZECは、同期後に残高に反映されます。';

  @override
  String get walletSwapRowOutcomeSuccess => 'スワップが完了しました。';

  @override
  String get walletSwapRowOutcomeRefunded => 'スワップが返金されました。';

  @override
  String get walletSwapRowOutcomeFailed => 'スワップが完了しませんでした。';

  @override
  String get walletSwapRemove => '削除';

  @override
  String get walletSwapRemoveTitle => 'このスワップをリストから削除しますか?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'これはこのリストからスワップを削除するだけで、スワップ自体はキャンセルされません。また、このウォレットはその返金の追跡を停止します。後で返金されたZECは引き続きこのウォレットに属します。完全な再スキャンでそれを見つけられます。';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'これはこのリストからスワップを削除するだけで、スワップ自体はキャンセルされません。また、このウォレットは入金されるZECの追跡を停止します。後で届いたZECは引き続きこのウォレットに属します。完全な再スキャンでそれを見つけられます。代わりにスワップが返金された場合、返金は送信した資産でこのウォレットの外に返されます。';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'これはこのリストからスワップを削除するだけで、スワップ自体はキャンセルされません。また、このウォレットはそこからまだ届いているZECの追跡を停止します。後で届くZECは引き続きこのウォレットに属します。完全な再スキャンでそれを見つけられます。';

  @override
  String get walletSwapRemoveBodyDone => '完了したスワップをリストから削除します。';

  @override
  String get walletSwapRemoveCancel => 'キャンセル';

  @override
  String get walletSwapRemoveConfirm => '削除';

  @override
  String walletSwapInFlightStarted(String time) {
    return '$timeに開始';
  }

  @override
  String get walletSwapViewSwap => 'スワップを表示';

  @override
  String get walletSwapsInFlightError => '現在、進行中のスワップを読み込めませんでした。';

  @override
  String get walletSwapsInFlightRetry => 'もう一度試す';

  @override
  String get walletSwapsInFlightRetryInProgress => '試行中…';

  @override
  String get walletSwapStartAnother => '別のスワップを開始';

  @override
  String get walletSwapStatusUnderTitle => '入金の完了を待っています';

  @override
  String get walletSwapStatusUnderBody => '入金の一部が到着しました。残りは処理中か、プロバイダーが返金します。';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      '入金の一部が到着しました。期限までに不足分を送金してください。送金されない場合、プロバイダーは到着済みの分を返金します。';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return '受領済み: $received。不足額: $missing。入金期限: $time。';
  }

  @override
  String get walletSwapStatusDetectedTitle => '入金を受領しました';

  @override
  String get walletSwapStatusDetectedBody => 'プロバイダーが入金を受領し、スワップを処理します。';

  @override
  String get walletSwapStatusProcessingTitle => 'スワップを処理中';

  @override
  String get walletSwapStatusProcessingBody => 'プロバイダーがスワップを完了しています。';

  @override
  String get walletSwapStatusSuccessTitle => 'スワップが完了しました';

  @override
  String get walletSwapStatusSuccessBody => 'スワップは正常に完了しました。';

  @override
  String get walletSwapStatusRefundedTitle => 'スワップが返金されました';

  @override
  String get walletSwapStatusRefundedBody =>
      'スワップが完了しなかったため、プロバイダーは資金を返金先アドレスに送り返しました。';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'スワップが完了しなかったため、プロバイダーはZECをこのウォレットへ返金しました。非シールド資金として届き、次回のウォレット同期後に残高へ反映されます―少し時間がかかることがあります。';

  @override
  String get walletSwapStatusFailedTitle => 'スワップが失敗しました';

  @override
  String get walletSwapStatusFailedBody =>
      'スワップを完了できませんでした。入金済みの資金は、プロバイダー側で決済または返金されます。';

  @override
  String get walletSwapStatusNotFoundTitle => 'スワップが見つかりません';

  @override
  String get walletSwapStatusNotFoundBody =>
      'プロバイダーにこのスワップの記録がありません — おそらく期限切れになったと考えられます。入金が行われていた場合、プロバイダーは返金先アドレスへ返金するはずです。このスワップはリストに残り、このウォレットはそのZECがそれでも届く場合に備えて追跡を続けます。いつでもリストから削除できます。';

  @override
  String get walletSwapStatusUnknownTitle => 'ステータスを取得できません';

  @override
  String get walletSwapStatusUnknownBody => '現在このスワップのステータスを確認できません。';

  @override
  String get walletSwapTrackingUnavailableTitle => '追跡できません';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'スワップ機能がオフのため、ここでは追跡できません。資金は、プロバイダー側で決済または返金されます。';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'ここではスワップ機能がオフになっているため、このスワップは現在追跡できません。返金された場合、ZECはこのウォレットに戻ります — スワップ機能を再度オンにしてウォレットが同期すると、残高に反映されます。';

  @override
  String get walletSwapTrackingError => 'このスワップを追跡できませんでした。';

  @override
  String get walletSwapTrackingErrorBody =>
      'このスワップの追跡を開始できませんでした。スワップ自体はそのまま進行している可能性があります — 入金済みの資金は、プロバイダー側で決済または返金されます。';

  @override
  String get walletSwapFaultDestinationRequired =>
      'スワップ後の資産を受け取るアドレスを入力してください。';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'この送金先アドレスは、この資産では無効です。確認のうえ、もう一度お試しください。';

  @override
  String get walletSwapFaultExpired => 'この見積もりは期限切れです。続けるには新しい見積もりを取得してください。';

  @override
  String get walletSwapFaultOutOfBounds =>
      'プロバイダーの価格が指定した上限を超えて変動したため、資金が動く前にスワップは中止されました。もう一度お試しください。';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'スリッページの許容値が高すぎるため、安全にスワップできません。もう一度お試しください。';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'スワッププロバイダーは現在利用できません。しばらくしてからもう一度お試しください。';

  @override
  String get walletSwapFaultConnection =>
      'スワップサービスに接続できませんでした。インターネット接続を確認し、もう一度お試しください。';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'スワッププロバイダーから予期しない応答があったため、スワップは中止されました。もう一度お試しください。';

  @override
  String get walletSwapFaultSwapOff => '現在スワップ機能はオフになっています。';

  @override
  String get walletSwapFaultDepositFailed =>
      '入金を送信できなかったため、ウォレットから資金は出ていません。新しい見積もりを取得してもう一度お試しください。';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'スワップはすでに進行中です。完全に決済されるか見積もりの有効期限が切れた後、新しいスワップを開始できます。しばらく時間がかかる場合があります。';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'このウォレットではまだ返金アドレスを設定できません — 通常、これは最初の同期がまだ完了していないことを意味します。同期が完了するのを待ってから、もう一度お試しください。';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'このウォレットではまだこのスワップの受取アドレスを設定できません — 通常、これは最初の同期がまだ完了していないことを意味します。同期が完了するのを待ってから、もう一度お試しください。';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'スワップを時間内に開始できませんでした — 接続が遅かったか、ウォレットが混み合っていた可能性があります。新しい見積もりを取得してもう一度お試しください。';

  @override
  String get walletSwapFaultStoreBusyRetry => 'ウォレットが一時的に混み合っています。もう一度お試しください。';

  @override
  String get walletSwapFaultTermsDiffer =>
      'この見積もりはウォレットが発行したものと一致しないため、何も送信されませんでした。新しい見積もりを取得してもう一度お試しください。';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'このスワップにはネットワーク手数料を含めて約$needed ZECが必要ですが、現在使用可能なのは$spendable ZECのみです。';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'このアプリでは現在、スワップ額は$limit ZECまでに制限されています。';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'このスワップにはネットワーク手数料を含めて約$needed ZECが必要ですが、現在使用可能なのは$spendable ZECのみです。残高はまだ追いつき中です — まもなくさらに使用可能になる場合があります。';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'ウォレットがこのスワップを安全に記録できなかったため、資金は移動していません。もう一度お試しください。';

  @override
  String get walletSwapFaultRequestInvalid =>
      'そのスワップリクエストは処理できませんでした。新しい見積もりを取得し、もう一度お試しください。';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'スワップの見積もりを取得できませんでした。内容を確認し、もう一度お試しください。';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'ウォレットの準備が現在できていません。戻って、もう一度お試しください。';

  @override
  String get walletSwapDirectionBuy => 'ZECを購入';

  @override
  String get walletSwapDirectionSell => 'ZECを売却';

  @override
  String get walletSwapRefundLabel => '返金先アドレス';

  @override
  String get walletSwapRefundHint => 'スワップが失敗した場合にコインが返却される場所';

  @override
  String get walletSwapRefundHelper => '送金元のチェーン上のアドレスです — Zcashアドレスではありません。';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'ご自身の$chain返金先アドレス';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return '$chainのアドレスです — スワップが失敗した場合にコインが返却される場所です。Zcashアドレスではありません。';
  }

  @override
  String get walletSwapRefundInfoTitle => '返金先アドレスについて';

  @override
  String get walletSwapRefundInfoBody =>
      'スワップが完了できない場合、プロバイダーは支払い元のチェーン上でこのアドレスへコインを送り返します。ご自身が管理するアドレスを入力してください — このウォレットは他チェーンのアドレスを検証できないため、ご自身で慎重にご確認ください。';

  @override
  String get walletSwapRefundScanTooltip => '返金先アドレスのQRコードをスキャン';

  @override
  String get walletSwapScanTitle => 'アドレスをスキャン';

  @override
  String get walletSwapScanInstruction => 'アドレスのQRコードにカメラを向けてください。';

  @override
  String get walletSwapScanManualEntry => '手動で入力';

  @override
  String get walletSwapScanCancel => 'キャンセル';

  @override
  String get walletSwapScanCameraUnavailable =>
      'カメラを使用できません。下のフィールドにアドレスを手動で入力してください。';

  @override
  String get walletSwapSourceAssetLabel => 'スワップ元の資産';

  @override
  String get walletSwapSourceAssetHint => '資産を選択';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return '送る金額($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => '送る金額';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$chain上の$symbol';
  }

  @override
  String get walletSwapPickerTitle => 'スワップ元の資産を選択';

  @override
  String get walletSwapPickerTitleReceive => '受け取る資産を選択';

  @override
  String get walletSwapPickerStale => '資産リストを更新できませんでした — 最後に取得したリストを表示しています。';

  @override
  String get walletSwapPickerEmpty => '現在スワップ可能な資産がありません。後でもう一度お試しください。';

  @override
  String get walletSwapPickerSearchHint => '名前またはチェーンで検索';

  @override
  String walletSwapPickerNoMatch(String query) {
    return '「$query」に一致する資産がありません。';
  }

  @override
  String get walletSwapPickerError => '資産リストを読み込めませんでした。接続を確認し、もう一度お試しください。';

  @override
  String get walletSwapPickerRetry => 'もう一度試す';

  @override
  String get walletSwapSlippageLabel => 'スリッページ許容値';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'カスタム';

  @override
  String get walletSwapSlippageCustomLabel => 'カスタムスリッページ';

  @override
  String get walletSwapSlippageMayFail =>
      '非常に低い値です — 価格が変動するとスワップが失敗する可能性があります。';

  @override
  String get walletSwapSlippageNormal => '安全な許容値です。';

  @override
  String get walletSwapSlippageRisky => '高い値です — 見積もりより大幅に少なく受け取る可能性があります。';

  @override
  String get walletSwapSlippageTooHigh =>
      '高すぎます — このスワップは拒否されます。10%以下に下げてください。';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return '少なくとも$zec ZECを受け取ります — これは$slippage%のスリッページ許容値による下限です。最終的な金額がこれを下回ることはありません。';
  }

  @override
  String get walletSwapIntoZecShieldTitle => 'ご自身のアドレスにZECを受け取ります';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'シールドするまでの間 — ワンタップで行え、到着時に案内が表示されます — 受け取った金額は一時的に公開状態となり、チェーン上で閲覧可能です。少額の場合、一定額が貯まるまで公開のままとなることがあります。';

  @override
  String get walletSwapRefundVerifyTitle => '返金先アドレスを確認';

  @override
  String get walletSwapRefundVerifyBody =>
      '1文字ずつ確認してください — スワップが失敗した場合、コインはここへ返却されます。このウォレットは他チェーンのアドレスを検証できません。';

  @override
  String get walletSwapRefundVerifyAck => '返金先アドレスが正しいことを確認しました。';

  @override
  String get walletSwapPayoutVerifyTitle => '受取先アドレスを確認';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return '1文字ずつ確認してください — $assetはこのアドレスで受け取ります。このウォレットは他チェーンのアドレスを検証できません。';
  }

  @override
  String get walletSwapPayoutVerifyAck => '受取先アドレスが正しいことを確認しました。';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'ここではスワップ機能がオフになっています。すでに送信中のZECは、次回の同期後にウォレットに表示されます。';

  @override
  String get walletSwapFaultForeignAmountRequired => 'スワップする金額を入力してください。';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      '送金元チェーン上の返金先アドレスを入力してください。';

  @override
  String get walletSwapDepositTitle => '支払いを送信';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return '$chain上で、下記のアドレスに正確に$amount $assetを送金してください。';
  }

  @override
  String get walletSwapDepositExactNote =>
      '正確な金額を送金してください。金額が不足していたり、期限後に送金したりした場合、プロバイダーは返金先アドレスへ返金します。';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return '入金期限: 残り$time';
  }

  @override
  String get walletSwapDepositExpired =>
      'この入金期限は終了しました。今は送金しないでください — 新しいスワップを開始してください。すでに送金済みの場合、プロバイダーから返金先アドレスへ返金されるはずです。';

  @override
  String get walletSwapDepositQrLabel => '入金アドレスのQRコード';

  @override
  String get walletSwapDepositAddressLabel => '入金アドレス';

  @override
  String get walletSwapDepositCopy => '入金アドレスをコピー';

  @override
  String get walletSwapDepositCopied => '入金アドレスをコピーしました';

  @override
  String get walletSwapDepositMemoRequired => 'この入金にはメモ/タグが必要です';

  @override
  String get walletSwapDepositMemoWarning =>
      '入金時には必ずこのメモを正確に含めてください。含めなかった場合や誤ったメモを使用した場合、資金が永久に失われる可能性があります。';

  @override
  String get walletSwapDepositMemoLabel => '入金メモ/タグ';

  @override
  String get walletSwapDepositMemoCopy => 'メモをコピー';

  @override
  String get walletSwapDepositMemoCopied => 'メモをコピーしました';

  @override
  String get walletSwapDepositSent => '送金しました';

  @override
  String get walletSwapDepositBackTitle => 'この画面を離れますか?';

  @override
  String get walletSwapDepositBackBody =>
      'この操作でスワップがキャンセルされることはありません — バックグラウンドで継続します。ただし支払いには入金アドレスが必要なため、まだの場合は先にコピーしてください。';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'この操作でスワップがキャンセルされることはありません — バックグラウンドで継続します。入金期限が終了しました。今は入金アドレスに送金しないでください。すでに送金済みの場合、プロバイダーから返金先アドレスへ返金されるはずです。';

  @override
  String get walletSwapDepositBackStay => 'とどまる';

  @override
  String get walletSwapDepositBackLeave => '離れる';

  @override
  String get walletReceive => '受取';

  @override
  String get walletReceiveSubtitle => 'このアドレスを共有してZECを受け取ります。公開して共有しても安全です。';

  @override
  String get walletReceiveCopy => 'アドレスをコピー';

  @override
  String get walletReceiveCopied => 'アドレスをコピーしました';

  @override
  String get walletReceiveUnavailable => 'ウォレットの準備がまだできていません。';

  @override
  String get walletReceiveError => 'アドレスを読み込めませんでした。もう一度お試しください。';

  @override
  String get walletReceivePreparing => 'アドレスを準備中…';

  @override
  String get walletReceivePreparingHint =>
      'ウォレットはこのアドレスをこの端末上で準備します — ウォレットが他の処理で混み合っている場合、少し時間がかかることがあります。';

  @override
  String get walletReceiveRetry => 'もう一度試す';

  @override
  String get walletReceiveQrLabel => '受取アドレスのQRコード';

  @override
  String get walletReceiveTypeShielded => 'シールド';

  @override
  String get walletReceiveTypeTransparent => '公開';

  @override
  String get walletReceiveSubtitleTransparent =>
      'シールドアドレスへ支払えない送金者からZECを受け取る場合は、この公開アドレスを共有してください。';

  @override
  String get walletReceiveTransparentWarning =>
      'これは公開アドレスです。チェーン上で公開されており、再利用すると支払いが関連付けられてしまいます。できる限りシールドアドレスをご利用ください。受け取った資金は、受取後にシールドしてください。';

  @override
  String get walletReceiveQrLabelTransparent => '公開の受取アドレスのQRコード';

  @override
  String get walletReceiveFreshAddress => '新しいアドレスを使う';

  @override
  String get walletReceiveFreshCaption =>
      '新しいアドレス — 他のアドレスとひも付けられません。ここへの支払いも、このウォレットに届き、これまでのアドレスもそのまま使えます。このアドレスはここでは二度と表示されません — 今すぐコピーしてください。';

  @override
  String get walletReceiveFreshError => '新しいアドレスを作成できませんでした。もう一度お試しください。';

  @override
  String get walletReceiveFreshBusy =>
      'ウォレットが混み合っています。しばらくしてから、新しいアドレスをもう一度お試しください。';

  @override
  String get walletReceiveShare => '共有';

  @override
  String get walletReceiveRequestAmount => '金額をリクエスト';

  @override
  String get walletReceiveRequestAmountLabel => '金額（任意）';

  @override
  String get walletReceiveFreshCopyNow =>
      'このアドレスはここでは二度と表示されません — 今すぐコピーしてください。';

  @override
  String get walletSecurityMenuItem => 'セキュリティ…';

  @override
  String get securityTitle => 'セキュリティ';

  @override
  String get securityUnavailableBody =>
      'ウォレットのセキュリティ設定は、ウォレットそのものではなく、このアプリによって管理されています。';

  @override
  String get securityCustodySectionTitle => '鍵の保管';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave(ハードウェア)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox(ハードウェア)';

  @override
  String get securityCustodyTierTee => 'ハードウェアキーストア(TEE)';

  @override
  String get securityCustodyTierSoftware => 'ソフトウェアキーストア';

  @override
  String get securityCustodyTierKeychain => 'キーチェーン(ソフトウェア暗号化)';

  @override
  String get securityCustodyTierNone => 'ハードウェアキーストアなし';

  @override
  String get securityCustodyTierUnknown => '不明';

  @override
  String get securityCustodyHardwareKey =>
      'このウォレットをロックする鍵はこの端末のセキュアなハードウェアに保管され、ウォレットと一緒に削除されます。';

  @override
  String get securityCustodyBestEffort =>
      '削除は鍵をベストエフォートで消去しますが、端末がストレージを再利用するまでの間、短時間フォレンジック的に復元できる余地が残る場合があります。確実に消去するには、端末の「すべてのコンテンツを消去」機能も併用してください。';

  @override
  String get securityCustodyProbeError => '保管ステータスを読み込めませんでした。戻ってもう一度お試しください。';

  @override
  String get securityDeleteWalletButton => 'ウォレットを削除';

  @override
  String get securityDeleteWalletSubtitle =>
      'この端末からウォレットとその鍵を削除します。資金はチェーン上に残り、リカバリーフレーズから復元できます。';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'この端末からウォレットとその鍵を削除します。送金キーは保持していないため、バックアップするものはなく — ビューイングキーでいつでも再追加できます。';

  @override
  String get securityDeleteDialogTitle => 'このウォレットを削除しますか?';

  @override
  String get securityDeleteDialogBody =>
      'この操作により、この端末からウォレットとその鍵が削除されます。リカバリーフレーズを必ずバックアップしてください — それが資金を復元する唯一の手段です。';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'この操作により、この端末からウォレットとその鍵が削除されます。送金キーは保持していないため、バックアップの必要はなく — ビューイングキーがあれば後で再追加できます。';

  @override
  String get securityDeleteDialogConfirm => '削除';

  @override
  String get securityDeleteDialogCancel => 'キャンセル';

  @override
  String get securityDeleteFailedSnack =>
      'ウォレットを削除できませんでした — ウォレットに変更はありません。もう一度お試しください。';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return '先にサーバーの切り替えを完了してください — 切り替えは$seconds秒以内に完了または停止します。その後、もう一度ウォレットの削除をお試しください。';
  }

  @override
  String get walletParkedTitle => '保存済み・保留中';

  @override
  String get walletParkedSubtitle => 'これらの支払いはまだ送信されていません。金額は引き続き残高に含まれています。';

  @override
  String get walletParkedSubtitlePreparing =>
      'これらの支払いはまだ送信されていません。金額は引き続き残高に含まれています — ただし、ウォレットが送信中の支払いは、その金額がすでに充当されている可能性があります。';

  @override
  String get walletParkedCancel => 'キャンセル';

  @override
  String get walletParkedPausedHint =>
      '一時停止中 — この支払いは自動的には送信されません。資金は安全です。今すぐ送金するか、キャンセルしてください。';

  @override
  String get walletParkedRetryStale => 'この支払いはもう保留されていません。保留中の支払いと履歴をご確認ください。';

  @override
  String get walletParkedAlreadyInProgress =>
      'この支払いはもう待機していません — ウォレットがすでに送信している可能性があります。「保存済み・保留中」と履歴をご確認ください。';

  @override
  String get walletReclaimExplainer =>
      '使い捨てアドレスでの送信が詰まっています。再開できます — ご自身のアドレス間で少額を移動し、それが戻ってきます。';

  @override
  String get walletReclaimButton => '送信を再開';

  @override
  String get walletReclaimInProgress => '再開中…';

  @override
  String get walletReclaimConfirmTitle => '使い捨てアドレスでの送信を再開しますか?';

  @override
  String get walletReclaimConfirmBody =>
      'ご自身のアドレス間で少額を移動して使い捨てアドレスでの送信を解放し、その金額は戻ってきます。数回分のネットワーク手数料がかかります。確定したら、「今すぐ回収」で移動した金額を回収してください。';

  @override
  String get walletReclaimConfirmCancel => '今はしない';

  @override
  String get walletReclaimConfirmAction => '再開';

  @override
  String get walletReclaimStarted =>
      '再開を開始しました。確定したら、一時停止中の支払いを送信し、その後「今すぐ回収」で移動した金額を回収してください。';

  @override
  String get walletReclaimNothing => '現在再開できるものはありません。';

  @override
  String get walletReclaimNotBroadcast =>
      'ネットワークに届いたかを確認できませんでした。移動は完了している可能性があります。しばらくしてからもう一度お試しください。';

  @override
  String get walletReclaimNeedsFunds => '送信を再開するにはシールドされたZECが必要です。';

  @override
  String get walletReclaimFailed => '現在送信を再開できませんでした。資金に変更はありません。もう一度お試しください。';

  @override
  String get walletReclaimUnknown =>
      '再開が終了しました。使い捨てアドレスでの送信をご確認のうえ、「今すぐ回収」で移動した金額があれば回収してください。';

  @override
  String get walletParkedError => '現在、保留中の支払いを読み込めませんでした。';

  @override
  String get walletParkedErrorRetry => 'もう一度試す';

  @override
  String get walletParkedErrorRetryInProgress => '試行中…';

  @override
  String get walletParkedCancelConfirmTitle => 'この保留中の支払いをキャンセルしますか?';

  @override
  String get walletParkedCancelConfirmBody =>
      'この操作により保存済みの支払いは破棄されます。まだ送信されていないため、ウォレットから資金が出ることはありませんが、この操作は取り消せません。';

  @override
  String get walletParkedCancelConfirmKeep => '残す';

  @override
  String get walletParkedCancelConfirmDiscard => '支払いを破棄';

  @override
  String get walletParkedCancelDone => '保留中の支払いをキャンセルしました。';

  @override
  String get walletParkedCancelAlreadySending =>
      'この支払いはすでに送信済みか送信中の可能性があります — 履歴をご確認ください。';

  @override
  String get walletParkedCancelFailed =>
      '現在キャンセルできませんでした。支払いに変更はありません。もう一度お試しください。';

  @override
  String get walletRecoverNow => '今すぐ回収';

  @override
  String get walletRecoverConfirmTitle => 'シールド残高へ回収しますか?';

  @override
  String get walletRecoverConfirmBody =>
      '使い捨てアドレスを確認し、見つかった資金を非公開のシールド残高へ移動します。何度でも安全に再実行できます。';

  @override
  String get walletRecoverConfirmCancel => '今はしない';

  @override
  String get walletRecoverConfirmAction => '回収';

  @override
  String get walletRecoverInProgress => '回収中…';

  @override
  String walletRecoverDone(String amount) {
    return '$amountをシールド残高へ回収中です。';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return '$amountを回収中です — 一部の資金には再試行が必要です。';
  }

  @override
  String get walletRecoverRetry => '一部の資金には再試行が必要です — 回収をもう一度実行してください。';

  @override
  String get walletRecoverTruncated =>
      'まだすべての使い捨てアドレスを確認できていません — 残りを確認するにはもう一度実行してください。';

  @override
  String get walletRecoverNothing => '現在回収できるものはありません。';

  @override
  String get walletRecoverFailed => '現在回収できませんでした。資金に変更はありません。もう一度お試しください。';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount 保存済み・保留中・$time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return '$timeに保存された$amountの支払いをキャンセル';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount 一時停止中・$time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount 送信準備中・$time';
  }

  @override
  String get walletParkedPreparingHint =>
      'ウォレットがこの支払いを準備しています — その金額はすでに充当されている可能性があります。資金は安全です。完了しない場合は、自動的に一覧に戻ります。';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'ウォレットがこの支払いを準備しています — その金額はすでに充当されている可能性があります。資金は安全ですが、ウォレットが再び同期するまでは完了できません。';

  @override
  String get walletParkedSendNow => '今すぐ送金';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return '$timeに保存された$amountの支払いを送信中';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return '$timeに保存された$amountの支払いを今すぐ送金';
  }

  @override
  String get walletParkedSendNowInProgress => '送信中…';

  @override
  String get walletParkedAuthorizeSent => '支払いを送金しています。';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      '支払いを送金しています。届かない場合、ウォレットが再び同期して初めて完了できます。';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'まだ送金する準備ができていません。支払いは保存されたままで、変更はありません。';

  @override
  String get walletParkedAuthorizeRearmed =>
      'まだ送金する準備ができていません。支払いは保存されており、一時停止は解除されました — 後でもう一度「今すぐ送金」をお試しいただくか、キャンセルしてください。';

  @override
  String get walletParkedAuthorizeFailed =>
      '現在送金できませんでした。支払いに変更はありません。もう一度お試しください。';

  @override
  String get walletTransparentFundsMenuItem => '公開の資金…';

  @override
  String get walletTransparentFundsTitle => '公開の資金';

  @override
  String get walletTransparentFundsIntro =>
      '公開の資金は、金額、アドレス、コインの履歴を含め、ブロックチェーン上で公開されています。';

  @override
  String get walletExpertToggleLabel => '詳細設定: 公開の資金';

  @override
  String get walletExpertToggleDescription =>
      '公開の資金を保有し、自動シールドをオフにするための詳細な操作を表示します。';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      '公開の資金を保有するための詳細な操作を表示します。';

  @override
  String get walletAutoShieldToggleLabel => '自動的にシールド';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return '公開の資金の残高が$minZec ZECに達すると、自動的にシールド残高へ移動されます。オフにすると、公開の資金はご自身でシールドするまで公開されたままになります。';
  }

  @override
  String get walletSettingsSaveFailed => '設定を保存できませんでした。もう一度お試しください。';

  @override
  String get walletAutoShieldIncomplete =>
      '自動シールドが完了しませんでした — この資金はまだ公開されています。今すぐシールドできます。';

  @override
  String get walletSendPrivacyShielded => 'シールドされた支払い — 金額と送金先はチェーン上で非公開のままです。';

  @override
  String get walletSendPrivacyTransparent =>
      '公開の支払い — 金額とアドレスはブロックチェーン上で公開されています。';

  @override
  String get walletActivityPublicBadge => 'ブロックチェーン上で公開';

  @override
  String get walletShieldWalletEnded =>
      'ウォレットのセッションが終了しました。閉じてから開き直し、もう一度お試しください。';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return '新しい公開の資金は、$minZec ZECに達すると自動的にシールドされて非公開の残高に組み込まれます。';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      '自動シールドはオフです — 公開の資金はご自身でシールドするまで公開されたままになります。';

  @override
  String get walletMoveAutoShieldNote =>
      '自動シールドがオンになっています。この資金の到着後、自動的に再度シールドされます(別途手数料がかかります)。公開のまま保持するには、まず「公開の資金」で自動シールドをオフにしてください。';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'この移動後、公開残高は $amount ZEC になります。再度シールドするのに必要な $floor ZEC に足りません。追加の資金が届くまで公開のままです。';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'ご自身の公開アドレスへ移動します。この移動の記録は公開台帳に永久に残ります。';

  @override
  String get walletTxDetailVisibility => '可視性';

  @override
  String get walletTransparentFundsAutoDenied =>
      '自動シールドは今回のセッションでは一時停止されています — 承認されなかったためです。手動でのシールドは引き続き行えます。';

  @override
  String get walletDeepScanMenuItem => 'より古いスワップアドレスを確認…';

  @override
  String get walletMenuSyncNotRunningHint => '同期は現在実行されていません。';

  @override
  String get walletDeepScanTitle => 'より古いスワップアドレスを確認';

  @override
  String get walletDeepScanBody =>
      'このウォレットを復元し、以前スワップを多く利用していた場合、最も古いスワップの資金は見つけるのに追加の手順が必要になることがあります。この確認はそれを探します — 見つかったものはウォレットの同期に伴い残高に反映されます。';

  @override
  String get walletDeepScanCoverage =>
      'より古いスワップアドレスはここまで確認済みです。古いスワップからの資金がまだ見つかっていない場合は、さらに深く確認してください。';

  @override
  String get walletDeepScanCoveragePending =>
      '現在の範囲を確認中です — 見つかったものは残高に反映されます。少し時間がかかることがあります。';

  @override
  String get walletDeepScanCoverageUnknown => 'ウォレットの最も古いスワップからの資金を確認します。';

  @override
  String get walletDeepScanCheckButton => 'より古いアドレスを確認';

  @override
  String get walletDeepScanCheckDeeperButton => 'さらに古いアドレスを確認';

  @override
  String get walletDeepScanChecking => '確認中…';

  @override
  String get walletDeepScanClose => '閉じる';

  @override
  String get walletDeepScanTorHint =>
      '現在Tor経由で接続されていません。プライバシーを高めるため、確認を行う前にTorが有効になるまで待つことをご検討ください。';

  @override
  String get walletDeepScanRescanBusy =>
      '再スキャンが終了すると、より古いスワップアドレスを確認できるようになります。';

  @override
  String get walletDeepScanRan => 'より古いスワップアドレスを確認しています — 見つかったものは残高に反映されます。';

  @override
  String get walletDeepScanFailed => '確認を開始できませんでした。変更はありません — もう一度お試しください。';

  @override
  String get walletDeepScanSlow =>
      '通常より時間がかかっています。より古いスワップアドレスが確認された場合、見つかったものは残高に反映されます — しばらくしてからもう一度ご確認ください。';

  @override
  String get walletDeepScanRefusedDisabled =>
      '現在スワップがオフになっているため、これを実行できません。スワップが利用可能になったら、もう一度お試しください。';

  @override
  String get walletDeepScanRefusedOutstanding =>
      '前回の範囲をまだ確認中です — 最大で2日ほどかかることがありますが、通常はもっと短時間で終わります。自動的に完了しますので、しばらく待ってから、もう一度確認してください。';

  @override
  String get walletDeepScanTorUnknownHint =>
      '現在、接続のプライバシー状態をまだ確認できません。プライバシーを高めるため、Torが有効になってから確認することをご検討ください。';

  @override
  String get walletDeepScanBannerChecking =>
      'より古いスワップアドレスをまだ確認中です — 見つかったものは残高に反映されます。';

  @override
  String get walletRescanSwapPointer =>
      '古いスワップからの資金をお探しですか?再スキャンではそれは見つかりません — 代わりに「より古いスワップアドレスを確認」をご利用ください。';

  @override
  String get walletDeepScanRestoreNoteTitle => 'スワップを利用していたウォレットを復元しましたか?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'このウォレットのスワップ履歴が非常に長い場合、最も古いスワップの資金は見つけるのに追加の手順が必要になることがあります。ほとんどのウォレットでは何も必要ありません。';

  @override
  String get walletDeepScanRestoreNoteCheck => '今すぐ確認';

  @override
  String get walletDeepScanRestoreNoteDismiss => '閉じる';

  @override
  String walletTorHostPath(String transport) {
    return 'アプリのプライベート経路経由（$transport）';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'アプリのプライベート経路経由（$transport）。接続がプロキシで関連付けられる可能性があります';
  }

  @override
  String get walletTorHostOtherTransport => 'プライベート経路';

  @override
  String get walletTorHostDirect => '非プライベート（アプリの直接接続）';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return '保存されたサーバーは暗号化されていないアドレスを使用しており、アプリのプライベート経路では送信できません。$host を使用しています。';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return '$labelの詳細';
  }

  @override
  String get walletSendPaste => '貼り付け';

  @override
  String get walletSendScanQr => 'QRコードをスキャン';

  @override
  String get walletSendRecipientGetsLabel => '受取人の受取額';

  @override
  String get walletSwapDepositCopyAmount => '金額をコピー';

  @override
  String get walletSwapDepositAmountCopied => '金額をコピーしました';

  @override
  String get walletScanOpenSettings => '設定を開く';

  @override
  String get walletScanOpenSettingsFailed => '設定を開けませんでした。';

  @override
  String get walletSendLeaveTitle => '送信中です';

  @override
  String get walletSendLeaveBody => '画面を離れても支払いは続行されます。結果はアクティビティで確認できます。';

  @override
  String get walletSendLeaveStay => 'とどまる';

  @override
  String get walletSendLeaveConfirm => '離れる';

  @override
  String get walletSheetLeaveBody => '画面を離れても処理は続行されます。結果はアクティビティで確認できます。';

  @override
  String get walletLoadingLabel => '読み込み中';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'もう一度シールドする前に確認してください';

  @override
  String get walletShieldUnknownBody =>
      'このシールドを確認できませんでした。もう一度試す前に「履歴」を確認してください。';

  @override
  String get walletMoveUnknownTitle => 'もう一度移動する前に確認してください';

  @override
  String get walletMoveUnknownBody => 'この移動を確認できませんでした。もう一度試す前に「履歴」を確認してください。';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
