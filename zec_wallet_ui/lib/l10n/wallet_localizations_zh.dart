// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class WalletLocalizationsZh extends WalletLocalizations {
  WalletLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get walletAppearanceMenuItem => '设置';

  @override
  String get walletTitle => '钱包';

  @override
  String get walletNotSetUpTitle => '钱包尚未设置';

  @override
  String get walletNotSetUpBody =>
      '钱包设置功能将在后续版本中推出。设置过程会引导您先写下恢复短语，然后才能接收资金——因此在没有备份的情况下，资金绝不会面临风险。';

  @override
  String get walletStartupFailedTitle => '钱包无法启动';

  @override
  String get walletStartupFailedBody =>
      '出现问题，导致钱包无法在此设备上加载。如果您已有钱包，其资金不受影响——资金保存在 Zcash 网络上，可通过恢复短语找回。请重试；如果问题持续出现，请关闭应用后重新打开。';

  @override
  String get walletBalanceLabel => '余额';

  @override
  String get walletHideBalance => '隐藏余额';

  @override
  String get walletShowBalance => '显示余额';

  @override
  String get walletBalanceHiddenAmount => '余额已隐藏';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => '当前可用';

  @override
  String get walletArrivingLabel => '即将到账';

  @override
  String get walletNotSpendableYetLabel => '暂不可用';

  @override
  String get walletActivityTitle => '活动';

  @override
  String get walletActivityEmpty => '暂无活动';

  @override
  String get walletActivityError => '无法加载活动记录';

  @override
  String get walletActivityReceived => '收款';

  @override
  String get walletActivitySent => '付款';

  @override
  String get walletActivityPending => '待确认';

  @override
  String get walletActivityQueued => '已排队';

  @override
  String get walletActivityRetrying => '重试中';

  @override
  String get walletActivitySaved => '已保存';

  @override
  String get walletActivityExpired => '已过期';

  @override
  String get walletActivityFailed => '失败';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 次确认',
      one: '1 次确认',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '收到$count笔付款',
      one: '收到付款',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => '显示交易详情';

  @override
  String get walletTxDetailStatus => '状态';

  @override
  String get walletTxDetailFee => '网络费';

  @override
  String get walletTxDetailDate => '日期';

  @override
  String get walletTxDetailHeight => '区块高度';

  @override
  String get walletTxDetailMemo => '备注';

  @override
  String get walletTxDetailMemoAttached => '已附带';

  @override
  String get walletTxDetailTxid => '交易 ID';

  @override
  String get walletTxDetailCopyTxid => '复制交易 ID';

  @override
  String get walletTxDetailCopied => '交易 ID 已复制';

  @override
  String get walletTxDetailClose => '关闭';

  @override
  String get walletTxFundsKept => '没有资金从您的钱包转出';

  @override
  String get walletTxExplainQueued => '已保存在此设备上，位于「已保存并待处理」中——您可以在那里发送或取消。';

  @override
  String get walletTxExplainPending => '已发送至 Zcash 网络——正在等待区块确认。';

  @override
  String get walletTxExplainRetrying =>
      '您的钱包尚未能将此交易发送至 Zcash 网络。它会保留已签名的交易，并在每次同步时重试，直到发送成功或过期。';

  @override
  String get walletTxExplainSaved => '您的钱包已保留这笔已签名的交易，但目前不会自动发送。';

  @override
  String get walletTxExplainConfirmed => '已在 Zcash 网络上确认。';

  @override
  String get walletTxExplainExpired => '此交易在网络确认前已过期，因此已被取消。该金额仍归您所有，可供支用。';

  @override
  String get walletTxExplainFailed => '网络拒绝了此交易，因此未能完成。该金额仍归您所有，可供支用。';

  @override
  String get walletTxExplainUnknown => '无法确定此交易的当前状态。下次同步后将会更新。';

  @override
  String get walletMenuTooltip => '更多选项';

  @override
  String get walletRescanMenuItem => '重新扫描历史记录…';

  @override
  String get walletCheckOneTimeMenuItem => '检查一次性地址…';

  @override
  String get walletRescanTitle => '重新扫描您的历史记录';

  @override
  String get walletRescanBody =>
      '缺少较早的资金？从更早的时间点重新扫描区块链，以找回因起始日期较晚而遗漏的存款。您的资金和恢复短语绝不会面临风险。';

  @override
  String get walletRescanRangeTitle => '扫描回溯的时间范围';

  @override
  String get walletRescanRangeAll => '扫描您的全部历史记录——速度最慢，但可找回所有资金。';

  @override
  String get walletRescanRangeDefault =>
      '从您钱包的起始点开始扫描。仍然缺少较早的资金？请选择更早的日期，或选择「扫描全部历史记录」。';

  @override
  String get walletRescanRangeResolving => '正在准备推荐的扫描范围…';

  @override
  String walletRescanEstimate(String blocks) {
    return '约需扫描 $blocks 个区块。';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return '从 $date 开始扫描。仍然缺少较早的资金？请选择更早的日期，或选择「扫描全部历史记录」。';
  }

  @override
  String get walletRescanPick => '选择日期';

  @override
  String get walletRescanChange => '更改日期';

  @override
  String get walletRescanScanAll => '扫描全部历史记录';

  @override
  String get walletRescanDatePick => '最早扫描日期';

  @override
  String get walletRescanWarning =>
      '此操作将重新扫描区块链。较近的日期只需几分钟；回溯较远则可能需要数小时。同步在后台运行——您可以继续使用钱包。';

  @override
  String get walletRescanSettlingAdvisory =>
      '此钱包中的一笔付款仍在确认中。钱包通常会在其完成前拒绝重新扫描——您可以尝试，但预计会被拒绝。';

  @override
  String get walletRescanConfirm => '开始重新扫描';

  @override
  String get walletRescanCancel => '取消';

  @override
  String get walletRescanRunning => '正在重建…';

  @override
  String get walletRescanRebuildingAll =>
      '正在重建您的历史记录——扫描整条区块链。随着进度追赶，您的余额和活动记录将逐步填充。';

  @override
  String walletRescanRebuildingFrom(String date) {
    return '正在从 $date 重建您的历史记录——随着进度追赶，您的余额和活动记录将逐步填充。';
  }

  @override
  String get walletRescanRebuildingDefault =>
      '正在从您钱包的起始点重建您的历史记录——随着进度追赶，您的余额和活动记录将逐步填充。';

  @override
  String get walletCatchUpBanner =>
      '正在追赶进度——钱包同步时，您的余额和活动记录将逐步填充。您收到的任何款项均安全无虞。';

  @override
  String get walletCatchUpRescanBanner =>
      '重新扫描后，正在重建您的历史记录——随着进度追赶，您的余额和活动记录将逐步填充。您收到的任何款项均安全无虞。';

  @override
  String get walletRescanFailedNotice =>
      '目前无法重新扫描——您的资金安全无虞，只是余额和历史记录可能需要一点时间才能赶上进度。请稍后再试。';

  @override
  String get walletRescanBlockedSettlingNotice =>
      '付款仍在确认中，因此系统暂停了重新扫描以保护您的资金。您的钱包未发生任何变化——请在几个小时后重试，并在此期间保持应用打开且联网。';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      '重新扫描会随着钱包同步而重建您的历史记录，且同步目前未在运行。您的钱包未发生任何变化——请在同步运行后重试。';

  @override
  String get walletRescanNeedsSpaceNotice =>
      '可用存储空间不足，无法重建您的钱包历史记录——您的资金安全无虞，只是余额和历史记录可能需要一点时间才能赶上进度。请释放一些空间后重试。';

  @override
  String get walletRescanFailedDismiss => '关闭';

  @override
  String get walletActivityRebuilding => '正在重建您的历史记录…';

  @override
  String get walletActivityCatchingUp => '仍在追赶进度——您收到的任何款项都会显示在此处。';

  @override
  String get walletActivitySyncNotRunning => '同步运行后，您的余额和历史记录将完成加载。';

  @override
  String get walletActivityLoadMore => '加载更多';

  @override
  String get walletPendingChangeLabel => '待确认找零';

  @override
  String get walletTransparentLabel => '未屏蔽（公开）';

  @override
  String get walletTransparentNote =>
      '不计入「当前可用」——需先屏蔽这些资金才能使用。在此之前，它们在链上仍是公开可见的。';

  @override
  String get walletTransparentNoteWatchOnly => '这些资金在链上仍是公开可见的。';

  @override
  String walletPoolShielded(String amount) {
    return '已屏蔽 $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return '公开 $amount';
  }

  @override
  String get walletPoolAllShielded => '全部已屏蔽 · 私密';

  @override
  String get walletPoolTapHint => '显示公开资金';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '您余额中的 $amount 位于一个一次性地址上（可恢复）。';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '您余额中的 $amount 位于一个一次性地址上。';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$amount 已被预留用于付款，仍在通过您钱包所控制的一次性地址完成中。请勿重复发送。',
      one: '$amount 已被预留用于付款，仍在通过您钱包所控制的一次性地址完成中。请勿重复发送。',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$amount 已被预留用于付款，仍通过您钱包所控制的一次性地址完成到一半。暂停中，直到您的钱包再次同步。请勿重复发送。',
      one: '$amount 已被预留用于付款，仍通过您钱包所控制的一次性地址完成到一半。暂停中，直到您的钱包再次同步。请勿重复发送。',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      '无法检查是否有付款仍在完成中。正在重试——在此之前，请先在您的活动中查看是否有待处理的付款，然后再重新发送。';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '您余额中的 $amount 位于一个一次性地址上（仍在确认中）。';
  }

  @override
  String get walletShieldButton => '屏蔽';

  @override
  String get walletShieldSheetTitle => '屏蔽公开资金';

  @override
  String get walletShieldNote => '此操作会将资金从您公开的、链上可见的余额转入您的私密屏蔽余额。';

  @override
  String get walletShieldPreparing => '正在准备…';

  @override
  String get walletShieldAmountLabel => '屏蔽金额';

  @override
  String get walletShieldFeeLabel => '网络费';

  @override
  String get walletShieldNetLabel => '屏蔽入账';

  @override
  String get walletShieldConfirmButton => '立即屏蔽';

  @override
  String get walletShieldSubmitting => '正在屏蔽…';

  @override
  String get walletShieldNothingTitle => '暂无可屏蔽资金';

  @override
  String get walletShieldNothingBody =>
      '这些资金目前低于值得屏蔽的金额——网络费用会超过屏蔽带来的收益。待稍有更多资金到账后即可屏蔽。';

  @override
  String get walletShieldDoneTitle => '屏蔽已提交';

  @override
  String get walletShieldDoneBody => '您的资金正在转入屏蔽余额，将很快在链上得到确认。';

  @override
  String get walletShieldSavedTitle => '已保存——我们将完成屏蔽';

  @override
  String get walletShieldSavedBody =>
      '目前无法连接网络。您的屏蔽操作已保存，您的钱包将在之后的同步中完成。资金不会有任何损失。';

  @override
  String get walletShieldAlreadyTitle => '已提交';

  @override
  String get walletShieldFailedTitle => '目前无法屏蔽';

  @override
  String get walletShieldStaleBody => '钱包仍在同步中，请稍后再次尝试屏蔽。';

  @override
  String get walletShieldTransientBody => '暂时无法准备屏蔽操作。请稍后再试。';

  @override
  String get walletShieldStorageFullBody =>
      '可用存储空间不足，目前无法屏蔽。请释放一些空间后重试。您的资金是安全的。';

  @override
  String get walletShieldClose => '关闭';

  @override
  String get walletShieldRetry => '重试';

  @override
  String get walletMoveMenuItem => '转为公开…';

  @override
  String get walletMoveSheetTitle => '转为公开';

  @override
  String get walletMoveSheetSubtitle => '将屏蔽 ZEC 发送到您自己的公开地址——适用于不接受屏蔽存款的交易所。';

  @override
  String get walletMoveDestinationLabel => '您的公开地址';

  @override
  String walletMoveAvailable(String amount) {
    return '可转出：$amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return '可转出：$amount ZEC——您的余额仍在追赶进度';
  }

  @override
  String get walletMoveDeshieldTitle => '此操作将使您的资金公开';

  @override
  String get walletMoveDeshieldBody =>
      '转至公开地址会将这些资金移出您的屏蔽余额——金额及您的公开地址将在 Zcash 区块链上公开可见。';

  @override
  String get walletMoveWalletEnded => '钱包会话已结束。请关闭后重新打开以再次尝试。';

  @override
  String get walletMoveLoading => '正在准备…';

  @override
  String get walletMovePreparing => '正在核对金额…';

  @override
  String get walletMoveSubmitting => '正在转移…';

  @override
  String get walletMoveReviewButton => '预览';

  @override
  String get walletMoveCancel => '取消';

  @override
  String get walletMoveReviewTitle => '预览转移';

  @override
  String get walletMoveOwnAddressNote =>
      '您正在转移至自己的公开地址。您之后可以再次屏蔽这些资金，但此次转移将永久保留在公开记录中。';

  @override
  String get walletMoveConfirmButton => '转为公开';

  @override
  String get walletMoveBackButton => '返回';

  @override
  String get walletMoveDoneTitle => '已转入公开地址';

  @override
  String get walletMoveDoneBody => '您的资金正在转入公开地址，很快将在链上确认。';

  @override
  String get walletMoveSavedTitle => '已保存——我们将完成转移';

  @override
  String get walletMoveSavedBody => '此次转移已保存，您的钱包将在之后的同步中发送。资金没有任何损失。';

  @override
  String get walletMoveAlreadyTitle => '已提交';

  @override
  String get walletMoveAlreadyBody => '这笔资金已经提交，正在转往您的公开地址。';

  @override
  String get walletMoveFailedTitle => '无法完成此次转移';

  @override
  String get walletMoveNothingTitle => '暂无可转移资金';

  @override
  String get walletMoveNothingBody => '您目前没有可转移的屏蔽余额。资金确认后，您就可以将其转入公开地址。';

  @override
  String get walletMoveNothingCatchingUpBody =>
      '您的钱包仍在追赶进度——您收到的任何款项将在同步完成后变为可转出。';

  @override
  String get walletMoveCouldNotLoad => '无法加载您的公开地址，请重试。';

  @override
  String get walletMoveRetry => '重试';

  @override
  String get walletMoveClose => '关闭';

  @override
  String get walletSnapshotUnavailable => '目前无法读取钱包，它将自动刷新。';

  @override
  String get walletBalanceStale => '无法刷新——显示您上次已知的余额。';

  @override
  String get walletSyncStartFailed => '无法开始同步，我们将持续重试。';

  @override
  String get walletSyncRetry => '重试';

  @override
  String get walletSyncTryNow => '立即尝试';

  @override
  String get walletSyncIdle => '尚未开始同步';

  @override
  String get walletSyncIdleDetail => '同步将自动开始。';

  @override
  String get walletSyncDisabled => '同步已关闭';

  @override
  String get walletSyncDisabledDetail => '在本应用的设置中开启同步以更新您的余额。';

  @override
  String get walletSyncExplainDisabled =>
      '同步已在本应用的设置中关闭。您的资金是安全的。您的余额和活动显示的是上次同步时的状态，在同步重新开启之前不会更新。';

  @override
  String get walletParkedSyncPausedNote =>
      '您的钱包未在同步，因此这些付款不会自行发送。请使用「立即发送」自行发送一笔。';

  @override
  String get walletSyncPausedMoneyNote => '暂停中，直到您的钱包再次同步。';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body$note';
  }

  @override
  String get walletSyncStarting => '正在连接…';

  @override
  String get walletSyncStartingDetail => '正在连接 Zcash 网络并准备扫描。';

  @override
  String get walletSyncConnecting => '正在连接…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return '正在连接… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return '正在扫描 $percent%';
  }

  @override
  String get walletSyncScanningEarly => '正在扫描…';

  @override
  String get walletSyncSpendableReady => '资金已可使用。';

  @override
  String get walletSyncCatchingUp => '正在追赶网络进度——深度初始同步可能需要一段时间。完成之前您仍可继续使用本应用';

  @override
  String walletSyncScanRemaining(String count) {
    return '剩余 $count 个区块';
  }

  @override
  String get walletSyncUpToDate => '已是最新';

  @override
  String get walletSyncOffline => '离线';

  @override
  String get walletSyncOfflineDetail => '排队中的付款会保留在「已保存并待处理」中。';

  @override
  String get walletSyncUnknown => '正在同步…';

  @override
  String get walletSyncStalled => '同步已暂停';

  @override
  String get walletStallEndpoint =>
      '目前无法连接到 Zcash 网络。我们将自动持续重试——请检查您的网络连接，也可能是服务器暂时不可用。';

  @override
  String get walletStallTor =>
      '您应用的隐私通道不可用，因此钱包未在连接。请检查您应用的网络设置，或关闭隐私通道。通道恢复后同步将继续。';

  @override
  String get walletStallStorage => '设备存储空间已满。请清理空间，同步将随即恢复。';

  @override
  String get walletStallReorg => '链发生重组，正在重新检查最近的区块。';

  @override
  String get walletStallInternal => '本地问题导致同步中断。如果此情况持续发生，请使用恢复短语还原钱包。';

  @override
  String get walletStallEndpointMisbehaving =>
      '此服务器发送的数据不可能正确，因此同步已停止。这不是连接问题——请切换到其他服务器。如果所有服务器都被拒绝，请重新扫描历史记录：钱包可能保留了来自早前服务器的错误记录。';

  @override
  String get walletStallBirthdayInFuture =>
      '此钱包设置为从一个该服务器尚未到达的区块开始。请检查此钱包设置的起始区块，或尝试其他服务器。';

  @override
  String get walletStallStorageUnavailable => '此设备上的同步已暂停。正在重试。';

  @override
  String get walletStallUnknown => '同步因未知原因而停止。';

  @override
  String get walletSyncBadgeHint => '显示同步详情';

  @override
  String get walletSyncSheetClose => '关闭';

  @override
  String get walletSyncSheetProgress => '进度';

  @override
  String get walletSyncSheetBlocksLeft => '剩余区块数';

  @override
  String get walletSyncSheetSyncedTo => '已同步至区块';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '至少落后 $blocks 个区块',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle => '同步尚未开始——它会自动开始，无需任何操作。';

  @override
  String get walletSyncExplainStartFailed =>
      '同步未能启动。您的资金安全无虞——钱包目前只是没有在检查新的活动。请在下方重试，或重新打开应用。';

  @override
  String get walletSyncExplainStarting => '钱包正在联系 Zcash 网络并准备扫描，通常只需几秒钟。';

  @override
  String get walletSyncExplainConnecting => '正在与 Zcash 网络建立连接。';

  @override
  String get walletSyncExplainScanning =>
      '钱包正在检查区块链中的区块以查找您的资金。发现新交易时，余额和活动记录会随之更新——同步完成前，您仍可继续使用本应用。';

  @override
  String get walletSyncExplainUpToDate => '已与 Zcash 网络完全同步。您的余额和活动记录均为最新。';

  @override
  String get walletSyncExplainStalled => '同步遇到问题并已暂停，系统会自动重试。';

  @override
  String get walletSyncExplainStalledOffline =>
      '无法连接到 Zcash 网络——如果您处于离线状态，这属于正常情况，也可能是服务器暂时不可用。您的资金是安全的：余额显示的是上次同步时的状态，排队中的转账会保留在「已保存并待处理」中。连接会自动重试。';

  @override
  String get walletSyncExplainOffline =>
      '没有网络连接。您的资金是安全的——余额显示的是上次同步时的状态，排队中的转账会保留在「已保存并待处理」中。';

  @override
  String get walletSyncExplainUnknown => '钱包正在同步。余额和活动记录会随进度更新。';

  @override
  String get walletTorOff => 'Tor 已关闭';

  @override
  String get walletTorBootstrapping => '隐私通道正在启动…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return '$transport 正在启动…';
  }

  @override
  String get walletTorActive => 'Tor 已启用';

  @override
  String get walletTorActiveUnverified => 'Tor 已启用（运行环境未验证）';

  @override
  String get walletTorActiveUnattested => '正在使用隐私通道（隐私性未验证）';

  @override
  String get walletTorFellBack => 'Tor 不可用——正在使用直接连接';

  @override
  String get walletTorUnavailable => '隐私通道不可用——未连接';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport 不可用——未连接';
  }

  @override
  String get walletTorUnanswered => '隐私通道已连接——没有任何回应';

  @override
  String get walletTorUnansweredUnattested => '隐私通道已连接——没有任何回应（隐私性未验证）';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport 已连接——没有任何回应';
  }

  @override
  String get walletTorUnansweredDirect => '非隐私（您应用的直接连接）——没有任何回应';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return '经由 $transport 已连接——没有任何回应；代理可以关联各个连接';
  }

  @override
  String get walletTorUnknown => 'Tor 状态未知——请视为未受保护';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return '余额（截至区块 $height）';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return '余额 · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return '余额（截至区块 $height，$time）';
  }

  @override
  String get walletSyncSheetConnection => '连接';

  @override
  String get walletSyncSheetServer => '服务器';

  @override
  String walletSyncServerRowSemantics(String host) {
    return '服务器，$host，打开服务器选择';
  }

  @override
  String get walletSyncServerSheetTitle => '同步服务器';

  @override
  String get walletSyncServerInUse => '使用中';

  @override
  String get walletSyncServerAppDefault => '应用默认';

  @override
  String get walletSyncServerCustom => '自定义服务器…';

  @override
  String get walletSyncServerCustomHint => 'https://主机:端口';

  @override
  String get walletSyncServerCheck => '检查服务器';

  @override
  String get walletSyncServerChecking => '正在检查…';

  @override
  String get walletSyncServerUse => '使用此服务器';

  @override
  String get walletSyncServerSwitching => '正在切换…';

  @override
  String get walletSyncServerContinue => '继续';

  @override
  String get walletSyncServerCancel => '取消';

  @override
  String get walletSyncServerTrustTitle => '信任此服务器？';

  @override
  String get walletSyncServerTrustNotice =>
      '您将信任此服务器报告您的余额和历史记录并转发您的付款。除非已开启 Tor，否则它会看到您的 IP 地址、钱包大致的创建时间、钱包检查的公开地址、钱包查询的交易以及您发送的交易。';

  @override
  String get walletSyncServerKeyLabel => '访问密钥（可选）';

  @override
  String get walletSyncServerKeyHeaderLabel => '密钥标头';

  @override
  String get walletSyncServerKeyHeaderNeeded => '请输入您的服务器要求的标头';

  @override
  String get walletSyncServerKeyInvalid => '无法使用此密钥或标头';

  @override
  String get walletSyncServerKeySaved => '密钥已保存';

  @override
  String get walletSyncServerKeyShow => '显示';

  @override
  String get walletSyncServerKeyHide => '隐藏';

  @override
  String get walletSyncServerTrustNoticeKey =>
      '您的密钥会向此服务器表明您的身份。即使通过 Tor，它也能将您的付款与您的钱包关联起来。';

  @override
  String get walletSyncServerSwitchNotice =>
      '切换会重新开始正在进行的同步。您的余额和历史记录会保留。在新服务器的扫描赶上之前，资金可能显示为即将到账。';

  @override
  String get walletSyncServerSwitchNoticeAtTip => '切换会重新连接到新服务器。您的余额和历史记录会保留。';

  @override
  String get walletSyncServerUnreachable =>
      '无法连接到此服务器。请检查地址——如果地址无误，则可能是此服务器没有回应，也可能是您的应用当前无法连上它。请重试，或换一台服务器。';

  @override
  String get walletSyncServerUnreachableOffered =>
      '无法连接到此服务器。是此服务器没有回应，还是您的应用当前无法连上它，钱包无法分辨。请换一台服务器，或稍后重试。';

  @override
  String get walletSyncServerWrongNetwork => '此服务器属于另一个 Zcash 网络。';

  @override
  String get walletSyncServerInvalidUrl => '这看起来不像服务器地址。请使用 https://主机:端口 的格式。';

  @override
  String get walletSyncServerNotOffered => '此应用未提供该服务器。';

  @override
  String get walletSyncServerBusy => '钱包当前正忙。请稍后重试。';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return '您选择的服务器已不再由此应用提供。当前使用 $host。';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return '无法读取已保存的服务器选择。当前使用 $host。';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return '无法切换 — 仍在使用 $host。';
  }

  @override
  String get walletTransportExplainDirect => '钱包流量直接连接到服务器，服务器可以看到您的 IP 地址。';

  @override
  String get walletTransportExplainTor => '钱包流量通过 Tor 网络传输，可对服务器隐藏您的 IP 地址。';

  @override
  String get walletTransportExplainBootstrapping =>
      '您应用的隐私通道正在启动。钱包流量将等待其就绪后再连接。';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport 正在启动。钱包流量将等待其就绪后再连接。';
  }

  @override
  String get walletTransportExplainFellBack =>
      '无法连接到 Tor,流量已回退为直接连接。服务器可以看到您的 IP 地址。';

  @override
  String get walletTransportExplainUnavailable =>
      '您应用的隐私通道不可用，因此钱包未在连接。请关闭隐私通道，或检查您应用的网络设置。';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport 不可用，因此钱包未在连接。请将其关闭，或检查您应用的网络设置。';
  }

  @override
  String get walletTransportExplainUnanswered =>
      '隐私通道已接受此连接，但已有一分钟没有任何回应。可能是通道，也可能是钱包服务器——钱包无法分辨。钱包会持续重试；如果一直不恢复，请换一台服务器，或检查您应用的网络设置。';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport 已接受此连接，但已有一分钟没有任何回应。可能是通道，也可能是钱包服务器——钱包无法分辨。钱包会持续重试；如果一直不恢复，请换一台服务器，或检查您应用的网络设置。';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      '钱包流量直接连接到服务器，服务器可以看到您的 IP 地址。此连接已被接受，但已有一分钟没有任何回应。可能是通道，也可能是钱包服务器——钱包无法分辨。钱包会持续重试；如果一直不恢复，请换一台服务器，或检查您应用的网络设置。';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      '此连接的隐私性无法验证——请视为不私密。此连接已被接受，但已有一分钟没有任何回应。可能是通道，也可能是钱包服务器——钱包无法分辨。钱包会持续重试；如果一直不恢复，请换一台服务器，或检查您应用的网络设置。';

  @override
  String get walletTransportExplainUnverified => '此连接的隐私性无法验证——请视为不私密。';

  @override
  String get walletTransportExplainHostProxy =>
      '钱包流量通过本应用的隐私传输通道路由，可对服务器隐藏您的 IP 地址。';

  @override
  String get walletOnboardingWelcomeTitle => '设置您的钱包';

  @override
  String get walletOnboardingWelcomeBody =>
      '创建一个新钱包以接收和持有 ZEC。在任何资金到账之前，我们会生成一个恢复短语，并引导您完成备份——确保资金在未备份的情况下绝不会面临风险。';

  @override
  String get walletCreateButton => '创建新钱包';

  @override
  String get walletRestoreButton => '通过恢复短语还原';

  @override
  String get walletWatchOnlyButton => '观察钱包（仅观察）';

  @override
  String get walletWatchOnlyTitle => '观察钱包';

  @override
  String get walletWatchOnlyBody =>
      '粘贴查看密钥即可观察钱包，而无需持有其花费密钥。您可以查看余额和交易记录，但无法发送资金。请选择钱包的大致起始日期，以便我们知道需要回溯查找多久。';

  @override
  String get walletWatchOnlyKeyLabel => '查看密钥';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip => '扫描查看密钥 QR 码';

  @override
  String get walletWatchOnlyScanTitle => '扫描查看密钥 QR 码';

  @override
  String get walletWatchOnlyScanInstruction => '将摄像头对准查看密钥 QR 码。';

  @override
  String get walletWatchOnlyScanCameraUnavailable => '摄像头不可用。请改为手动粘贴密钥。';

  @override
  String get walletWatchOnlyScanManualEntry => '改为粘贴';

  @override
  String get walletWatchOnlyScanHint => '或点击扫描按钮以读取查看密钥 QR 码。';

  @override
  String get walletWatchOnlyScanFilled => '查看密钥已扫描。';

  @override
  String get walletWatchOnlyBirthdayTitle => '钱包起始日期';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return '从 $date 开始扫描——在此之前收到的资金将不会显示。钱包更早创建？请选择更早的日期。';
  }

  @override
  String get walletWatchOnlyBirthdayPick => '选择钱包的起始日期';

  @override
  String get walletWatchOnlyBirthdayChange => '更改日期';

  @override
  String get walletWatchOnlySubmit => '观察此钱包';

  @override
  String get walletWatchOnlyBack => '返回';

  @override
  String get walletWatchOnlyFaultInvalidKey => '这看起来不是有效的查看密钥。请检查后重试。';

  @override
  String get walletWatchOnlyFaultNetworkMismatch => '该查看密钥属于其他网络，无法在此使用。';

  @override
  String get walletWatchOnlyFaultAlreadyExists => '此设备上已存在钱包。请返回并打开该钱包。';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent => '该起始日期太近。请选择更早的日期。';

  @override
  String get walletRestoreTitle => '还原您的钱包';

  @override
  String get walletRestoreBody =>
      '输入您的恢复短语以还原钱包——请按顺序输入或粘贴单词，单词之间用空格分隔。仅支持标准短语：如果您的钱包使用了额外的密码短语（「第25个单词」），本应用目前尚无法还原它——您将看到一个空钱包，而不是错误提示。';

  @override
  String get walletRestorePhraseHint => '单词一  单词二  单词三  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 个单词',
      one: '1 个单词',
      zero: '尚无单词',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint => '恢复短语应为 12、15、18、21 或 24 个单词';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count 个单词不是恢复单词——请更正高亮显示的单词',
      one: '1 个单词不是恢复单词——请更正高亮显示的单词',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return '第 $index 个单词：$word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return '第 $index 个单词：不是恢复单词';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return '删除第 $index 个单词';
  }

  @override
  String get walletRestoreSubmit => '还原钱包';

  @override
  String get walletRestoreBack => '返回';

  @override
  String get walletRestoreBirthdayTitle => '扫描起始时间';

  @override
  String get walletRestoreBirthdayNone => '我们将扫描您的全部历史记录——速度较慢，但不会遗漏任何内容。';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return '从 $date 开始扫描——在此之前收到的资金将不会显示。钱包更早创建？请选择更早的日期，或选择「扫描全部历史记录」。';
  }

  @override
  String get walletRestoreBirthdayPick => '选择日期';

  @override
  String get walletRestoreBirthdayChange => '更改日期';

  @override
  String get walletRestoreBirthdayClear => '扫描全部历史记录';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return '第 $index 个单词不是恢复单词。请检查短语中是否有拼写错误，然后重试。';
  }

  @override
  String get walletRestoreFaultInvalidPhrase => '该恢复短语无效。请检查单词及其顺序，然后重试。';

  @override
  String get walletRestoreFaultSeedMismatch => '该短语与此设备上的钱包不匹配。请仔细核对后重试。';

  @override
  String get walletRestoreFaultAlreadyExists => '此设备上已存在钱包。请返回以打开它。';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      '该日期过于接近当前。请选择更早的日期，或扫描全部历史记录。';

  @override
  String get walletGeneratingLabel => '正在创建您的钱包…';

  @override
  String get walletOpeningLabel => '正在打开您的钱包…';

  @override
  String get walletBackupTitle => '备份您的恢复短语';

  @override
  String get walletBackupBody =>
      '这些单词是恢复您钱包和资金的唯一途径。请按顺序抄写下来，并存放在安全隐秘的地方。切勿与任何人分享，也不要存储在网络上——任何掌握这些单词的人都能取走您的资金。';

  @override
  String get walletBackupSecureNoteAndroid => '此屏幕已禁用截屏功能。';

  @override
  String get walletBackupSecureNoteOther => '请确保周围无人能看到您的屏幕。';

  @override
  String get walletBackupReveal => '显示恢复短语';

  @override
  String get walletBackupRevealing => '正在准备您的恢复短语…';

  @override
  String get walletBackupRevealFailed => '暂时无法显示您的恢复短语。请确保设备已解锁，然后重试。';

  @override
  String get walletBackupRetryReveal => '重试';

  @override
  String get walletBackupReauthFailed => '无法验证您的身份。请重试。';

  @override
  String get walletBackupConfirmCheckbox => '我已抄录我的恢复短语并妥善保存。';

  @override
  String get walletBackupContinue => '继续';

  @override
  String get walletBackupSaveFailed => '无法保存您的确认。请重试。';

  @override
  String get walletBackupStartOver => '重新开始';

  @override
  String get walletBackupStartOverConfirmTitle => '放弃此钱包并重新开始？';

  @override
  String get walletBackupStartOverConfirmBody =>
      '此操作会从设备中删除此钱包并返回起始页面。设置完成前，无法通过本应用存入任何资金。\n\n如果此钱包曾持有资金——或曾通过恢复短语恢复——只有该短语才能将其找回。';

  @override
  String get walletBackupStartOverConfirm => '删除并重新开始';

  @override
  String get walletBackupStartOverKeep => '保留此钱包';

  @override
  String get walletBackupSectionTitle => '恢复短语';

  @override
  String get walletBackupTileTitle => '备份您的恢复短语';

  @override
  String get walletBackupTileSubtitle => '显示用于恢复您钱包和资金的单词。';

  @override
  String get walletBackupScreenTitle => '恢复短语';

  @override
  String get walletBackupDone => '完成';

  @override
  String get walletBackupManagedTitle => '没有独立的恢复短语';

  @override
  String get walletBackupManagedBody =>
      '此钱包是使用安装它的应用中的账户设置的，因此没有属于自己的恢复短语。您的资金将与该账户一起恢复——请使用该账户的备份以确保资金安全。';

  @override
  String get walletExportViewingKeyTitle => '导出查看密钥';

  @override
  String get walletExportViewingKeyTileTitle => '导出查看密钥';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      '分享您钱包的只读副本——可以查看交易记录，但无法花费资金。';

  @override
  String get walletExportViewingKeyWarning =>
      '持有此密钥的任何人都可以查看此钱包过去收到和发送的所有交易，以及未来的所有交易。它无法花费您的资金，也无法恢复您的钱包。请仅与您信任、愿意让其查看您完整交易记录的人分享，例如您的会计师或您自己的第二台设备。日后取消分享的唯一方法是将资金转移到一个新钱包。';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      '持有此密钥的任何人都可以查看此钱包过去收到和发送的所有交易，以及未来的所有交易。它无法花费您的资金，也无法恢复您的钱包。请仅与您信任、愿意让其查看您完整交易记录的人分享，例如您的会计师或您自己的第二台设备。一旦分享，就无法撤销。';

  @override
  String get walletExportViewingKeyReveal => '显示查看密钥';

  @override
  String get walletExportViewingKeyRetry => '重试';

  @override
  String get walletExportViewingKeyRevealing => '正在准备您的查看密钥…';

  @override
  String get walletExportViewingKeyFailed => '暂时无法显示您的查看密钥。请稍后重试。';

  @override
  String get walletExportViewingKeyQrLabel => '查看密钥二维码';

  @override
  String get walletExportViewingKeyCopy => '复制查看密钥';

  @override
  String get walletExportViewingKeyCopied => '查看密钥已复制';

  @override
  String get walletExportViewingKeyDone => '完成';

  @override
  String get walletExportViewingKeySecureNoteAndroid => '此屏幕已禁用截屏功能。';

  @override
  String get walletExportViewingKeySecureNoteOther => '请确保周围无人能看到您的屏幕。';

  @override
  String get walletWatchOnlySectionTitle => '关于此仅观察钱包';

  @override
  String get walletWatchOnlyAboutBody =>
      '这是一个仅观察钱包。它是通过查看密钥设置的，因此可以查看余额和交易记录，但不持有花费密钥——这里没有需要备份的内容，也无法发送资金。';

  @override
  String get walletWatchOnlyBadge => '仅观察';

  @override
  String get walletOnboardingFailedTitle => '钱包设置未能完成';

  @override
  String get walletOnboardingRetry => '重试';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      '您手机的安全存储没有响应。请解锁设备后重试。如果问题持续出现，请重启手机。';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      '此钱包正在另一个窗口或应用中打开，或仍在完成上一次操作。请关闭正在使用它的其他窗口，或稍等片刻，然后重试。';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      '此钱包的安全密钥已不可用，因此无法在此设备上打开。您的资金是安全的——请使用恢复短语进行恢复。';

  @override
  String get walletOnboardingFailedRestoreAction => '使用恢复短语恢复';

  @override
  String get walletOnboardingRecoverConfirmTitle => '恢复此钱包？';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      '继续前请确保您已备好恢复短语——下一屏需要用它来恢复您的资金。您的资金安全存储在区块链上，并由该短语控制。此操作将从本设备移除无法读取的钱包数据，以便重新构建。';

  @override
  String get walletOnboardingRecoverConfirmCancel => '取消';

  @override
  String get walletOnboardingFailedStorageFull =>
      '可用存储空间不足，无法设置您的钱包。请释放一些空间后重试。';

  @override
  String get walletOnboardingFailedNoVault => '此设备没有安全密钥存储，因此钱包无法在此保护您的恢复短语。';

  @override
  String get walletOnboardingFailedNetwork => '设置过程中无法连接到网络。请检查您的网络连接后重试。';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      '钱包设置未完成。请重试以完成设置——没有任何数据丢失。';

  @override
  String get walletOnboardingFailedUnknown => '设置钱包时出现问题。请重试。';

  @override
  String get walletOnboardingFailedConfiguration =>
      '此应用的钱包配置有误，导致钱包无法启动。重试无法解决问题——请将此情况报告给该应用的开发者。您的资金不受影响。';

  @override
  String get walletSendButton => '发送';

  @override
  String get walletSendSyncNotRunning => '同步未在运行——您的可用余额无法更新';

  @override
  String get walletSendWaitingForFunds => '仍在同步——您有可用余额后即可发送';

  @override
  String get walletSendNoSpendableYet => '暂无可用余额';

  @override
  String get walletSendSyncUnavailable => '同步恢复后即可发送';

  @override
  String get walletSendTitle => '发送';

  @override
  String get walletSendUnavailable => '您的钱包目前尚未就绪。请返回后重试。';

  @override
  String get walletSendWatchOnly =>
      '这是一个仅观察钱包。它可以显示余额和接收付款，但不持有任何花费密钥——因此无法发送。';

  @override
  String get walletSendExpiredTitle => '此付款请求已过期';

  @override
  String get walletSendExpiredBody =>
      '发送页面打开耗时超过五秒，因此应用已被告知未发送任何内容。该答复为最终结果：无法从此处支付此请求。如需支付，请从应用重新开始。';

  @override
  String get walletSendFaultWatchOnly => '这是一个仅观察钱包——它不持有任何花费密钥，因此无法发送。';

  @override
  String walletSendAvailable(String amount) {
    return '可发送余额：$amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return '可发送余额：$amount ZEC——您的余额仍在追赶进度';
  }

  @override
  String get walletSendRecipientLabel => '收款地址';

  @override
  String get walletSendRecipientHint => 'Zcash 地址（以 u、z 或 t 开头）';

  @override
  String get walletSendRecipientLocked => '收款人在此处无法更改';

  @override
  String get walletSendAmountLabel => '金额（ZEC）';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => '备注（可选）';

  @override
  String get walletSendMemoHint => '仅可送达屏蔽（隐私）收款人';

  @override
  String get walletSendMemoTransparentDisabled => '备注需要屏蔽收款人。此公开地址无法接收备注。';

  @override
  String get walletSendMemoMachineDisabled => '此付款已带有应用的引用信息，因此不能再附加手写备注。';

  @override
  String get walletSendMachineMemoTitle => '应用正在附加一条引用信息';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return '应用称其用途为：$purpose';
  }

  @override
  String get walletSendMachineMemoLimit => '它会随交易一起保留，之后无法移除。钱包无法核实其内容。';

  @override
  String get walletSendRecipientShielded => '屏蔽 · 隐私';

  @override
  String get walletSendRecipientTransparent => '公开';

  @override
  String get walletSendRecipientInvalid => '这看起来不是有效的 Zcash 地址。';

  @override
  String get walletSendRecipientWrongNetwork => '此地址属于其他 Zcash 网络。';

  @override
  String get walletSendReviewButton => '核对付款';

  @override
  String get walletSendQueueButton => '加入队列稍后发送';

  @override
  String get walletSendQueueHint =>
      '已排队的付款保留在「已保存并待处理」中，您可以在那里发送或取消。网络手续费会在发送时计算。';

  @override
  String get walletSendPreparing => '正在准备您的付款…';

  @override
  String get walletSendSubmitting => '发送中…';

  @override
  String get walletSendQueuing => '加入队列中…';

  @override
  String get walletSendReviewTitle => '确认付款';

  @override
  String get walletSendTotalLabel => '总计';

  @override
  String get walletSendFeeLabel => '网络手续费';

  @override
  String get walletSendChangeLabel => '找零返还';

  @override
  String get walletSendDeshieldTitle => '此付款不具备隐私保护';

  @override
  String get walletSendDeshieldBody => '该付款发送至公开地址，因此金额和收款人将在 Zcash 区块链上公开可见。';

  @override
  String get walletSendPublicAckLabel => '我了解这笔付款将是公开的。';

  @override
  String get walletSendConfirmButton => '立即发送';

  @override
  String get walletSendBackButton => '返回';

  @override
  String get walletSendSelfSendNote => '您正在向自己的钱包发送。网络手续费仍将收取。';

  @override
  String get walletSendLargeConfirmTitle => '发送大额付款？';

  @override
  String get walletSendLargeConfirmNearTotal => '这几乎是您的全部余额。付款一旦发送即无法撤销。';

  @override
  String get walletSendLargeConfirmOverThreshold => '这是一笔大额付款。付款一旦发送即无法撤销。';

  @override
  String get walletSendLargeConfirmBoth => '这是一笔大额付款——几乎是您的全部余额。付款一旦发送即无法撤销。';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return '发送 $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => '返回';

  @override
  String get walletSendSentTitle => '付款已发送';

  @override
  String get walletSendSentBody => '您的付款已广播至网络。';

  @override
  String get walletSendSavedTitle => '已保存——我们将继续完成发送';

  @override
  String get walletSendSavedBody => '您的付款目前未能发出，已被保存，您的钱包将在之后的同步中发送。资金不会有任何损失。';

  @override
  String get walletSendKeptTitle => '已保存';

  @override
  String get walletSendKeptBody => '您的钱包已保留这笔交易，但不会承诺自动发送。请在“活动”中查看其状态。';

  @override
  String get walletSendPartialBody => '您的部分付款已发出；您的钱包将在之后的同步中完成剩余部分。资金不会有任何损失。';

  @override
  String get walletSendInMotionTitle => '付款处理中';

  @override
  String get walletSendInMotionBody =>
      '您的付款已启动，正通过您钱包控制的一次性地址转移中。请勿重复发送。如未能完成，您可以在钱包主屏幕中恢复资金。';

  @override
  String get walletSendAlreadyTitle => '已提交';

  @override
  String get walletSendAlreadyBody => '此付款已提交——不会被重复发送。';

  @override
  String get walletSendFailedTitle => '付款未能完成';

  @override
  String get walletSendFailedBody => '完成此付款时出现问题，未发送任何款项。您可以重试。';

  @override
  String get walletSendTryAgain => '重试';

  @override
  String get walletSendDone => '完成';

  @override
  String get walletSendAnother => '再发一笔';

  @override
  String get walletSendQueuedTitle => '已加入发送队列';

  @override
  String get walletSendQueuedBody => '此付款已保存。您可以在「已保存并待处理」中找到它，并在那里立即发送或取消。';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return '可用余额不足——您拥有 $available ZEC，此付款需要 $required ZEC。';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Zcash 网络已升级，此应用需要更新后才能发送。您的资金是安全的。';

  @override
  String get walletSyncUpToDateLimited => '已同步至此版本能够读取的范围';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Zcash 网络已升级。此版本已扫描它能读取的全部内容，但较新的区块可能包含它尚无法显示的资金，近期付款的备注也无法读取。请更新应用以查看全部内容。';

  @override
  String get walletSyncUpToDateDegraded => '已同步，但此服务器未提供所有资金池';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      '此服务器拒绝、隐瞒或错误报告了 Zcash 的一个隐私资金池。在该资金池中收到的资金无法通过此服务器花费，显示的余额是下限。请切换到其他服务器以使用这些资金——这不是连接问题。';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool：此服务器拒绝提供该池';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool：此服务器隐瞒了该池的一部分';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool：此服务器报告的该池数据有误';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool：此服务器对该池的服务状态未知';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind => '已与此服务器同步，但该服务器落后于网络';

  @override
  String get walletSyncExplainEndpointBehind =>
      '此服务器的链停留在一个网络早在此版本应用构建之前就已越过的区块，因此您的余额仅更新到该区块。新收到的付款可能尚未显示，从此处发出的付款也可能无法送达。请切换到其他服务器以赶上网络进度——这不是连接问题。';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      '正在等待应用更新 — 您的资金是安全的，尚未发送任何内容。';

  @override
  String get walletParkedBlockedByServerSilent =>
      '正在等待报告网络版本的服务器 — 请切换服务器。您的资金是安全的，尚未发送任何内容。';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      '正在等待报告网络版本的服务器。如果此设备的日期和时间有误，请先更正，然后再切换服务器。您的资金是安全的，尚未发送任何内容。';

  @override
  String get walletSyncUnverified => '已同步，但此服务器未报告网络版本';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other: '发送功能还能使用约 $hours 小时 — 之后请切换服务器。',
      zero: '发送功能还能使用不到 1 小时 — 之后请切换服务器。',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return '发送功能还能使用约 $blocks 个区块 — 之后请切换服务器。';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return '此服务器已有 $blocks 个区块未报告网络版本，因此本应用无法确认发送是否安全。请切换到其他服务器。';
  }

  @override
  String get walletSyncGraceEndedClock =>
      '此服务器已有一天未报告网络版本，因此本应用无法确认发送是否安全。如果此设备的日期和时间有误，请先更正，然后再切换到会报告网络版本的服务器。';

  @override
  String get walletSyncGraceNeverConfirmed =>
      '此服务器从未报告过网络版本，因此本应用无法确认发送是否安全。请切换到其他服务器。';

  @override
  String get walletSyncExplainUnverified =>
      '此服务器不说明自己运行在哪个版本的 Zcash 网络上，因此本应用无法确认它签名的付款会被接受。 您的余额是最新的。 请切换到其他服务器 — 这不是连接问题。';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      '此服务器不说明自己运行在哪个版本的 Zcash 网络上，因此本应用无法确认它签名的付款会被接受。 它还持续提供了此钱包随后不得不撤销的区块，因此您的余额可能不是最新的。 请切换到其他服务器 — 这不是连接问题。';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      '此服务器还在持续提供此钱包随后不得不撤销的区块 — 请切换服务器。';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      '您的余额仍在追赶进度——钱包同步时，可能会有更多余额变为可用。';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC 仍在到账中，钱包追上链上进度后即可使用。';
  }

  @override
  String get walletSendFaultAmountEmpty => '请输入发送金额。';

  @override
  String get walletSendFaultAmountNotANumber => '请以数字形式输入金额，例如 0.25。';

  @override
  String get walletSendFaultAmountDecimals => 'ZEC 最多支持 8 位小数。';

  @override
  String get walletSendFaultAmountNotPositive => '请输入大于零的金额。';

  @override
  String get walletSendFaultAmountOutOfRange => '该金额超过了 ZEC 的总供应量。';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return '此应用目前将单笔发送限制在 $limit ZEC 以内。';
  }

  @override
  String get walletSendFaultAddressInvalid => '这看起来不是此网络下有效的 Zcash 地址。请检查后重试。';

  @override
  String get walletSendFaultMemoToTransparent =>
      '此收款人无法接收备注。请删除备注，或改为发送至屏蔽（隐私）地址。';

  @override
  String get walletSendFaultMemoTooLong => '您的备注过长。请缩短后重试。';

  @override
  String get walletSendFaultMemoNotSendable => '该备注无法发送。请删除后重试。';

  @override
  String get walletSendFaultMemoConflict => '无法发送此笔付款——应用为其附加了两条备注。未发送任何内容。';

  @override
  String get walletSendFaultNetworkMismatch => '该地址属于其他网络。';

  @override
  String get walletSendFaultUriInvalid => '无法构建此付款。请检查地址和金额。';

  @override
  String get walletSendFaultNotSynced => '您的钱包同步进度尚不足够。请等待同步完成，或将此付款加入队列稍后发送。';

  @override
  String get walletSendFaultNotSyncedNoQueue => '您的钱包同步进度尚不足够。请等待同步完成。';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      '您的钱包同步进度尚不足够，且同步目前未在运行。请在钱包主屏幕中查看同步状态。';

  @override
  String get walletSendFaultAmountsExpired => '在您核对期间，金额信息已过期。请重新核对付款。';

  @override
  String get walletSendFaultQueueFull => '等待发送的付款过多。请先让它们发出，然后重试。';

  @override
  String get walletSendFaultWalletBusy => '钱包当前繁忙。请稍后重试。';

  @override
  String get walletSendFaultStorageFull => '可用存储空间不足，无法完成此次发送。请释放一些空间后重试。';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      '当前使用中的一次性地址过多。部分地址可能会随转账确认而释放，但此情况可能不会自行清除。您的资金是安全的。';

  @override
  String get walletSendFaultCouldNotPrepare => '无法准备此付款。请检查详情后重试。';

  @override
  String get walletSendFaultCouldNotPrepareTransient => '暂时无法准备这笔付款。请稍后再试。';

  @override
  String get walletSwapButton => '兑换';

  @override
  String get walletSwapTitle => '兑换 ZEC';

  @override
  String get walletSwapUnavailableWallet => '您的钱包目前尚未就绪。请返回后重试。';

  @override
  String get walletSwapUnavailableOff => '兑换功能目前不可用。';

  @override
  String get walletSwapUnavailableWatchOnly => '这是一个仅观察钱包——它无法兑换。';

  @override
  String get walletSwapDone => '完成';

  @override
  String get walletSwapBackToWallet => '返回钱包';

  @override
  String walletSwapAvailable(String amount) {
    return '可兑换余额：$amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return '可兑换余额：$amount ZEC——您的余额仍在追赶进度';
  }

  @override
  String get walletSwapAssetLabel => '接收资产';

  @override
  String get walletSwapAmountLabel => '兑换金额（ZEC）';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => '目标地址';

  @override
  String get walletSwapDestinationHint => '您在目标链上的接收地址';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return '您的 $chain 接收地址';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return '一个 $chain 地址——您兑换后的资产将发送到此处。请仔细核对链是否正确。';
  }

  @override
  String get walletSwapDestinationScanTooltip => '扫描目标地址 QR 码';

  @override
  String get walletSwapTargetAssetHint => '选择要接收的资产';

  @override
  String get walletSwapQuoteButton => '获取报价';

  @override
  String get walletSwapQuoting => '正在获取报价…';

  @override
  String get walletSwapExecuting => '正在启动兑换…';

  @override
  String get walletSwapExecuteStillWorking => '仍在处理中——兑换正在启动。此过程最多可能需要一分钟。';

  @override
  String get walletSwapReviewTitle => '确认兑换';

  @override
  String get walletSwapYouSendLabel => '您将发送';

  @override
  String get walletSwapYouReceiveLabel => '您至少将收到';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => '网络费用';

  @override
  String get walletSwapNetworkFeeValue => '在发送存款时添加';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return '报价有效期约为 $time——请在过期前确认。';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute => '报价有效期不到一分钟——请在过期前确认。';

  @override
  String get walletSwapQuoteExpired => '此报价已过期。请返回获取新报价——原汇率已不再保证，现在发送可能导致退款。';

  @override
  String get walletCountdownUnderMinute => '不到一分钟';

  @override
  String walletCountdownMinutes(int minutes) {
    return '$minutes分钟';
  }

  @override
  String walletCountdownSeconds(int seconds) {
    return '$seconds秒';
  }

  @override
  String walletCountdownHoursMinutes(int hours, String minutes) {
    return '$hours小时$minutes分钟';
  }

  @override
  String get walletSwapDeshieldTitle => '此次兑换不具备隐私性';

  @override
  String get walletSwapDeshieldBody =>
      '卖出兑换会解除您 ZEC 的屏蔽保护——存款交易是公开的，服务商一侧的交易在其网络上同样公开可见。';

  @override
  String get walletSwapDiscloseTitle => '兑换服务商将看到的信息';

  @override
  String get walletSwapDiscloseAmounts => '双方的金额';

  @override
  String get walletSwapDiscloseCrossLink => '此笔 ZEC 与您接收的资产属于同一笔兑换';

  @override
  String get walletSwapDiscloseDestination => '您的目标地址';

  @override
  String get walletSwapDiscloseSource => '您的来源地址';

  @override
  String get walletSwapDiscloseIp => '您的 IP 地址（除非您通过 Tor 路由连接）';

  @override
  String get walletSwapDiscloseGeneric => '此次兑换的其他详细信息';

  @override
  String get walletSwapDiscloseProviderLegsPublic => '服务商自身的交易在其网络上是公开的';

  @override
  String get walletSwapAckLabel => '我理解服务商将会看到以上信息。';

  @override
  String get walletSwapConfirmButton => '开始兑换';

  @override
  String get walletSwapBackButton => '返回';

  @override
  String get walletSwapStatusPendingTitle => '兑换已开始';

  @override
  String get walletSwapStatusCheckingTitle => '正在检查兑换状态…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      '您的钱包正在将 ZEC 存款发送至服务商。如果您短暂离线，系统会在您重新联网后自动发送——但发送窗口很短，如果窗口在此之前关闭，兑换将直接结束，不会进行任何交换。您的 ZEC 仍然是您的，但可能需要长达一小时才能再次显示为可用余额。';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      '正在等待您的存款到账。如果您尚未从其他钱包发送资金，请在报价过期前完成发送。';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      '此次兑换仍在等待存款到账。存款说明已不再保存在此设备上——如果您已经发送了资金，系统会检测到；如果尚未发送，请让此次兑换过期后重新开始一笔新的兑换。';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return '存款窗口将于 $time 结束。';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      '存款窗口已过期。如果存款未能及时发送，兑换将结束，您的 ZEC 仍会留在钱包中。';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      '存款窗口已过期。如果您尚未发送存款，此次兑换将直接结束——准备好后可以重新获取报价。';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '兑换进行中',
      one: '兑换进行中',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec => '您的 ZEC 正在发送至兑换服务商。';

  @override
  String get walletSwapInFlightRowIntoZec => '正在等待您的存款到达兑换服务商。';

  @override
  String get walletSwapInFlightRowGeneric => '有一笔兑换正在进行中。';

  @override
  String get walletSwapInFlightRowPastWindow => '存款窗口已过期——请查看此次兑换的状态。';

  @override
  String get walletSwapInFlightRowOverdue =>
      '此次兑换尚未在此处得到确认结果——请打开查看。任何退回此钱包的 ZEC 将在同步后显示在您的余额中。';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      '此次兑换尚未在此处得到确认结果——请打开查看。此次兑换送达此钱包的任何 ZEC 将在同步后显示在您的余额中。';

  @override
  String get walletSwapRowOutcomeSuccess => '兑换已完成。';

  @override
  String get walletSwapRowOutcomeRefunded => '兑换已退款。';

  @override
  String get walletSwapRowOutcomeFailed => '兑换未完成。';

  @override
  String get walletSwapRemove => '移除';

  @override
  String get walletSwapRemoveTitle => '从列表中移除此次兑换？';

  @override
  String get walletSwapRemoveBodyInFlight =>
      '此操作仅将该兑换从此列表中移除——不会取消兑换本身，此钱包也将停止追踪其退款。稍后退回的 ZEC 仍属于此钱包；完整重新扫描可以找到它。';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      '此操作仅将该兑换从此列表中移除——不会取消兑换本身，此钱包也将停止追踪即将到账的 ZEC。稍后到账的 ZEC 仍属于此钱包；完整重新扫描可以找到它。如果该兑换改为退款，退款将以您发送的资产原路退回，不会进入此钱包。';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      '此操作仅将该兑换从此列表中移除——不会取消兑换本身，此钱包也将停止追踪仍从其中到账的 ZEC。稍后到账的 ZEC 仍属于此钱包；完整重新扫描可以找到它。';

  @override
  String get walletSwapRemoveBodyDone => '此操作会将已完成的兑换从列表中移除。';

  @override
  String get walletSwapRemoveCancel => '取消';

  @override
  String get walletSwapRemoveConfirm => '移除';

  @override
  String walletSwapInFlightStarted(String time) {
    return '$time 开始';
  }

  @override
  String get walletSwapViewSwap => '显示兑换';

  @override
  String get walletSwapsInFlightError => '暂时无法加载您正在进行中的兑换。';

  @override
  String get walletSwapsInFlightRetry => '重试';

  @override
  String get walletSwapsInFlightRetryInProgress => '正在重试…';

  @override
  String get walletSwapStartAnother => '再兑换一笔';

  @override
  String get walletSwapStatusUnderTitle => '等待存款到齐';

  @override
  String get walletSwapStatusUnderBody => '部分存款已到账。其余部分正在完成，或服务商将进行退款。';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      '您的部分存款已到账。请在截止时间前发送缺少的金额，否则服务商将退回已到账的部分。';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return '已收到 $received；仍缺少 $missing。存款窗口将于 $time 结束。';
  }

  @override
  String get walletSwapStatusDetectedTitle => '已收到存款';

  @override
  String get walletSwapStatusDetectedBody => '服务商已收到您的存款，将处理此次兑换。';

  @override
  String get walletSwapStatusProcessingTitle => '正在处理您的兑换';

  @override
  String get walletSwapStatusProcessingBody => '服务商正在完成您的兑换。';

  @override
  String get walletSwapStatusSuccessTitle => '兑换完成';

  @override
  String get walletSwapStatusSuccessBody => '您的兑换已成功完成。';

  @override
  String get walletSwapStatusRefundedTitle => '兑换已退款';

  @override
  String get walletSwapStatusRefundedBody => '此次兑换未能完成，服务商已将资金退回至您的退款地址。';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      '此次兑换未能完成，服务商已将您的 ZEC 退回此钱包。这笔资金将作为未屏蔽资金到账，并在钱包下次同步后显示在您的余额中——这可能需要一些时间。';

  @override
  String get walletSwapStatusFailedTitle => '兑换失败';

  @override
  String get walletSwapStatusFailedBody => '此次兑换未能完成。已存入的资金将在服务商一侧结算或退回。';

  @override
  String get walletSwapStatusNotFoundTitle => '未找到兑换';

  @override
  String get walletSwapStatusNotFoundBody =>
      '服务商已没有此次兑换的记录——很可能已过期。如果已完成存款，服务商应将其退回至退款地址。该兑换将保留在您的列表中，此钱包会持续追踪它的 ZEC，以防它仍会到账；您可以随时将其从列表中移除。';

  @override
  String get walletSwapStatusUnknownTitle => '状态不可用';

  @override
  String get walletSwapStatusUnknownBody => '目前无法读取此次兑换的状态。';

  @override
  String get walletSwapTrackingUnavailableTitle => '无法追踪';

  @override
  String get walletSwapTrackingUnavailableBody =>
      '兑换功能已关闭，因此无法在此追踪。资金将在服务商一侧结算或退回。';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      '此处的兑换功能已关闭，因此目前无法追踪此次兑换。如果已退款，ZEC 会退回此钱包——在兑换功能重新开启且钱包完成同步后，将显示在您的余额中。';

  @override
  String get walletSwapTrackingError => '无法追踪此次兑换。';

  @override
  String get walletSwapTrackingErrorBody =>
      '无法为此次兑换开启追踪。兑换本身可能仍在进行——已存入的资金将在服务商一侧结算或退回。';

  @override
  String get walletSwapFaultDestinationRequired => '请输入用于接收兑换资产的地址。';

  @override
  String get walletSwapFaultDestinationInvalid => '该目标地址对此资产无效。请核对后重试。';

  @override
  String get walletSwapFaultExpired => '此报价已过期。请获取新报价以继续。';

  @override
  String get walletSwapFaultOutOfBounds =>
      '服务商的价格已超出您设定的限制，因此在资金转移前兑换已被停止。请重试。';

  @override
  String get walletSwapFaultSlippageTooHigh => '滑点限制过高，无法安全完成兑换。请重试。';

  @override
  String get walletSwapFaultProviderUnavailable => '兑换服务商目前不可用。请稍后重试。';

  @override
  String get walletSwapFaultConnection => '无法连接到兑换服务。请检查您的网络连接后重试。';

  @override
  String get walletSwapFaultProviderMisbehaved => '兑换服务商返回了异常响应，兑换已被停止。请重试。';

  @override
  String get walletSwapFaultSwapOff => '兑换功能目前已关闭。';

  @override
  String get walletSwapFaultDepositFailed =>
      '无法发送您的存款，您的钱包中没有任何资金转出。请获取新报价后重试。';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      '已有一笔兑换正在进行中。待其完全结算或报价过期后，您可以开始新的兑换——这可能需要一段时间。';

  @override
  String get walletSwapFaultRefundUnavailable =>
      '此钱包尚无法设置退款地址——这通常只是意味着首次同步尚未完成。请等待同步完成后再重试。';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      '此钱包尚无法为该兑换设置收款地址——这通常只是意味着首次同步尚未完成。请等待同步完成后再重试。';

  @override
  String get walletSwapFaultExecuteTimeout =>
      '兑换未能及时启动——可能是网络连接较慢，也可能是钱包当时正忙。请获取新报价后重试。';

  @override
  String get walletSwapFaultStoreBusyRetry => '钱包暂时繁忙。请重试。';

  @override
  String get walletSwapFaultTermsDiffer =>
      '此报价与您的钱包所签发的报价不一致，因此未发送任何内容。请获取新报价后重试。';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return '此次兑换（含网络费用）大约需要 $needed ZEC，但目前仅有 $spendable ZEC 可用。';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return '此应用目前将单笔兑换限制在 $limit ZEC 以内。';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return '此次兑换（含网络费用）大约需要 $needed ZEC，但目前仅有 $spendable ZEC 可用。您的余额仍在追赶进度——不久后可能会有更多余额可用。';
  }

  @override
  String get walletSwapFaultStateUnavailable => '钱包无法安全记录此次兑换，因此没有资金发生转移。请重试。';

  @override
  String get walletSwapFaultRequestInvalid => '该兑换请求无法处理。请获取新报价后重试。';

  @override
  String get walletSwapFaultCouldNotQuote => '无法获取兑换报价。请核对详细信息后重试。';

  @override
  String get walletSwapFaultWalletUnavailable => '您的钱包目前尚未就绪。请返回后重试。';

  @override
  String get walletSwapDirectionBuy => '买入 ZEC';

  @override
  String get walletSwapDirectionSell => '卖出 ZEC';

  @override
  String get walletSwapRefundLabel => '您的退款地址';

  @override
  String get walletSwapRefundHint => '若兑换失败，您的币将退回至此地址';

  @override
  String get walletSwapRefundHelper => '位于您发送资产所在的链上——而非 Zcash 地址。';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return '您的 $chain 退款地址';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return '一个 $chain 地址——若兑换失败，您的币将退回至此。并非 Zcash 地址。';
  }

  @override
  String get walletSwapRefundInfoTitle => '关于您的退款地址';

  @override
  String get walletSwapRefundInfoBody =>
      '若兑换无法完成，服务商会将您的币退回到您付款所在链上的此地址。请输入一个您自己掌控的地址——钱包无法为您校验外部链地址，因此请仔细核实。';

  @override
  String get walletSwapRefundScanTooltip => '扫描退款地址 QR 码';

  @override
  String get walletSwapScanTitle => '扫描地址';

  @override
  String get walletSwapScanInstruction => '将摄像头对准地址 QR 码。';

  @override
  String get walletSwapScanManualEntry => '手动输入';

  @override
  String get walletSwapScanCancel => '取消';

  @override
  String get walletSwapScanCameraUnavailable => '摄像头不可用。请在下方手动输入地址。';

  @override
  String get walletSwapSourceAssetLabel => '兑换来源资产';

  @override
  String get walletSwapSourceAssetHint => '选择一种资产';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return '发送金额（$symbol）';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => '发送金额';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$chain上的$symbol';
  }

  @override
  String get walletSwapPickerTitle => '选择兑换来源资产';

  @override
  String get walletSwapPickerTitleReceive => '选择要接收的资产';

  @override
  String get walletSwapPickerStale => '无法刷新资产列表——正在显示最近一次已知列表。';

  @override
  String get walletSwapPickerEmpty => '目前没有可兑换的资产。请稍后重试。';

  @override
  String get walletSwapPickerSearchHint => '按名称或链搜索';

  @override
  String walletSwapPickerNoMatch(String query) {
    return '没有资产匹配「$query」。';
  }

  @override
  String get walletSwapPickerError => '无法加载资产列表。请检查网络连接后重试。';

  @override
  String get walletSwapPickerRetry => '重试';

  @override
  String get walletSwapSlippageLabel => '滑点容差';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => '自定义';

  @override
  String get walletSwapSlippageCustomLabel => '自定义滑点';

  @override
  String get walletSwapSlippageMayFail => '过低——若价格波动，兑换可能失败。';

  @override
  String get walletSwapSlippageNormal => '安全的容差范围。';

  @override
  String get walletSwapSlippageRisky => '过高——您可能收到明显低于报价的金额。';

  @override
  String get walletSwapSlippageTooHigh => '过高——兑换将被拒绝。请将其降至10%或更低。';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return '您至少将收到 $zec ZEC——这是您$slippage%滑点下的最低保障。最终金额不会低于此数值。';
  }

  @override
  String get walletSwapIntoZecShieldTitle => '您将收到 ZEC 至您自己的地址';

  @override
  String get walletSwapIntoZecEndsShielded =>
      '在您屏蔽资金前——到账时会提示您一键完成——收到的金额会短暂处于公开状态，可在链上查看。小额到账可能会保持公开，直至累积到一定数量。';

  @override
  String get walletSwapRefundVerifyTitle => '验证您的退款地址';

  @override
  String get walletSwapRefundVerifyBody =>
      '请逐字符核对——若兑换失败，您的币将退回至此地址。钱包无法为您验证外部地址。';

  @override
  String get walletSwapRefundVerifyAck => '我已确认退款地址正确无误。';

  @override
  String get walletSwapPayoutVerifyTitle => '验证您的收款地址';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return '请逐字符核对——您将在此地址收到 $asset。钱包无法为您验证外部地址。';
  }

  @override
  String get walletSwapPayoutVerifyAck => '我已确认收款地址正确无误。';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      '此处的兑换功能已关闭。任何已在途中的 ZEC 将在您下次同步后显示在钱包中。';

  @override
  String get walletSwapFaultForeignAmountRequired => '请输入您要兑换的金额。';

  @override
  String get walletSwapFaultRefundAddressRequired => '请输入您在源链上的退款地址。';

  @override
  String get walletSwapDepositTitle => '发送您的付款';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return '请在$chain上向下方地址发送恰好 $amount $asset。';
  }

  @override
  String get walletSwapDepositExactNote =>
      '请发送准确金额。若发送金额不足，或在窗口关闭后发送，服务商将把资金退回至您的退款地址。';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return '存款窗口：剩余 $time';
  }

  @override
  String get walletSwapDepositExpired =>
      '此存款窗口已关闭。请勿再发送资金——请重新发起兑换。如您已发送，服务商应退款至您的退款地址。';

  @override
  String get walletSwapDepositQrLabel => '存款地址的 QR 码';

  @override
  String get walletSwapDepositAddressLabel => '存款地址';

  @override
  String get walletSwapDepositCopy => '复制存款地址';

  @override
  String get walletSwapDepositCopied => '存款地址已复制';

  @override
  String get walletSwapDepositMemoRequired => '此存款需要备注/标签';

  @override
  String get walletSwapDepositMemoWarning =>
      '您必须在存款时附上此确切备注。若不附加，或备注有误，可能导致您的资金永久丢失。';

  @override
  String get walletSwapDepositMemoLabel => '存款备注/标签';

  @override
  String get walletSwapDepositMemoCopy => '复制备注';

  @override
  String get walletSwapDepositMemoCopied => '备注已复制';

  @override
  String get walletSwapDepositSent => '我已发送资金';

  @override
  String get walletSwapDepositBackTitle => '离开此页面？';

  @override
  String get walletSwapDepositBackBody =>
      '这不会取消您的兑换——它将在后台继续进行。但您需要存款地址才能付款，如尚未复制，请先复制。';

  @override
  String get walletSwapDepositBackBodyExpired =>
      '这不会取消您的兑换——它将在后台继续进行。存款窗口已关闭。请勿再向存款地址发送资金。如您已发送，服务商应退款至您的退款地址。';

  @override
  String get walletSwapDepositBackStay => '留下';

  @override
  String get walletSwapDepositBackLeave => '离开';

  @override
  String get walletReceive => '收款';

  @override
  String get walletReceiveSubtitle => '分享此地址以接收 ZEC。可安全公开分享。';

  @override
  String get walletReceiveCopy => '复制地址';

  @override
  String get walletReceiveCopied => '地址已复制';

  @override
  String get walletReceiveUnavailable => '您的钱包尚未就绪。';

  @override
  String get walletReceiveError => '无法加载您的地址，请重试。';

  @override
  String get walletReceivePreparing => '正在准备您的地址…';

  @override
  String get walletReceivePreparingHint =>
      '您的钱包正在您的设备上准备此地址——如果钱包正忙于其他工作，可能需要一点时间。';

  @override
  String get walletReceiveRetry => '重试';

  @override
  String get walletReceiveQrLabel => '收款地址的 QR 码';

  @override
  String get walletReceiveTypeShielded => '屏蔽';

  @override
  String get walletReceiveTypeTransparent => '公开';

  @override
  String get walletReceiveSubtitleTransparent =>
      '若付款方无法使用屏蔽地址，可分享此公开地址以接收 ZEC。';

  @override
  String get walletReceiveTransparentWarning =>
      '这是一个公开地址：它在链上可见，若重复使用会将您的多笔付款相互关联。请优先使用屏蔽地址；收到资金后请将其屏蔽。';

  @override
  String get walletReceiveQrLabelTransparent => '公开收款地址的 QR 码';

  @override
  String get walletReceiveFreshAddress => '使用新地址';

  @override
  String get walletReceiveFreshCaption =>
      '新地址——无法与您的其他地址关联。付款到此地址仍会进入此钱包，您之前的地址仍可正常使用。此地址不会再次显示——请立即复制。';

  @override
  String get walletReceiveFreshError => '创建新地址失败，请重试。';

  @override
  String get walletReceiveFreshBusy => '钱包目前正忙，请稍后重试新地址。';

  @override
  String get walletReceiveShare => '分享';

  @override
  String get walletReceiveRequestAmount => '请求金额';

  @override
  String get walletReceiveRequestAmountLabel => '金额（可选）';

  @override
  String get walletReceiveFreshCopyNow => '此地址不会再次显示——请立即复制。';

  @override
  String get walletSecurityMenuItem => '安全…';

  @override
  String get securityTitle => '安全';

  @override
  String get securityUnavailableBody => '钱包安全设置由本应用管理，而非钱包本身。';

  @override
  String get securityCustodySectionTitle => '密钥托管';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave（硬件）';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox（硬件）';

  @override
  String get securityCustodyTierTee => '硬件密钥库（TEE）';

  @override
  String get securityCustodyTierSoftware => '软件密钥库';

  @override
  String get securityCustodyTierKeychain => 'Keychain（软件加密）';

  @override
  String get securityCustodyTierNone => '无硬件密钥库';

  @override
  String get securityCustodyTierUnknown => '未知';

  @override
  String get securityCustodyHardwareKey => '锁定此钱包的密钥保存在此设备的安全硬件中，并会随钱包一起删除。';

  @override
  String get securityCustodyBestEffort =>
      '删除操作会尽力清除您的密钥；在设备回收该存储空间之前，仍可能存在短暂的取证可恢复窗口。如需完全确保安全，请同时使用设备的「抹掉所有内容」功能。';

  @override
  String get securityCustodyProbeError => '无法读取托管状态。请返回并重试。';

  @override
  String get securityDeleteWalletButton => '删除钱包';

  @override
  String get securityDeleteWalletSubtitle =>
      '从此设备删除该钱包及其密钥。您的资金仍保留在链上，并可通过恢复短语找回。';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      '从此设备删除该钱包及其密钥。它不持有花费密钥，因此没有需要备份的内容——可随时使用其查看密钥重新添加。';

  @override
  String get securityDeleteDialogTitle => '删除此钱包？';

  @override
  String get securityDeleteDialogBody =>
      '此操作将从此设备移除该钱包及其密钥。请确保您已备份恢复短语——这是找回资金的唯一方式。';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      '此操作将从此设备移除该钱包及其密钥。它不持有花费密钥，因此无需备份任何内容——您可以日后使用其查看密钥重新添加。';

  @override
  String get securityDeleteDialogConfirm => '删除';

  @override
  String get securityDeleteDialogCancel => '取消';

  @override
  String get securityDeleteFailedSnack => '无法删除钱包——您的钱包未发生变化。请重试。';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return '请先完成服务器切换——切换会在 $seconds 秒内完成或停止。然后再次尝试删除钱包。';
  }

  @override
  String get walletParkedTitle => '已保存并待处理';

  @override
  String get walletParkedSubtitle => '这些付款尚未发送。其金额仍计入您的余额。';

  @override
  String get walletParkedSubtitlePreparing =>
      '这些付款尚未发送。其金额仍计入您的余额——但您的钱包正在发送的付款除外，其金额可能已被预留。';

  @override
  String get walletParkedCancel => '取消';

  @override
  String get walletParkedPausedHint => '已暂停——此付款不会自行发送。您的资金是安全的。立即发送，或取消。';

  @override
  String get walletParkedRetryStale => '此付款已不再等待。请查看您的待处理付款和活动记录。';

  @override
  String get walletParkedAlreadyInProgress =>
      '此付款已不再等待——您的钱包可能已经在发送它。请查看「已保存并待处理」和您的活动记录。';

  @override
  String get walletReclaimExplainer =>
      '一次性地址付款已被卡住。您可以重新开启——此操作会在您自己的地址之间转移一小笔资金，随后返还。';

  @override
  String get walletReclaimButton => '重新开启发送';

  @override
  String get walletReclaimInProgress => '正在重新开启…';

  @override
  String get walletReclaimConfirmTitle => '重新开启一次性地址发送？';

  @override
  String get walletReclaimConfirmBody =>
      '此操作会在您自己的地址之间转移一小笔资金，以释放一次性地址发送功能，随后返还。这会产生几笔网络手续费。确认后，请使用「立即恢复」取回这笔转移的资金。';

  @override
  String get walletReclaimConfirmCancel => '暂不';

  @override
  String get walletReclaimConfirmAction => '重新开启';

  @override
  String get walletReclaimStarted => '重新开启已发起。确认后，请发送已暂停的付款，然后使用「立即恢复」取回转移的资金。';

  @override
  String get walletReclaimNothing => '当前没有可重新开启的发送。';

  @override
  String get walletReclaimNotBroadcast => '未能确认是否已送达网络，仍可能已成功。请稍等片刻后重试。';

  @override
  String get walletReclaimNeedsFunds => '您需要一些屏蔽 ZEC 才能重新开启发送。';

  @override
  String get walletReclaimFailed => '暂时无法重新开启发送。您的资金未发生变化。请重试。';

  @override
  String get walletReclaimUnknown =>
      '重新开启已结束。请查看您的一次性地址付款，并使用「立即恢复」取回任何已转移的资金。';

  @override
  String get walletParkedError => '暂时无法加载您的待处理付款。';

  @override
  String get walletParkedErrorRetry => '重试';

  @override
  String get walletParkedErrorRetryInProgress => '正在重试…';

  @override
  String get walletParkedCancelConfirmTitle => '取消此待处理付款？';

  @override
  String get walletParkedCancelConfirmBody =>
      '此操作将丢弃已保存的付款。由于尚未发送，您的钱包不会有任何支出——但此操作无法撤销。';

  @override
  String get walletParkedCancelConfirmKeep => '保留';

  @override
  String get walletParkedCancelConfirmDiscard => '丢弃付款';

  @override
  String get walletParkedCancelDone => '待处理付款已取消。';

  @override
  String get walletParkedCancelAlreadySending => '此付款可能已在途中——请查看您的活动记录。';

  @override
  String get walletParkedCancelFailed => '暂时无法取消。您的付款未发生变化。请重试。';

  @override
  String get walletRecoverNow => '立即恢复';

  @override
  String get walletRecoverConfirmTitle => '恢复至您的屏蔽余额？';

  @override
  String get walletRecoverConfirmBody =>
      '此操作将检查您的一次性地址，并将发现的资金转入您的私密屏蔽余额。可随时安全地再次运行。';

  @override
  String get walletRecoverConfirmCancel => '暂不';

  @override
  String get walletRecoverConfirmAction => '恢复';

  @override
  String get walletRecoverInProgress => '正在恢复…';

  @override
  String walletRecoverDone(String amount) {
    return '正在将 $amount 恢复至您的屏蔽余额。';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return '正在恢复 $amount——仍有部分资金需要再次尝试。';
  }

  @override
  String get walletRecoverRetry => '部分资金需要再次尝试——请再次运行恢复操作。';

  @override
  String get walletRecoverTruncated => '尚未检查所有一次性地址——请再次运行以检查其余部分。';

  @override
  String get walletRecoverNothing => '当前没有可恢复的资金。';

  @override
  String get walletRecoverFailed => '暂时无法恢复。您的资金未发生变化。请重试。';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount 已保存并待处理 · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return '取消已于 $time 保存的 $amount 付款';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount 已暂停 · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount 正在准备发送 · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      '您的钱包正在准备此付款——其金额可能已被预留。您的资金是安全的。如果未完成，它会自行返回列表。';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      '您的钱包正在准备此付款——其金额可能已被预留。您的资金是安全的，但只有在您的钱包再次同步后才能完成。';

  @override
  String get walletParkedSendNow => '立即发送';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return '正在发送已于 $time 保存的 $amount 付款';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return '立即发送已于 $time 保存的 $amount 付款';
  }

  @override
  String get walletParkedSendNowInProgress => '正在发送…';

  @override
  String get walletParkedAuthorizeSent => '正在发送您的付款。';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      '正在发送您的付款。如果未能送达，只有在您的钱包再次同步后才能完成。';

  @override
  String get walletParkedAuthorizeStillWaiting => '尚未准备好发送。您的付款已保存，未发生变化。';

  @override
  String get walletParkedAuthorizeRearmed =>
      '尚未准备好发送。您的付款已保存，且已不再暂停——请稍后再次尝试「立即发送」，或选择取消。';

  @override
  String get walletParkedAuthorizeFailed => '暂时无法发送。您的付款未发生变化。请重试。';

  @override
  String get walletTransparentFundsMenuItem => '公开资金…';

  @override
  String get walletTransparentFundsTitle => '公开资金';

  @override
  String get walletTransparentFundsIntro =>
      '公开资金在区块链上公开可见——金额、地址以及这些资金的历史记录都是如此。';

  @override
  String get walletExpertToggleLabel => '高级：公开资金';

  @override
  String get walletExpertToggleDescription => '显示用于持有公开资金及关闭自动屏蔽的高级控制选项。';

  @override
  String get walletExpertToggleDescriptionNoAutoShield => '显示用于持有公开资金的高级控制选项。';

  @override
  String get walletAutoShieldToggleLabel => '自动屏蔽';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return '当您的公开余额达到 $minZec ZEC 时，资金会自动转入您的屏蔽余额。关闭此选项后，公开资金将仍然公开可见，直至您自行将其屏蔽。';
  }

  @override
  String get walletSettingsSaveFailed => '无法保存此设置。请重试。';

  @override
  String get walletAutoShieldIncomplete => '自动屏蔽未能完成——这些资金仍然公开可见。您现在可以屏蔽这些资金。';

  @override
  String get walletSendPrivacyShielded => '屏蔽付款——金额和收款人在链上保持私密。';

  @override
  String get walletSendPrivacyTransparent => '公开付款——金额和地址在区块链上公开可见。';

  @override
  String get walletActivityPublicBadge => '链上公开可见';

  @override
  String get walletShieldWalletEnded => '钱包会话已结束。请关闭后重新打开以再次尝试。';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return '新到账的公开资金达到 $minZec ZEC 后，会自动屏蔽并转入您的私密屏蔽余额。';
  }

  @override
  String get walletTransparentFundsAutoOff => '自动屏蔽已关闭——公开资金将保持公开可见，直至您将其屏蔽。';

  @override
  String get walletMoveAutoShieldNote =>
      '自动屏蔽已开启：这些资金到账后，将被自动重新屏蔽（需另付费用）。如需保持公开，请先在“公开资金”中关闭自动屏蔽。';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return '此次转移后，您的公开余额将为 $amount ZEC，低于重新屏蔽所需的 $floor ZEC。在有更多资金到账之前，它将保持公开。';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      '您正在转移至自己的公开地址。此次转移将永久保留在公开记录中。';

  @override
  String get walletTxDetailVisibility => '可见性';

  @override
  String get walletTransparentFundsAutoDenied => '自动屏蔽已暂停——本次会话未获批准。您仍可手动屏蔽。';

  @override
  String get walletDeepScanMenuItem => '检查较早的兑换地址…';

  @override
  String get walletMenuSyncNotRunningHint => '同步目前未在运行。';

  @override
  String get walletDeepScanTitle => '检查较早的兑换地址';

  @override
  String get walletDeepScanBody =>
      '如果您还原了此钱包，并且它以前经常使用兑换功能，其最早期兑换的资金可能需要额外的步骤才能找到。此检查会查找这些资金——找到的任何资金将在钱包同步时显示在您的余额中。';

  @override
  String get walletDeepScanCoverage => '您较早的兑换地址已检查至此。如果旧兑换中的资金仍未找到，请检查更早的地址。';

  @override
  String get walletDeepScanCoveragePending =>
      '仍在检查当前范围——找到的任何资金都将显示在您的余额中。这可能需要一些时间。';

  @override
  String get walletDeepScanCoverageUnknown => '检查您钱包最早期兑换中的资金。';

  @override
  String get walletDeepScanCheckButton => '检查较早的地址';

  @override
  String get walletDeepScanCheckDeeperButton => '检查更早的地址';

  @override
  String get walletDeepScanChecking => '正在检查…';

  @override
  String get walletDeepScanClose => '关闭';

  @override
  String get walletDeepScanTorHint =>
      '您目前未通过 Tor 连接。为了获得更高的隐私性，建议在检查前等待 Tor 处于活动状态。';

  @override
  String get walletDeepScanRescanBusy => '重新扫描完成后，您就可以检查较早的兑换地址了。';

  @override
  String get walletDeepScanRan => '正在检查较早的兑换地址——找到的任何资金都将显示在您的余额中。';

  @override
  String get walletDeepScanFailed => '无法开始检查。未发生任何变化——请重试。';

  @override
  String get walletDeepScanSlow =>
      '此操作耗时比平常更长。如果已检查您较早的兑换地址，找到的任何资金都将显示在您的余额中——请稍后再查看。';

  @override
  String get walletDeepScanRefusedDisabled =>
      '兑换功能目前已关闭，因此无法运行此操作。请在兑换功能可用后重试。';

  @override
  String get walletDeepScanRefusedOutstanding =>
      '仍在检查上一个范围——最长可能需要一两天时间，但通常快得多。此过程会自动完成，请稍后再次检查。';

  @override
  String get walletDeepScanTorUnknownHint =>
      '暂时无法确认您连接的隐私状态。为了获得更高的隐私性，建议在 Tor 处于活动状态后再检查。';

  @override
  String get walletDeepScanBannerChecking => '仍在检查较早的兑换地址——找到的任何资金都将显示在您的余额中。';

  @override
  String get walletRescanSwapPointer => '在寻找旧兑换中的资金？重新扫描无法找到它——请改用「检查较早的兑换地址」。';

  @override
  String get walletDeepScanRestoreNoteTitle => '还原了曾使用兑换功能的钱包？';

  @override
  String get walletDeepScanRestoreNoteBody =>
      '如果此钱包的兑换历史很长，其最早期兑换的资金可能需要额外的步骤才能找到。大多数钱包无需任何操作。';

  @override
  String get walletDeepScanRestoreNoteCheck => '立即检查';

  @override
  String get walletDeepScanRestoreNoteDismiss => '关闭';

  @override
  String walletTorHostPath(String transport) {
    return '经由您应用的隐私通道（$transport）';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return '经由您应用的隐私通道（$transport）；代理可以关联各个连接';
  }

  @override
  String get walletTorHostOtherTransport => '隐私通道';

  @override
  String get walletTorHostDirect => '非隐私（您应用的直接连接）';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return '已保存的服务器使用未加密的地址，您应用的隐私通道无法传输。当前使用 $host。';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return '关于$label的更多信息';
  }

  @override
  String get walletSendPaste => '粘贴';

  @override
  String get walletSendScanQr => '扫描二维码';

  @override
  String get walletSendRecipientGetsLabel => '收款方收到';

  @override
  String get walletSwapDepositCopyAmount => '复制金额';

  @override
  String get walletSwapDepositAmountCopied => '已复制金额';

  @override
  String get walletScanOpenSettings => '打开设置';

  @override
  String get walletScanOpenSettingsFailed => '无法打开设置。';

  @override
  String get walletSendLeaveTitle => '仍在发送';

  @override
  String get walletSendLeaveBody => '离开后付款仍会继续。你可以在活动中查看结果。';

  @override
  String get walletSendLeaveStay => '留下';

  @override
  String get walletSendLeaveConfirm => '离开';

  @override
  String get walletSheetLeaveBody => '离开后仍会继续。你可以在活动中查看结果。';

  @override
  String get walletLoadingLabel => '加载中';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => '再次屏蔽前请先查看';

  @override
  String get walletShieldUnknownBody => '我们无法确认此次屏蔽。再次尝试前，请先在“活动”中查看。';

  @override
  String get walletMoveUnknownTitle => '再次转移前请先查看';

  @override
  String get walletMoveUnknownBody => '我们无法确认此次转移。再次尝试前，请先在“活动”中查看。';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
