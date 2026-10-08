// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Portuguese (`pt`).
class WalletLocalizationsPt extends WalletLocalizations {
  WalletLocalizationsPt([String locale = 'pt']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Definições';

  @override
  String get walletTitle => 'Carteira';

  @override
  String get walletNotSetUpTitle => 'Carteira ainda não configurada';

  @override
  String get walletNotSetUpBody =>
      'A configuração da carteira estará disponível numa atualização futura. Vai guiá-lo a anotar a sua frase de recuperação antes de poder receber fundos — para que nada fique em risco sem uma cópia de segurança.';

  @override
  String get walletStartupFailedTitle => 'Não foi possível iniciar a carteira';

  @override
  String get walletStartupFailedBody =>
      'Algo impediu a carteira de carregar neste dispositivo. Se já tem uma carteira, os fundos dela não são afetados — vivem na rede Zcash e podem ser restaurados com a sua frase de recuperação. Tente novamente; se isto continuar a acontecer, feche e volte a abrir a aplicação.';

  @override
  String get walletBalanceLabel => 'Saldo';

  @override
  String get walletHideBalance => 'Ocultar saldo';

  @override
  String get walletShowBalance => 'Mostrar saldo';

  @override
  String get walletBalanceHiddenAmount => 'Saldo oculto';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Disponível agora';

  @override
  String get walletArrivingLabel => 'A caminho';

  @override
  String get walletNotSpendableYetLabel => 'Ainda não pode ser gasto';

  @override
  String get walletActivityTitle => 'Atividade';

  @override
  String get walletActivityEmpty => 'Ainda sem atividade';

  @override
  String get walletActivityError => 'Não foi possível carregar a atividade';

  @override
  String get walletActivityReceived => 'Recebido';

  @override
  String get walletActivitySent => 'Enviado';

  @override
  String get walletActivityPending => 'Pendente';

  @override
  String get walletActivityQueued => 'Em fila';

  @override
  String get walletActivityRetrying => 'A tentar novamente';

  @override
  String get walletActivitySaved => 'Guardada';

  @override
  String get walletActivityExpired => 'Expirado';

  @override
  String get walletActivityFailed => 'Falhou';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count confirmações',
      one: '1 confirmação',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pagamentos recebidos',
      one: 'Pagamento recebido',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Mostrar detalhes da transação';

  @override
  String get walletTxDetailStatus => 'Estado';

  @override
  String get walletTxDetailFee => 'Taxa de rede';

  @override
  String get walletTxDetailDate => 'Data';

  @override
  String get walletTxDetailHeight => 'Altura do bloco';

  @override
  String get walletTxDetailMemo => 'Memo';

  @override
  String get walletTxDetailMemoAttached => 'Incluído';

  @override
  String get walletTxDetailTxid => 'ID da transação';

  @override
  String get walletTxDetailCopyTxid => 'Copiar ID da transação';

  @override
  String get walletTxDetailCopied => 'ID da transação copiado';

  @override
  String get walletTxDetailClose => 'Fechar';

  @override
  String get walletTxFundsKept => 'Nenhum fundo saiu da sua carteira';

  @override
  String get walletTxExplainQueued =>
      'Guardado neste dispositivo, em Guardado e pendente — pode enviá-lo ou cancelá-lo aí.';

  @override
  String get walletTxExplainPending =>
      'Enviado para a rede Zcash — a aguardar confirmação num bloco.';

  @override
  String get walletTxExplainRetrying =>
      'A sua carteira ainda não conseguiu enviar isto para a rede Zcash. Mantém a transação assinada e tenta novamente em cada sincronização até passar ou expirar.';

  @override
  String get walletTxExplainSaved =>
      'A sua carteira guardou esta transação assinada, mas neste momento não a está a enviar por si própria.';

  @override
  String get walletTxExplainConfirmed => 'Confirmado na rede Zcash.';

  @override
  String get walletTxExplainExpired =>
      'Esta transação expirou antes de a rede a confirmar, pelo que foi cancelada. O valor continua disponível para gastar.';

  @override
  String get walletTxExplainFailed =>
      'A rede rejeitou esta transação, pelo que não foi concluída. O valor continua disponível para gastar.';

  @override
  String get walletTxExplainUnknown =>
      'Não é possível determinar o estado atual desta transação. Será atualizado após a próxima sincronização.';

  @override
  String get walletMenuTooltip => 'Mais opções';

  @override
  String get walletRescanMenuItem => 'Analisar novamente o histórico…';

  @override
  String get walletCheckOneTimeMenuItem => 'Verificar endereços de uso único…';

  @override
  String get walletRescanTitle => 'Analisar novamente o seu histórico';

  @override
  String get walletRescanBody =>
      'Faltam fundos mais antigos? Analise novamente o blockchain a partir de uma data mais recuada para recuperar depósitos que uma data de início anterior tenha ignorado. Os seus fundos e a frase de recuperação nunca ficam em risco.';

  @override
  String get walletRescanRangeTitle => 'Até quando recuar na análise';

  @override
  String get walletRescanRangeAll =>
      'Analisar todo o seu histórico — mais lento, mas recupera tudo.';

  @override
  String get walletRescanRangeDefault =>
      'A analisar a partir do início da sua carteira. Ainda faltam fundos mais antigos? Escolha uma data anterior ou analise todo o histórico.';

  @override
  String get walletRescanRangeResolving =>
      'A preparar o intervalo recomendado…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Cerca de $blocks blocos a analisar.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'A analisar a partir de $date. Ainda faltam fundos mais antigos? Escolha uma data anterior ou analise todo o histórico.';
  }

  @override
  String get walletRescanPick => 'Escolher uma data';

  @override
  String get walletRescanChange => 'Alterar data';

  @override
  String get walletRescanScanAll => 'Analisar todo o histórico';

  @override
  String get walletRescanDatePick => 'Data mais antiga a analisar';

  @override
  String get walletRescanWarning =>
      'Isto analisa novamente o blockchain. Datas recentes demoram minutos; recuar muito pode demorar horas. A sincronização decorre em segundo plano — pode continuar a usar a carteira.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Um pagamento desta carteira ainda está a ser confirmado. A carteira normalmente recusa a nova análise até que este seja concluído — pode tentar, mas espere que seja recusado.';

  @override
  String get walletRescanConfirm => 'Iniciar nova análise';

  @override
  String get walletRescanCancel => 'Cancelar';

  @override
  String get walletRescanRunning => 'A reconstruir…';

  @override
  String get walletRescanRebuildingAll =>
      'A reconstruir o seu histórico — a analisar toda a cadeia. O saldo e a atividade vão-se preenchendo à medida que avança.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'A reconstruir o seu histórico a partir de $date — o saldo e a atividade vão-se preenchendo à medida que avança.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'A reconstruir o seu histórico a partir do início da sua carteira — o saldo e a atividade vão-se preenchendo à medida que avança.';

  @override
  String get walletCatchUpBanner =>
      'A atualizar-se — o saldo e a atividade vão-se preenchendo à medida que a carteira sincroniza. Tudo o que recebeu está seguro.';

  @override
  String get walletCatchUpRescanBanner =>
      'A reconstruir o seu histórico após uma nova análise — o saldo e a atividade vão-se preenchendo à medida que avança. Tudo o que recebeu está seguro.';

  @override
  String get walletRescanFailedNotice =>
      'Não foi possível analisar novamente agora — os seus fundos estão seguros, embora o saldo e o histórico possam demorar um pouco a atualizar-se. Tente novamente dentro de momentos.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Um pagamento ainda está a ser confirmado, pelo que a nova análise foi colocada em pausa para proteger os seus fundos. A sua carteira não foi alterada — tente novamente dentro de umas horas e mantenha a aplicação aberta e ligada à internet entretanto.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'A nova análise reconstrói o seu histórico à medida que a carteira sincroniza, e a sincronização não está a decorrer agora. A sua carteira não foi alterada — tente novamente assim que a sincronização estiver a decorrer.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Não há espaço livre suficiente para reconstruir o histórico da sua carteira — os seus fundos estão seguros, embora o saldo e o histórico possam demorar um pouco a atualizar-se. Liberte espaço e tente novamente.';

  @override
  String get walletRescanFailedDismiss => 'Dispensar';

  @override
  String get walletActivityRebuilding => 'A reconstruir o seu histórico…';

  @override
  String get walletActivityCatchingUp =>
      'Ainda a atualizar-se — tudo o que recebeu vai aparecer aqui.';

  @override
  String get walletActivitySyncNotRunning =>
      'O saldo e o histórico terminarão de carregar assim que a sincronização estiver a decorrer.';

  @override
  String get walletActivityLoadMore => 'Carregar mais';

  @override
  String get walletPendingChangeLabel => 'Troco pendente';

  @override
  String get walletTransparentLabel => 'Não protegido (público)';

  @override
  String get walletTransparentNote =>
      'Não incluídos em \"Disponível agora\" — proteja estes fundos para os poder gastar. Até lá, permanecem publicamente visíveis na blockchain.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Estes fundos permanecem publicamente visíveis na blockchain.';

  @override
  String walletPoolShielded(String amount) {
    return 'Protegido $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Público $amount';
  }

  @override
  String get walletPoolAllShielded => 'Tudo protegido · privado';

  @override
  String get walletPoolTapHint => 'Mostrar fundos públicos';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount do seu saldo está num endereço de uso único (recuperável).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount do seu saldo está num endereço de uso único.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Pagamentos no total de $amount estão reservados e ainda a ser concluídos através de endereços de uso único que a sua carteira controla. Não os envie novamente.',
      one:
          '$amount está reservado para um pagamento que a sua carteira ainda está a concluir através de um endereço de uso único que controla. Não o envie novamente.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Pagamentos no total de $amount estão reservados a meio de endereços de uso único que a sua carteira controla. Estão em pausa até a sua carteira voltar a sincronizar. Não os envie novamente.',
      one:
          '$amount está reservado para um pagamento a meio de um endereço de uso único que a sua carteira controla. Está em pausa até a sua carteira voltar a sincronizar. Não o envie novamente.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Não foi possível verificar se um pagamento ainda está a ser concluído. A tentar novamente — até lá, procure um pagamento pendente na sua atividade antes de enviar de novo.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount do seu saldo está num endereço de uso único (ainda a confirmar).';
  }

  @override
  String get walletShieldButton => 'Proteger';

  @override
  String get walletShieldSheetTitle => 'Proteger fundos públicos';

  @override
  String get walletShieldNote =>
      'Isto move fundos do seu saldo público, visível na blockchain, para o seu saldo privado protegido.';

  @override
  String get walletShieldPreparing => 'A preparar…';

  @override
  String get walletShieldAmountLabel => 'A proteger';

  @override
  String get walletShieldFeeLabel => 'Taxa de rede';

  @override
  String get walletShieldNetLabel => 'Fica protegido';

  @override
  String get walletShieldConfirmButton => 'Proteger agora';

  @override
  String get walletShieldSubmitting => 'A proteger…';

  @override
  String get walletShieldNothingTitle => 'Ainda não há nada para proteger';

  @override
  String get walletShieldNothingBody =>
      'Estes fundos estão abaixo do valor que compensa proteger neste momento — a taxa de rede seria superior ao benefício. Poderão ser protegidos assim que chegar um pouco mais.';

  @override
  String get walletShieldDoneTitle => 'Proteção submetida';

  @override
  String get walletShieldDoneBody =>
      'Os seus fundos estão a ser movidos para o saldo protegido. Serão confirmados na blockchain em breve.';

  @override
  String get walletShieldSavedTitle => 'Guardado — vamos terminar a proteção';

  @override
  String get walletShieldSavedBody =>
      'Não foi possível contactar a rede neste momento. A sua proteção está guardada e a sua carteira vai concluí-la numa sincronização posterior. Nada é perdido.';

  @override
  String get walletShieldAlreadyTitle => 'Já submetido';

  @override
  String get walletShieldFailedTitle => 'Não foi possível proteger agora';

  @override
  String get walletShieldStaleBody =>
      'A carteira ainda está a sincronizar. Tente proteger novamente dentro de momentos.';

  @override
  String get walletShieldTransientBody =>
      'Não foi possível preparar a blindagem agora. Tente novamente dentro de instantes.';

  @override
  String get walletShieldStorageFullBody =>
      'Não há espaço livre suficiente para proteger agora. Liberte espaço e tente novamente. Os seus fundos estão seguros.';

  @override
  String get walletShieldClose => 'Fechar';

  @override
  String get walletShieldRetry => 'Tentar novamente';

  @override
  String get walletMoveMenuItem => 'Mover para público…';

  @override
  String get walletMoveSheetTitle => 'Mover para público';

  @override
  String get walletMoveSheetSubtitle =>
      'Envie ZEC protegido para o seu próprio endereço público — útil para uma exchange que não aceite depósitos protegidos.';

  @override
  String get walletMoveDestinationLabel => 'O seu endereço público';

  @override
  String walletMoveAvailable(String amount) {
    return 'Disponível para mover: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Disponível para mover: $amount ZEC — o saldo ainda está a atualizar-se';
  }

  @override
  String get walletMoveDeshieldTitle =>
      'Este movimento torna os seus fundos públicos';

  @override
  String get walletMoveDeshieldBody =>
      'Mover para um endereço público retira estes fundos do seu saldo protegido — o valor e o seu endereço público tornam-se publicamente visíveis na blockchain do Zcash.';

  @override
  String get walletMoveWalletEnded =>
      'A sessão da carteira terminou. Feche e reabra para tentar novamente.';

  @override
  String get walletMoveLoading => 'A preparar…';

  @override
  String get walletMovePreparing => 'A verificar o valor…';

  @override
  String get walletMoveSubmitting => 'A mover…';

  @override
  String get walletMoveReviewButton => 'Rever';

  @override
  String get walletMoveCancel => 'Cancelar';

  @override
  String get walletMoveReviewTitle => 'Rever movimento';

  @override
  String get walletMoveOwnAddressNote =>
      'Está a mover para o seu próprio endereço público. Pode voltar a proteger estes fundos mais tarde, mas este movimento permanece no registo público de forma permanente.';

  @override
  String get walletMoveConfirmButton => 'Mover para público';

  @override
  String get walletMoveBackButton => 'Voltar';

  @override
  String get walletMoveDoneTitle => 'Movido para público';

  @override
  String get walletMoveDoneBody =>
      'Os seus fundos estão a ser movidos para o seu endereço público. Serão confirmados na blockchain em breve.';

  @override
  String get walletMoveSavedTitle => 'Guardado — vamos terminar o movimento';

  @override
  String get walletMoveSavedBody =>
      'Este movimento está guardado e a sua carteira vai enviá-lo numa sincronização posterior. Nada foi perdido.';

  @override
  String get walletMoveAlreadyTitle => 'Já submetido';

  @override
  String get walletMoveAlreadyBody =>
      'Estes fundos já foram submetidos e estão a caminho do seu endereço público.';

  @override
  String get walletMoveFailedTitle =>
      'Não foi possível concluir este movimento';

  @override
  String get walletMoveNothingTitle => 'Ainda não há nada para mover';

  @override
  String get walletMoveNothingBody =>
      'Não tem saldo protegido disponível para mover neste momento. Assim que os fundos forem confirmados, pode movê-los para o seu endereço público.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'A carteira ainda está a atualizar-se — tudo o que recebeu ficará disponível para mover assim que a sincronização terminar.';

  @override
  String get walletMoveCouldNotLoad =>
      'Não foi possível carregar o seu endereço público. Tente novamente.';

  @override
  String get walletMoveRetry => 'Tentar novamente';

  @override
  String get walletMoveClose => 'Fechar';

  @override
  String get walletSnapshotUnavailable =>
      'Não foi possível ler a carteira neste momento. Será atualizada automaticamente.';

  @override
  String get walletBalanceStale =>
      'Não foi possível atualizar — a mostrar o último saldo conhecido.';

  @override
  String get walletSyncStartFailed =>
      'Não foi possível iniciar a sincronização. Vamos continuar a tentar.';

  @override
  String get walletSyncRetry => 'Tentar novamente';

  @override
  String get walletSyncTryNow => 'Tentar agora';

  @override
  String get walletSyncIdle => 'Ainda sem sincronização';

  @override
  String get walletSyncIdleDetail =>
      'A sincronização inicia-se automaticamente.';

  @override
  String get walletSyncDisabled => 'Sincronização desativada';

  @override
  String get walletSyncDisabledDetail =>
      'Ative a sincronização nas definições desta aplicação para atualizar o seu saldo.';

  @override
  String get walletSyncExplainDisabled =>
      'A sincronização está desativada nas definições desta aplicação. Os seus fundos estão seguros. O seu saldo e atividade mostram o último estado sincronizado e não serão atualizados até a sincronização ser ativada.';

  @override
  String get walletParkedSyncPausedNote =>
      'A sua carteira não está a sincronizar, por isso estes não serão enviados por si só. Utilize Enviar agora para enviar um manualmente.';

  @override
  String get walletSyncPausedMoneyNote =>
      'Em pausa até a sua carteira voltar a sincronizar.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'A ligar…';

  @override
  String get walletSyncStartingDetail =>
      'A contactar a rede Zcash e a preparar a análise.';

  @override
  String get walletSyncConnecting => 'A ligar…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'A ligar… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'A analisar $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'A analisar…';

  @override
  String get walletSyncSpendableReady => 'Os fundos estão prontos para gastar.';

  @override
  String get walletSyncCatchingUp =>
      'A atualizar-se com a rede — uma sincronização inicial profunda pode demorar. Pode continuar a usar a aplicação enquanto termina';

  @override
  String walletSyncScanRemaining(String count) {
    return 'Faltam $count blocos';
  }

  @override
  String get walletSyncUpToDate => 'Atualizado';

  @override
  String get walletSyncOffline => 'Offline';

  @override
  String get walletSyncOfflineDetail =>
      'Os envios em fila permanecem guardados em Guardado e pendente.';

  @override
  String get walletSyncUnknown => 'A sincronizar…';

  @override
  String get walletSyncStalled => 'Sincronização em pausa';

  @override
  String get walletStallEndpoint =>
      'Não é possível contactar a rede Zcash neste momento. Vamos continuar a tentar automaticamente — verifique a sua ligação à internet, ou pode ser que o servidor esteja temporariamente indisponível.';

  @override
  String get walletStallTor =>
      'O caminho privado da sua aplicação não está disponível, por isso a carteira não se liga. Verifique as definições de rede da sua aplicação ou desative o caminho privado. A sincronização retoma assim que o caminho voltar.';

  @override
  String get walletStallStorage =>
      'O armazenamento do dispositivo está cheio. Liberte espaço e a sincronização será retomada.';

  @override
  String get walletStallReorg =>
      'A cadeia reorganizou-se; a reverificar os blocos recentes.';

  @override
  String get walletStallInternal =>
      'Um problema local interrompeu a sincronização. Se persistir, restaure a partir da sua frase de recuperação.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Este servidor enviou dados que não podem estar corretos, por isso a sincronização parou. Não é um problema de conexão — mude para outro servidor. Se todos os servidores forem recusados, analise novamente o histórico: a carteira pode estar guardando um registro incorreto de um servidor anterior.';

  @override
  String get walletStallBirthdayInFuture =>
      'Esta carteira está configurada para começar num bloco que este servidor ainda não alcançou. Verifique o bloco inicial configurado nesta carteira ou experimente outro servidor.';

  @override
  String get walletStallStorageUnavailable =>
      'Sincronização em pausa neste dispositivo. A tentar novamente.';

  @override
  String get walletStallUnknown =>
      'A sincronização parou por motivo desconhecido.';

  @override
  String get walletSyncBadgeHint => 'Mostrar detalhes da sincronização';

  @override
  String get walletSyncSheetClose => 'Fechar';

  @override
  String get walletSyncSheetProgress => 'Progresso';

  @override
  String get walletSyncSheetBlocksLeft => 'Blocos em falta';

  @override
  String get walletSyncSheetSyncedTo => 'Sincronizado até ao bloco';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Pelo menos $blocks blocos atrás',
      one: 'Pelo menos 1 bloco atrás',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'A sincronização ainda não começou — inicia-se automaticamente. Não é necessária nenhuma ação.';

  @override
  String get walletSyncExplainStartFailed =>
      'A sincronização não conseguiu iniciar. Os seus fundos estão seguros — a carteira simplesmente não está a verificar nova atividade. Tente novamente abaixo, ou reabra a aplicação.';

  @override
  String get walletSyncExplainStarting =>
      'A carteira está a contactar a rede Zcash e a preparar a análise. Isto demora geralmente alguns segundos.';

  @override
  String get walletSyncExplainConnecting =>
      'A estabelecer ligação à rede Zcash.';

  @override
  String get walletSyncExplainScanning =>
      'A carteira está a verificar os blocos da blockchain à procura dos seus fundos. O saldo e a atividade são atualizados à medida que são encontradas novas transações — pode continuar a usar a aplicação enquanto termina.';

  @override
  String get walletSyncExplainUpToDate =>
      'Totalmente sincronizado com a rede Zcash. O saldo e a atividade estão atualizados.';

  @override
  String get walletSyncExplainStalled =>
      'A sincronização encontrou um problema e está em pausa. Tenta novamente de forma automática.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Não é possível contactar a rede Zcash — isso é normal se estiver offline, ou pode ser que o servidor esteja temporariamente indisponível. Os seus fundos estão seguros: o saldo mostra o último estado sincronizado, e os envios em fila permanecem guardados em Guardado e pendente. A ligação tenta novamente por conta própria.';

  @override
  String get walletSyncExplainOffline =>
      'Sem ligação de rede. Os seus fundos estão seguros — o saldo mostra o último estado sincronizado, e os envios em fila permanecem guardados em Guardado e pendente.';

  @override
  String get walletSyncExplainUnknown =>
      'A carteira está a sincronizar. O saldo e a atividade são atualizados à medida que avança.';

  @override
  String get walletTorOff => 'Tor desativado';

  @override
  String get walletTorBootstrapping => 'A iniciar o caminho privado…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return 'A iniciar $transport…';
  }

  @override
  String get walletTorActive => 'Tor ativo';

  @override
  String get walletTorActiveUnverified => 'Tor ativo (execução não verificada)';

  @override
  String get walletTorActiveUnattested =>
      'Caminho privado em uso (privacidade não verificada)';

  @override
  String get walletTorFellBack => 'Tor indisponível — a usar ligação direta';

  @override
  String get walletTorUnavailable =>
      'Caminho privado indisponível — sem ligação';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport indisponível — sem ligação';
  }

  @override
  String get walletTorUnanswered =>
      'Caminho privado ligado — não vem nada de volta';

  @override
  String get walletTorUnansweredUnattested =>
      'Caminho privado ligado — não vem nada de volta (privacidade não verificada)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport ligado — não vem nada de volta';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Não privado (ligação direta da sua aplicação) — não vem nada de volta';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Ligado através de $transport — não vem nada de volta; o proxy pode associar as ligações';
  }

  @override
  String get walletTorUnknown =>
      'Estado do Tor desconhecido — considerar não protegido';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Saldo (à data do bloco $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Saldo · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Saldo (à data do bloco $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Ligação';

  @override
  String get walletSyncSheetServer => 'Servidor';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Servidor, $host, abre o seletor de servidor';
  }

  @override
  String get walletSyncServerSheetTitle => 'Servidor de sincronização';

  @override
  String get walletSyncServerInUse => 'Em uso';

  @override
  String get walletSyncServerAppDefault => 'Padrão do app';

  @override
  String get walletSyncServerCustom => 'Servidor personalizado…';

  @override
  String get walletSyncServerCustomHint => 'https://host:porta';

  @override
  String get walletSyncServerCheck => 'Verificar servidor';

  @override
  String get walletSyncServerChecking => 'Verificando…';

  @override
  String get walletSyncServerUse => 'Usar este servidor';

  @override
  String get walletSyncServerSwitching => 'Trocando…';

  @override
  String get walletSyncServerContinue => 'Continuar';

  @override
  String get walletSyncServerCancel => 'Cancelar';

  @override
  String get walletSyncServerTrustTitle => 'Confiar neste servidor?';

  @override
  String get walletSyncServerTrustNotice =>
      'Você está confiando neste servidor para informar seu saldo e histórico e retransmitir seus pagamentos. Ele verá seu endereço IP a menos que o Tor esteja ativo, aproximadamente quando sua carteira foi criada, os endereços públicos que sua carteira verifica, as transações que ela consulta e as transações que você envia.';

  @override
  String get walletSyncServerKeyLabel => 'Chave de acesso (opcional)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Cabeçalho da chave';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Digite o cabeçalho que seu servidor espera';

  @override
  String get walletSyncServerKeyInvalid =>
      'Esta chave ou cabeçalho não pode ser usado';

  @override
  String get walletSyncServerKeySaved => 'Chave salva';

  @override
  String get walletSyncServerKeyShow => 'Mostrar';

  @override
  String get walletSyncServerKeyHide => 'Ocultar';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Sua chave identifica você para este servidor. Ela pode vincular seus pagamentos à sua carteira, mesmo pelo Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Trocar reinicia a sincronização em andamento. Seu saldo e histórico permanecem. Os fundos podem aparecer como a caminho até que a varredura do novo servidor se atualize.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Trocar reconecta ao novo servidor. Seu saldo e histórico permanecem.';

  @override
  String get walletSyncServerUnreachable =>
      'Não foi possível alcançar este servidor. Verifique o endereço — e se estiver certo, ou este servidor não está a responder, ou a sua aplicação não consegue chegar-lhe neste momento. Tente novamente ou escolha outro servidor.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Não foi possível alcançar este servidor. A carteira não consegue distinguir se este servidor não está a responder ou se a sua aplicação não consegue chegar-lhe neste momento. Escolha outro servidor ou tente mais tarde.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Este servidor está em outra rede Zcash.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Isso não parece um endereço de servidor. Use https://host:porta.';

  @override
  String get walletSyncServerNotOffered =>
      'Este servidor não é oferecido por este app.';

  @override
  String get walletSyncServerBusy =>
      'A carteira está ocupada agora. Tente novamente em instantes.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'O servidor escolhido não é mais oferecido por este app. Usando $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'A escolha de servidor salva não pôde ser lida. Usando $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Não foi possível trocar — ainda usando $host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'O tráfego da carteira liga-se diretamente ao servidor. O servidor pode ver o seu endereço IP.';

  @override
  String get walletTransportExplainTor =>
      'O tráfego da carteira é encaminhado através da rede Tor, que oculta o seu endereço IP do servidor.';

  @override
  String get walletTransportExplainBootstrapping =>
      'O caminho privado da sua aplicação está a iniciar. O tráfego da carteira aguarda por ele antes de ligar.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport está a iniciar. O tráfego da carteira aguarda por ele antes de ligar.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Não foi possível contactar o Tor, pelo que o tráfego recuou para uma ligação direta. O servidor pode ver o seu endereço IP.';

  @override
  String get walletTransportExplainUnavailable =>
      'O caminho privado da sua aplicação não está disponível, por isso a carteira não se liga. Desative o caminho privado ou verifique as definições de rede da sua aplicação.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport não está disponível, por isso a carteira não se liga. Desative-o ou verifique as definições de rede da sua aplicação.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'O caminho privado aceitou a ligação, mas há um minuto que não vem nada de volta. Pode ser o caminho ou o servidor da carteira — a carteira não consegue distinguir. Continua a tentar; se não resolver, experimente outro servidor ou verifique as definições de rede da sua aplicação.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport aceitou a ligação, mas há um minuto que não vem nada de volta. Pode ser o caminho ou o servidor da carteira — a carteira não consegue distinguir. Continua a tentar; se não resolver, experimente outro servidor ou verifique as definições de rede da sua aplicação.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'O tráfego da carteira liga-se diretamente ao servidor. O servidor pode ver o seu endereço IP. A ligação foi aceite, mas há um minuto que não vem nada de volta. Pode ser o caminho ou o servidor da carteira — a carteira não consegue distinguir. Continua a tentar; se não resolver, experimente outro servidor ou verifique as definições de rede da sua aplicação.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'A privacidade desta ligação não pode ser verificada — considere-a não privada. A ligação foi aceite, mas há um minuto que não vem nada de volta. Pode ser o caminho ou o servidor da carteira — a carteira não consegue distinguir. Continua a tentar; se não resolver, experimente outro servidor ou verifique as definições de rede da sua aplicação.';

  @override
  String get walletTransportExplainUnverified =>
      'A privacidade desta ligação não pode ser verificada — considere-a não privada.';

  @override
  String get walletTransportExplainHostProxy =>
      'O tráfego da carteira é encaminhado através do transporte de privacidade desta aplicação, que oculta o seu endereço IP do servidor.';

  @override
  String get walletOnboardingWelcomeTitle => 'Configure a sua carteira';

  @override
  String get walletOnboardingWelcomeBody =>
      'Crie uma nova carteira para receber e guardar ZEC. Vamos gerar uma frase de recuperação e guiá-lo na sua cópia de segurança antes de poderem chegar quaisquer fundos — para que nada fique em risco sem uma cópia de segurança.';

  @override
  String get walletCreateButton => 'Criar uma nova carteira';

  @override
  String get walletRestoreButton =>
      'Restaurar a partir de uma frase de recuperação';

  @override
  String get walletWatchOnlyButton =>
      'Visualizar uma carteira (apenas visualização)';

  @override
  String get walletWatchOnlyTitle => 'Visualizar uma carteira';

  @override
  String get walletWatchOnlyBody =>
      'Cole uma chave de visualização para visualizar uma carteira sem as respetivas chaves de gasto. Verá o saldo e o histórico, mas não poderá enviar fundos. Escolha a data de início aproximada da carteira, para sabermos até onde recuar.';

  @override
  String get walletWatchOnlyKeyLabel => 'Chave de visualização';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip =>
      'Ler um código QR da chave de visualização';

  @override
  String get walletWatchOnlyScanTitle => 'Ler chave de visualização';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Aponte a câmara para o código QR da chave de visualização.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Câmara indisponível. Cole a chave manualmente em alternativa.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Colar em alternativa';

  @override
  String get walletWatchOnlyScanHint =>
      'Ou toque no botão de leitura para ler um código QR da chave de visualização.';

  @override
  String get walletWatchOnlyScanFilled => 'Chave de visualização lida.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Data de início da carteira';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'A analisar a partir de $date — fundos recebidos antes dessa data não aparecerão. Carteira mais antiga? Escolha uma data anterior.';
  }

  @override
  String get walletWatchOnlyBirthdayPick =>
      'Escolha a data de início da carteira';

  @override
  String get walletWatchOnlyBirthdayChange => 'Alterar data';

  @override
  String get walletWatchOnlySubmit => 'Visualizar esta carteira';

  @override
  String get walletWatchOnlyBack => 'Voltar';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Isso não parece ser uma chave de visualização válida. Verifique-a e tente novamente.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Essa chave de visualização é para uma rede diferente. Não pode ser utilizada aqui.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'Já existe uma carteira neste dispositivo. Volte atrás e abra-a.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Essa data de início é demasiado recente. Escolha uma data anterior.';

  @override
  String get walletRestoreTitle => 'Restaurar a sua carteira';

  @override
  String get walletRestoreBody =>
      'Introduza a sua frase de recuperação para restaurar a carteira — digite ou cole as palavras por ordem, separadas por espaços. Apenas frases padrão: se a sua carteira usou uma frase-chave adicional (uma \"25.ª palavra\"), esta aplicação ainda não a consegue restaurar — verá uma carteira vazia, não um erro.';

  @override
  String get walletRestorePhraseHint =>
      'palavra um  palavra dois  palavra três  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count palavras',
      one: '1 palavra',
      zero: 'Ainda sem palavras',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'as frases de recuperação têm 12, 15, 18, 21 ou 24 palavras';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count palavras não são palavras de recuperação — corrija as destacadas',
      one: '1 palavra não é uma palavra de recuperação — corrija a destacada',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'palavra $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'palavra $index: não é uma palavra de recuperação';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Remover palavra $index';
  }

  @override
  String get walletRestoreSubmit => 'Restaurar carteira';

  @override
  String get walletRestoreBack => 'Voltar';

  @override
  String get walletRestoreBirthdayTitle => 'Até quando recuar na análise';

  @override
  String get walletRestoreBirthdayNone =>
      'Vamos analisar todo o seu histórico — mais lento, mas nada fica de fora.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'A analisar a partir de $date — fundos recebidos antes dessa data não aparecerão. Carteira mais antiga? Escolha uma data anterior ou analise todo o histórico.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Escolher uma data';

  @override
  String get walletRestoreBirthdayChange => 'Alterar data';

  @override
  String get walletRestoreBirthdayClear => 'Analisar todo o histórico';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'A palavra $index não é uma palavra de recuperação. Verifique a sua frase quanto a erros e tente novamente.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Essa frase de recuperação não é válida. Verifique as palavras e a sua ordem e tente novamente.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Essa frase não corresponde à carteira neste dispositivo. Verifique-a novamente e tente outra vez.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'Já existe uma carteira neste dispositivo. Volte atrás para a abrir.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Essa data é demasiado recente. Escolha uma data anterior ou analise tudo.';

  @override
  String get walletGeneratingLabel => 'A criar a sua carteira…';

  @override
  String get walletOpeningLabel => 'A abrir a sua carteira…';

  @override
  String get walletBackupTitle =>
      'Faça uma cópia de segurança da frase de recuperação';

  @override
  String get walletBackupBody =>
      'Estas palavras são a ÚNICA forma de recuperar a sua carteira e fundos. Anote-as por ordem e guarde-as num local seguro e privado. Nunca as partilhe nem as guarde online — qualquer pessoa com estas palavras pode ficar com os seus fundos.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'As capturas de ecrã estão desativadas neste ecrã.';

  @override
  String get walletBackupSecureNoteOther =>
      'Certifique-se de que ninguém consegue ver o seu ecrã.';

  @override
  String get walletBackupReveal => 'Mostrar frase de recuperação';

  @override
  String get walletBackupRevealing => 'A preparar a sua frase de recuperação…';

  @override
  String get walletBackupRevealFailed =>
      'Não foi possível mostrar a sua frase de recuperação agora. Certifique-se de que o dispositivo está desbloqueado e tente novamente.';

  @override
  String get walletBackupRetryReveal => 'Tentar novamente';

  @override
  String get walletBackupReauthFailed =>
      'Não foi possível verificar a sua identidade. Tente novamente.';

  @override
  String get walletBackupConfirmCheckbox =>
      'Anotei a minha frase de recuperação e guardei-a em segurança.';

  @override
  String get walletBackupContinue => 'Continuar';

  @override
  String get walletBackupSaveFailed =>
      'Não foi possível guardar a sua confirmação. Tente novamente.';

  @override
  String get walletBackupStartOver => 'Recomeçar';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Recomeçar sem esta carteira?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Isto elimina esta carteira do dispositivo e devolve-o ao início. Nada pode ser depositado através desta aplicação antes de a configuração estar concluída.\n\nSe esta carteira alguma vez teve fundos — ou foi restaurada a partir de uma frase de recuperação — só essa frase a pode restaurar.';

  @override
  String get walletBackupStartOverConfirm => 'Eliminar e recomeçar';

  @override
  String get walletBackupStartOverKeep => 'Manter esta carteira';

  @override
  String get walletBackupSectionTitle => 'Frase de recuperação';

  @override
  String get walletBackupTileTitle =>
      'Faça uma cópia de segurança da frase de recuperação';

  @override
  String get walletBackupTileSubtitle =>
      'Mostre as palavras que podem recuperar a sua carteira e fundos.';

  @override
  String get walletBackupScreenTitle => 'Frase de recuperação';

  @override
  String get walletBackupDone => 'Concluído';

  @override
  String get walletBackupManagedTitle => 'Sem frase de recuperação própria';

  @override
  String get walletBackupManagedBody =>
      'Esta carteira foi configurada com a conta da aplicação que a instalou, pelo que não tem uma frase de recuperação própria. Os seus fundos são recuperados em conjunto com essa conta — utilize a cópia de segurança dessa conta para os manter protegidos.';

  @override
  String get walletExportViewingKeyTitle => 'Exportar chave de visualização';

  @override
  String get walletExportViewingKeyTileTitle =>
      'Exportar chave de visualização';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Partilhe uma cópia apenas de visualização da sua carteira — esta pode ver o seu histórico, mas não pode gastar os seus fundos.';

  @override
  String get walletExportViewingKeyWarning =>
      'Esta chave permite a quem a possuir ver tudo o que esta carteira já recebeu e enviou — e tudo o que vier a receber e enviar no futuro. Não permite gastar os seus fundos nem recuperar a sua carteira. Partilhe-a apenas com alguém de confiança a quem deseje mostrar o seu histórico completo, como um contabilista ou o seu próprio segundo dispositivo. A única forma de deixar de a partilhar mais tarde é transferir os seus fundos para uma carteira nova.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Esta chave permite a quem a possuir ver tudo o que esta carteira já recebeu e enviou — e tudo o que vier a receber e enviar no futuro. Não permite gastar os seus fundos nem recuperar a sua carteira. Partilhe-a apenas com alguém de confiança a quem deseje mostrar o seu histórico completo, como um contabilista ou o seu próprio segundo dispositivo. Uma vez partilhada, não há forma de deixar de a partilhar.';

  @override
  String get walletExportViewingKeyReveal => 'Mostrar chave de visualização';

  @override
  String get walletExportViewingKeyRetry => 'Tentar novamente';

  @override
  String get walletExportViewingKeyRevealing =>
      'A preparar a sua chave de visualização…';

  @override
  String get walletExportViewingKeyFailed =>
      'Não foi possível mostrar a sua chave de visualização agora. Tente novamente dentro de instantes.';

  @override
  String get walletExportViewingKeyQrLabel =>
      'Código QR da chave de visualização';

  @override
  String get walletExportViewingKeyCopy => 'Copiar chave de visualização';

  @override
  String get walletExportViewingKeyCopied => 'Chave de visualização copiada';

  @override
  String get walletExportViewingKeyDone => 'Concluído';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'As capturas de ecrã estão desativadas neste ecrã.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Certifique-se de que ninguém consegue ver o seu ecrã.';

  @override
  String get walletWatchOnlySectionTitle =>
      'Acerca desta carteira apenas de visualização';

  @override
  String get walletWatchOnlyAboutBody =>
      'Esta é uma carteira apenas de visualização. Foi configurada a partir de uma chave de visualização, pelo que pode ver o seu saldo e histórico, mas não contém quaisquer chaves de gasto — não existe aqui nada que necessite de cópia de segurança, e não pode enviar fundos.';

  @override
  String get walletWatchOnlyBadge => 'Apenas visualização';

  @override
  String get walletOnboardingFailedTitle =>
      'Não foi possível concluir a configuração da carteira';

  @override
  String get walletOnboardingRetry => 'Tentar novamente';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'O armazenamento seguro do seu telefone não está a responder. Desbloqueie o dispositivo e tente novamente. Se isto continuar a acontecer, reinicie o telefone.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Esta carteira está aberta noutra janela ou aplicação, ou ainda está a concluir uma operação anterior. Feche qualquer outra janela que a esteja a usar — ou aguarde alguns instantes — e tente novamente.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'A chave segura desta carteira já não está disponível, pelo que não pode ser aberta neste dispositivo. Os seus fundos estão seguros — restaure a partir da sua frase de recuperação para os recuperar.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Restaurar a partir da frase de recuperação';

  @override
  String get walletOnboardingRecoverConfirmTitle => 'Restaurar esta carteira?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Certifique-se de que tem a sua frase de recuperação antes de continuar — vai precisar dela no ecrã seguinte para recuperar os seus fundos. Os seus fundos estão seguros na blockchain e são controlados por essa frase. Isto remove os dados ilegíveis da carteira deste dispositivo para que possa ser reconstruída.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Cancelar';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Não há espaço livre suficiente para configurar a sua carteira. Liberte espaço e tente novamente.';

  @override
  String get walletOnboardingFailedNoVault =>
      'Este dispositivo não tem um armazenamento seguro de chaves, pelo que a carteira não consegue proteger aqui a sua frase de recuperação.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Não foi possível contactar a rede durante a configuração. Verifique a sua ligação e tente novamente.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'A configuração da carteira não foi concluída. Tente novamente para a concluir — nada foi perdido.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Ocorreu um problema ao configurar a sua carteira. Tente novamente.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'A configuração da carteira desta aplicação está incorreta, pelo que a carteira não consegue iniciar. Tentar novamente não vai ajudar — comunique isto ao programador da aplicação. Os seus fundos não são afetados.';

  @override
  String get walletSendButton => 'Enviar';

  @override
  String get walletSendSyncNotRunning =>
      'A sincronização não está a decorrer — o seu saldo disponível não pode atualizar-se';

  @override
  String get walletSendWaitingForFunds =>
      'Ainda a sincronizar — pode enviar assim que tiver saldo disponível';

  @override
  String get walletSendNoSpendableYet => 'Ainda sem saldo disponível';

  @override
  String get walletSendSyncUnavailable =>
      'Pode enviar assim que a sincronização for retomada';

  @override
  String get walletSendTitle => 'Enviar';

  @override
  String get walletSendUnavailable =>
      'A sua carteira não está pronta neste momento. Volte atrás e tente novamente.';

  @override
  String get walletSendWatchOnly =>
      'Esta é uma carteira apenas de visualização. Pode mostrar saldos e receber pagamentos, mas não possui chaves de gasto — por isso não pode enviar.';

  @override
  String get walletSendExpiredTitle => 'Este pedido de pagamento expirou';

  @override
  String get walletSendExpiredBody =>
      'O ecrã de envio demorou mais de cinco segundos a abrir, por isso a app foi informada de que nada foi enviado. Essa resposta é definitiva: este pedido não pode ser pago a partir daqui. Para pagar, recomece a partir da app.';

  @override
  String get walletSendFaultWatchOnly =>
      'Esta é uma carteira apenas de visualização — não possui chaves de gasto, por isso não pode enviar.';

  @override
  String walletSendAvailable(String amount) {
    return 'Disponível para enviar: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Disponível para enviar: $amount ZEC — o saldo ainda está a atualizar-se';
  }

  @override
  String get walletSendRecipientLabel => 'Endereço do destinatário';

  @override
  String get walletSendRecipientHint => 'Endereço Zcash (começa por u, z ou t)';

  @override
  String get walletSendRecipientLocked =>
      'O destinatário não pode ser alterado aqui';

  @override
  String get walletSendAmountLabel => 'Valor (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Memo (opcional)';

  @override
  String get walletSendMemoHint =>
      'Só é entregue a destinatários protegidos (privados)';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Os memos exigem um destinatário protegido. Este endereço público não pode receber um.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Este pagamento já leva uma referência do app, então não pode levar também uma nota escrita.';

  @override
  String get walletSendMachineMemoTitle => 'O app vai anexar uma referência';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Ele diz que é para: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Ela fica com a transação e não pode ser removida depois. A carteira não consegue verificar o que ela contém.';

  @override
  String get walletSendRecipientShielded => 'Protegido · privado';

  @override
  String get walletSendRecipientTransparent => 'Público';

  @override
  String get walletSendRecipientInvalid =>
      'Isto não parece ser um endereço Zcash válido.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Este endereço é de uma rede Zcash diferente.';

  @override
  String get walletSendReviewButton => 'Rever pagamento';

  @override
  String get walletSendQueueButton => 'Colocar em fila para enviar mais tarde';

  @override
  String get walletSendQueueHint =>
      'Um pagamento em fila espera em Guardado e pendente, onde pode enviá-lo ou cancelá-lo. A taxa de rede é calculada quando for enviado.';

  @override
  String get walletSendPreparing => 'A preparar o seu pagamento…';

  @override
  String get walletSendSubmitting => 'A enviar…';

  @override
  String get walletSendQueuing => 'A colocar em fila…';

  @override
  String get walletSendReviewTitle => 'Confirmar pagamento';

  @override
  String get walletSendTotalLabel => 'Total';

  @override
  String get walletSendFeeLabel => 'Taxa de rede';

  @override
  String get walletSendChangeLabel => 'Troco devolvido';

  @override
  String get walletSendDeshieldTitle => 'Este pagamento não é privado';

  @override
  String get walletSendDeshieldBody =>
      'É enviado para um endereço público, pelo que o valor e o destinatário ficarão publicamente visíveis na blockchain do Zcash.';

  @override
  String get walletSendPublicAckLabel =>
      'Entendo que este pagamento será público.';

  @override
  String get walletSendConfirmButton => 'Enviar agora';

  @override
  String get walletSendBackButton => 'Voltar';

  @override
  String get walletSendSelfSendNote =>
      'Está a enviar para a sua própria carteira. A taxa de rede continua a aplicar-se.';

  @override
  String get walletSendLargeConfirmTitle => 'Enviar um valor elevado?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Isto é quase todo o seu saldo. Um pagamento enviado não pode ser revertido.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Este é um pagamento elevado. Um pagamento enviado não pode ser revertido.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Este é um pagamento elevado — quase todo o seu saldo. Um pagamento enviado não pode ser revertido.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Enviar $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Voltar atrás';

  @override
  String get walletSendSentTitle => 'Pagamento enviado';

  @override
  String get walletSendSentBody =>
      'O seu pagamento foi transmitido para a rede.';

  @override
  String get walletSendSavedTitle => 'Guardado — vamos concluir o envio';

  @override
  String get walletSendSavedBody =>
      'O seu pagamento não pôde ser enviado neste momento, por isso está guardado e a sua carteira vai enviá-lo numa sincronização posterior. Nada é perdido.';

  @override
  String get walletSendKeptTitle => 'Guardada';

  @override
  String get walletSendKeptBody =>
      'A sua carteira guardou esta transação, mas não se comprometeu a enviá-la por si própria. Consulte Atividade para ver em que ponto está.';

  @override
  String get walletSendPartialBody =>
      'Parte do seu pagamento foi enviada; a sua carteira vai concluir o restante numa sincronização posterior. Nada é perdido.';

  @override
  String get walletSendInMotionTitle => 'Pagamento em curso';

  @override
  String get walletSendInMotionBody =>
      'O seu pagamento foi iniciado e está a passar por um endereço de uso único que a sua carteira controla. Não o envie novamente. Se não for concluído, pode recuperar os fundos a partir do ecrã da carteira.';

  @override
  String get walletSendAlreadyTitle => 'Já submetido';

  @override
  String get walletSendAlreadyBody =>
      'Este pagamento já foi submetido — não será enviado duas vezes.';

  @override
  String get walletSendFailedTitle => 'Não foi possível concluir o pagamento';

  @override
  String get walletSendFailedBody =>
      'Ocorreu um problema ao concluir este pagamento e nada foi enviado. Pode tentar novamente.';

  @override
  String get walletSendTryAgain => 'Tentar novamente';

  @override
  String get walletSendDone => 'Concluído';

  @override
  String get walletSendAnother => 'Enviar outro';

  @override
  String get walletSendQueuedTitle => 'Em fila para enviar';

  @override
  String get walletSendQueuedBody =>
      'Este pagamento está guardado. Vai encontrá-lo em Guardado e pendente, onde pode enviá-lo agora ou cancelá-lo.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Saldo disponível insuficiente — tem $available ZEC e isto requer $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'A rede Zcash foi atualizada e este app precisa de uma atualização antes de poder enviar. Seus fundos estão seguros.';

  @override
  String get walletSyncUpToDateLimited =>
      'Em dia até onde esta versão consegue ler';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'A rede Zcash foi atualizada. Esta versão verificou tudo o que consegue ler, mas blocos mais recentes podem conter fundos que ela ainda não consegue mostrar, e os memorandos de pagamentos recentes não estão disponíveis. Atualize o app para ver tudo.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Em dia, mas este servidor não serve todos os pools';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Este servidor recusa, retém ou informa incorretamente um dos pools protegidos da Zcash. Os fundos recebidos nesse pool não podem ser gastos através dele, e o saldo mostrado é um mínimo. Mude para outro servidor para usá-los — não é um problema de conexão.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: este servidor se recusa a servi-lo';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: este servidor está retendo parte dele';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: este servidor está informando dados incorretos sobre ele';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: não se sabe se este servidor o serve';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Em dia com este servidor, mas o servidor está atrasado em relação à rede';

  @override
  String get walletSyncExplainEndpointBehind =>
      'A cadeia deste servidor para num bloco que a rede já tinha ultrapassado antes de esta versão do app ser compilada, por isso o seu saldo só está atualizado até esse bloco. Novos pagamentos recebidos podem ainda não aparecer, e um pagamento enviado daqui pode não chegar. Mude para outro servidor para se atualizar — não é um problema de conexão.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Aguardando uma atualização do app — seus fundos estão seguros e nada foi enviado.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Aguardando um servidor que informe a versão da rede — troque de servidor. Seus fundos estão seguros e nada foi enviado.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Aguardando um servidor que informe a versão da rede. Se a data e a hora deste dispositivo estiverem erradas, corrija-as primeiro — depois troque de servidor. Seus fundos estão seguros e nada foi enviado.';

  @override
  String get walletSyncUnverified =>
      'Atualizado, mas este servidor não informa a versão da rede';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'O envio ainda funciona por cerca de mais $hours horas — depois, troque de servidor.',
      one:
          'O envio ainda funciona por cerca de mais 1 hora — depois, troque de servidor.',
      zero:
          'O envio ainda funciona por menos de uma hora — depois, troque de servidor.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'O envio ainda funciona por cerca de mais $blocks blocos — depois, troque de servidor.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Este servidor não informa a versão da rede há $blocks blocos, então este app não consegue confirmar que é seguro enviar. Troque para outro servidor.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Este servidor não informa a versão da rede há um dia, então este app não consegue confirmar que é seguro enviar. Se a data e a hora deste dispositivo estiverem erradas, corrija-as primeiro — depois troque para um servidor que informe a versão da rede.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Este servidor nunca informou a versão da rede, então este app não consegue confirmar que é seguro enviar. Troque para outro servidor.';

  @override
  String get walletSyncExplainUnverified =>
      'Este servidor não diz em qual versão da rede Zcash ele está, então este app não consegue confirmar que um pagamento assinado por ele será aceito. Seu saldo está atualizado. Troque para outro servidor — isto não é um problema de conexão.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Este servidor não diz em qual versão da rede Zcash ele está, então este app não consegue confirmar que um pagamento assinado por ele será aceito. Ele também continuou servindo blocos que esta carteira depois precisou desfazer, então seu saldo pode não estar atualizado. Troque para outro servidor — isto não é um problema de conexão.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Este servidor também continua servindo blocos que esta carteira depois precisa desfazer — troque de servidor.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'O saldo ainda está a atualizar-se — mais poderá ficar disponível à medida que a carteira sincroniza.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC ainda está a chegar e ficará disponível assim que a carteira recuperar o atraso.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Introduza um valor a enviar.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Introduza o valor como um número, por exemplo 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'O ZEC tem, no máximo, 8 casas decimais.';

  @override
  String get walletSendFaultAmountNotPositive =>
      'Introduza um valor superior a zero.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Esse valor é superior ao fornecimento total de ZEC.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Esta aplicação limita atualmente os envios a $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Isso não parece ser um endereço Zcash válido para esta rede. Verifique-o e tente novamente.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Este destinatário não pode receber um memo. Remova o memo ou envie para um endereço protegido (privado).';

  @override
  String get walletSendFaultMemoTooLong =>
      'O seu memo é demasiado longo. Reduza-o e tente novamente.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Esse memo não pode ser enviado. Remova-o e tente novamente.';

  @override
  String get walletSendFaultMemoConflict =>
      'Não foi possível enviar este pagamento — o app anexou duas notas a ele. Nada foi enviado.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Esse endereço é de uma rede diferente.';

  @override
  String get walletSendFaultUriInvalid =>
      'Não foi possível criar este pagamento. Verifique o endereço e o valor.';

  @override
  String get walletSendFaultNotSynced =>
      'A sua carteira ainda não está suficientemente sincronizada. Aguarde que a sincronização avance ou coloque este envio em fila para mais tarde.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'A sua carteira ainda não está suficientemente sincronizada. Aguarde que a sincronização avance.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'A sua carteira ainda não está suficientemente sincronizada, e a sincronização não está a decorrer agora. Verifique o estado da sincronização no ecrã da carteira.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Os valores expiraram enquanto estava a rever. Reveja o pagamento novamente.';

  @override
  String get walletSendFaultQueueFull =>
      'Há demasiados envios à espera de ser enviados. Deixe-os enviar primeiro e tente novamente.';

  @override
  String get walletSendFaultWalletBusy =>
      'A carteira está ocupada neste momento. Tente novamente dentro de momentos.';

  @override
  String get walletSendFaultStorageFull =>
      'Não há espaço livre suficiente para concluir este envio. Liberte espaço e tente novamente.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Há demasiados endereços de uso único em utilização neste momento. Alguns podem libertar-se à medida que as transferências forem confirmadas, mas isto pode não se resolver sozinho. Os seus fundos estão seguros.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Não foi possível preparar este pagamento. Verifique os detalhes e tente novamente.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Não foi possível preparar este pagamento agora. Tente novamente dentro de instantes.';

  @override
  String get walletSwapButton => 'Trocar';

  @override
  String get walletSwapTitle => 'Trocar ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'A sua carteira não está pronta neste momento. Volte atrás e tente novamente.';

  @override
  String get walletSwapUnavailableOff =>
      'A troca não está disponível neste momento.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Esta é uma carteira apenas de visualização — não pode trocar.';

  @override
  String get walletSwapDone => 'Concluído';

  @override
  String get walletSwapBackToWallet => 'Voltar à carteira';

  @override
  String walletSwapAvailable(String amount) {
    return 'Disponível para trocar: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Disponível para trocar: $amount ZEC — o saldo ainda está a atualizar-se';
  }

  @override
  String get walletSwapAssetLabel => 'Ativo a receber';

  @override
  String get walletSwapAmountLabel => 'Valor a trocar (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Endereço de destino';

  @override
  String get walletSwapDestinationHint =>
      'O seu endereço de receção na cadeia de destino';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'O seu endereço de receção em $chain';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Um endereço $chain — para onde é enviado o ativo trocado. Confirme bem que a cadeia está correta.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Ler um código QR do endereço de destino';

  @override
  String get walletSwapTargetAssetHint => 'Selecione um ativo a receber';

  @override
  String get walletSwapQuoteButton => 'Obter cotação';

  @override
  String get walletSwapQuoting => 'A obter cotação…';

  @override
  String get walletSwapExecuting => 'A iniciar a sua troca…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Ainda em processamento — a troca está a começar. Isto pode demorar até um minuto.';

  @override
  String get walletSwapReviewTitle => 'Confirmar troca';

  @override
  String get walletSwapYouSendLabel => 'Envia';

  @override
  String get walletSwapYouReceiveLabel => 'Recebe, no mínimo,';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Taxa de rede';

  @override
  String get walletSwapNetworkFeeValue =>
      'Adicionada quando o depósito é enviado';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Cotação válida por mais cerca de $time — confirme antes que expire.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Cotação válida por menos de um minuto — confirme antes que expire.';

  @override
  String get walletSwapQuoteExpired =>
      'Esta cotação expirou. Volte atrás e obtenha uma nova — a taxa já não está garantida, e enviar agora arrisca um reembolso.';

  @override
  String get walletCountdownUnderMinute => 'menos de um minuto';

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
  String get walletSwapDeshieldTitle => 'Esta troca não é privada';

  @override
  String get walletSwapDeshieldBody =>
      'Trocar para fora desprotege o seu ZEC — o depósito é uma transação pública, e o lado do fornecedor é público na sua rede.';

  @override
  String get walletSwapDiscloseTitle => 'O que o fornecedor da troca irá ver';

  @override
  String get walletSwapDiscloseAmounts => 'Os valores de ambos os lados';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Que este ZEC e o ativo que recebe são a mesma troca';

  @override
  String get walletSwapDiscloseDestination => 'O seu endereço de destino';

  @override
  String get walletSwapDiscloseSource => 'O seu endereço de origem';

  @override
  String get walletSwapDiscloseIp =>
      'O seu endereço IP (a menos que passe pelo Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'Outros detalhes desta troca';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'As próprias transações do fornecedor são públicas na sua rede';

  @override
  String get walletSwapAckLabel =>
      'Compreendo que o fornecedor irá ver as informações acima.';

  @override
  String get walletSwapConfirmButton => 'Iniciar troca';

  @override
  String get walletSwapBackButton => 'Voltar';

  @override
  String get walletSwapStatusPendingTitle => 'Troca iniciada';

  @override
  String get walletSwapStatusCheckingTitle => 'A verificar o estado da troca…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'A sua carteira está a enviar o depósito de ZEC para o fornecedor. Se estiver momentaneamente offline, é enviado automaticamente assim que voltar a estar online — mas a janela de envio é curta, e se esta se fechar primeiro, a troca simplesmente termina e nada é trocado. O seu ZEC continua a ser seu, mas pode demorar até uma hora a voltar a aparecer como disponível.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'A aguardar a chegada do seu depósito. Se ainda não enviou os fundos da sua outra carteira, envie-os antes que a cotação expire.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Esta troca ainda está a aguardar o depósito. As instruções de depósito já não estão disponíveis neste dispositivo — se já enviou os fundos, estes serão detetados; se não enviou, deixe esta troca expirar e inicie uma nova.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'A janela de depósito termina: $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'A janela de depósito expirou. Se o depósito não foi enviado a tempo, a troca termina e o seu ZEC permanece na sua carteira.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'A janela de depósito expirou. Se ainda não enviou o seu depósito, esta troca simplesmente termina — obtenha uma nova cotação quando estiver pronto.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Trocas em curso',
      one: 'Troca em curso',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'O seu ZEC está a caminho do fornecedor.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'A aguardar que o seu depósito chegue ao fornecedor.';

  @override
  String get walletSwapInFlightRowGeneric => 'Uma troca está em curso.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'A janela de depósito expirou — verifique o estado desta troca.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Esta troca ainda não atingiu um resultado confirmado aqui — abra-a para verificar. Qualquer ZEC devolvido a esta carteira aparece no seu saldo após uma sincronização.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Esta troca ainda não atingiu um resultado confirmado aqui — abra-a para verificar. Qualquer ZEC entregue por esta troca a esta carteira aparece no seu saldo após uma sincronização.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Troca concluída.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Troca reembolsada.';

  @override
  String get walletSwapRowOutcomeFailed => 'Troca não concluída.';

  @override
  String get walletSwapRemove => 'Remover';

  @override
  String get walletSwapRemoveTitle => 'Remover esta troca da lista?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Isto apenas remove a troca desta lista — não cancela a troca, e esta carteira deixará de acompanhar o respetivo reembolso. O ZEC reembolsado mais tarde continua a pertencer a esta carteira; uma nova análise completa pode encontrá-lo.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Isto apenas remove a troca desta lista — não cancela a troca, e esta carteira deixará de acompanhar o ZEC recebido. O ZEC entregue mais tarde continua a pertencer a esta carteira; uma nova análise completa pode encontrá-lo. Se a troca for reembolsada em vez disso, o reembolso regressa no ativo que enviou, fora desta carteira.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Isto apenas remove a troca desta lista — não cancela a troca, e esta carteira deixará de acompanhar o ZEC que ainda esteja a chegar dela. O ZEC que chegue mais tarde continua a pertencer a esta carteira; uma nova análise completa pode encontrá-lo.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Isto remove a troca concluída da lista.';

  @override
  String get walletSwapRemoveCancel => 'Cancelar';

  @override
  String get walletSwapRemoveConfirm => 'Remover';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Iniciada: $time';
  }

  @override
  String get walletSwapViewSwap => 'Mostrar troca';

  @override
  String get walletSwapsInFlightError =>
      'Não foi possível carregar as suas trocas em curso neste momento.';

  @override
  String get walletSwapsInFlightRetry => 'Tentar novamente';

  @override
  String get walletSwapsInFlightRetryInProgress => 'A tentar…';

  @override
  String get walletSwapStartAnother => 'Iniciar outra troca';

  @override
  String get walletSwapStatusUnderTitle => 'A aguardar o depósito completo';

  @override
  String get walletSwapStatusUnderBody =>
      'Parte do depósito já chegou. O restante está a completar-se, ou o fornecedor irá reembolsar.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Parte do seu depósito já chegou. Envie o valor em falta antes do prazo, ou o fornecedor reembolsa o que chegou.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Recebido $received; ainda em falta $missing. A janela de depósito termina: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Depósito recebido';

  @override
  String get walletSwapStatusDetectedBody =>
      'O fornecedor recebeu o seu depósito e irá processar a troca.';

  @override
  String get walletSwapStatusProcessingTitle => 'A processar a sua troca';

  @override
  String get walletSwapStatusProcessingBody =>
      'O fornecedor está a concluir a sua troca.';

  @override
  String get walletSwapStatusSuccessTitle => 'Troca concluída';

  @override
  String get walletSwapStatusSuccessBody =>
      'A sua troca foi concluída com sucesso.';

  @override
  String get walletSwapStatusRefundedTitle => 'Troca reembolsada';

  @override
  String get walletSwapStatusRefundedBody =>
      'A troca não foi concluída, pelo que o fornecedor devolveu os fundos ao seu endereço de reembolso.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'A troca não foi concluída, pelo que o fornecedor devolveu o seu ZEC a esta carteira. Os fundos chegam como não protegidos e aparecem no seu saldo após a próxima sincronização da carteira — o que pode demorar um pouco.';

  @override
  String get walletSwapStatusFailedTitle => 'A troca falhou';

  @override
  String get walletSwapStatusFailedBody =>
      'Não foi possível concluir a troca. Quaisquer fundos depositados são liquidados ou reembolsados do lado do fornecedor.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Troca não encontrada';

  @override
  String get walletSwapStatusNotFoundBody =>
      'O fornecedor já não tem registo desta troca — muito provavelmente expirou. Se foi feito um depósito, o fornecedor deverá reembolsá-lo para o endereço de reembolso. A troca permanece na sua lista e esta carteira continua a acompanhar o ZEC dela caso ainda chegue; pode removê-la da lista a qualquer momento.';

  @override
  String get walletSwapStatusUnknownTitle => 'Estado indisponível';

  @override
  String get walletSwapStatusUnknownBody =>
      'Não é possível ler o estado desta troca neste momento.';

  @override
  String get walletSwapTrackingUnavailableTitle =>
      'Acompanhamento indisponível';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'A troca está desativada, pelo que não é possível acompanhar isto aqui. Quaisquer fundos são liquidados ou reembolsados do lado do fornecedor.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'A troca está desativada aqui, pelo que não é possível acompanhar esta troca neste momento. Se foi reembolsada, o ZEC volta para esta carteira — aparece no seu saldo depois de a troca ser reativada e a carteira sincronizar.';

  @override
  String get walletSwapTrackingError =>
      'Não foi possível acompanhar esta troca.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Não foi possível abrir o acompanhamento desta troca. A troca em si pode continuar em curso — quaisquer fundos depositados são liquidados ou reembolsados do lado do fornecedor.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Introduza o endereço onde pretende receber o ativo trocado.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Esse endereço de destino não é válido para este ativo. Verifique-o e tente novamente.';

  @override
  String get walletSwapFaultExpired =>
      'Esta cotação expirou. Obtenha uma nova cotação para continuar.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'O preço do fornecedor saiu do seu limite, pelo que a troca foi interrompida antes de qualquer movimento. Tente novamente.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'O limite de deslizamento é demasiado elevado para uma troca segura. Tente novamente.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'O fornecedor da troca está indisponível neste momento. Tente novamente dentro de momentos.';

  @override
  String get walletSwapFaultConnection =>
      'Não foi possível contactar o serviço de troca. Verifique a sua ligação à internet e tente novamente.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'O fornecedor da troca devolveu uma resposta inesperada, pelo que a troca foi interrompida. Tente novamente.';

  @override
  String get walletSwapFaultSwapOff => 'A troca está desativada neste momento.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Não foi possível enviar o seu depósito, pelo que nada saiu da sua carteira. Obtenha uma nova cotação para tentar novamente.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Já existe uma troca em curso. Poderá iniciar uma nova depois de esta ser totalmente liquidada ou de a sua cotação expirar — isto pode demorar algum tempo.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Esta carteira ainda não consegue configurar um endereço de reembolso — isto normalmente significa apenas que a primeira sincronização não terminou. Aguarde a conclusão da sincronização e tente novamente.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Esta carteira ainda não consegue configurar um endereço de receção para esta troca — isto normalmente significa apenas que a primeira sincronização não terminou. Aguarde a conclusão da sincronização e tente novamente.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'Não foi possível iniciar a troca a tempo — a ligação pode ter estado lenta, ou a carteira estava ocupada. Obtenha uma nova cotação e tente novamente.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'A carteira está ocupada por instantes. Tente novamente.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Esta cotação não corresponde à que a sua carteira emitiu, por isso nada foi enviado. Obtenha uma nova cotação e tente novamente.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Esta troca precisa de cerca de $needed ZEC, incluindo a taxa de rede, mas apenas $spendable ZEC está disponível neste momento.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Esta aplicação limita atualmente as trocas a $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Esta troca precisa de cerca de $needed ZEC, incluindo a taxa de rede, mas apenas $spendable ZEC está disponível neste momento. O seu saldo ainda está a atualizar-se — mais poderá ficar disponível em breve.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'A carteira não conseguiu registar esta troca com segurança, pelo que nada foi movido. Tente novamente.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Não foi possível processar esse pedido de troca. Obtenha uma nova cotação e tente novamente.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Não foi possível obter uma cotação de troca. Verifique os detalhes e tente novamente.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'A sua carteira não está pronta neste momento. Volte atrás e tente novamente.';

  @override
  String get walletSwapDirectionBuy => 'Comprar ZEC';

  @override
  String get walletSwapDirectionSell => 'Vender ZEC';

  @override
  String get walletSwapRefundLabel => 'O seu endereço de reembolso';

  @override
  String get walletSwapRefundHint =>
      'Para onde as suas moedas voltam se a troca falhar';

  @override
  String get walletSwapRefundHelper =>
      'Na cadeia a partir da qual está a enviar — não é um endereço Zcash.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'O seu endereço de reembolso em $chain';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Um endereço $chain — para onde as suas moedas voltam se a troca falhar. Não é um endereço Zcash.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'Sobre o seu endereço de reembolso';

  @override
  String get walletSwapRefundInfoBody =>
      'Se a troca não puder ser concluída, o fornecedor devolve as suas moedas a este endereço, na cadeia a partir da qual pagou. Introduza um endereço que controle — a carteira não consegue verificar um endereço de outra cadeia por si, por isso confirme-o cuidadosamente.';

  @override
  String get walletSwapRefundScanTooltip =>
      'Ler um código QR do endereço de reembolso';

  @override
  String get walletSwapScanTitle => 'Ler endereço';

  @override
  String get walletSwapScanInstruction =>
      'Aponte a câmara para o código QR do endereço.';

  @override
  String get walletSwapScanManualEntry => 'Introduzir manualmente';

  @override
  String get walletSwapScanCancel => 'Cancelar';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Câmara indisponível. Introduza o endereço manualmente abaixo.';

  @override
  String get walletSwapSourceAssetLabel => 'Ativo de origem da troca';

  @override
  String get walletSwapSourceAssetHint => 'Selecione um ativo';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Valor a enviar ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Valor a enviar';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol em $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Escolha um ativo de origem';

  @override
  String get walletSwapPickerTitleReceive => 'Escolha um ativo a receber';

  @override
  String get walletSwapPickerStale =>
      'Não foi possível atualizar a lista de ativos — a mostrar a última lista conhecida.';

  @override
  String get walletSwapPickerEmpty =>
      'Não há ativos disponíveis para troca neste momento. Tente novamente mais tarde.';

  @override
  String get walletSwapPickerSearchHint => 'Pesquisar por nome ou cadeia';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Nenhum ativo corresponde a \"$query\".';
  }

  @override
  String get walletSwapPickerError =>
      'Não foi possível carregar a lista de ativos. Verifique a sua ligação e tente novamente.';

  @override
  String get walletSwapPickerRetry => 'Tentar novamente';

  @override
  String get walletSwapSlippageLabel => 'Tolerância de deslizamento';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Personalizado';

  @override
  String get walletSwapSlippageCustomLabel => 'Deslizamento personalizado';

  @override
  String get walletSwapSlippageMayFail =>
      'Muito baixo — a troca pode falhar se o preço se mover.';

  @override
  String get walletSwapSlippageNormal => 'Uma tolerância segura.';

  @override
  String get walletSwapSlippageRisky =>
      'Elevado — pode receber um valor claramente inferior ao cotado.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Demasiado elevado — a troca será rejeitada. Reduza para 10% ou menos.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Irá receber, no mínimo, $zec ZEC — o seu limite mínimo com $slippage% de deslizamento. O valor final não descerá abaixo deste.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'Recebe ZEC no seu próprio endereço';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Até o proteger — um toque, sugerido assim que chega — o valor recebido fica brevemente público e visível na blockchain. Uma entrega pequena pode permanecer pública até se acumular.';

  @override
  String get walletSwapRefundVerifyTitle =>
      'Verifique o seu endereço de reembolso';

  @override
  String get walletSwapRefundVerifyBody =>
      'Verifique-o carácter a carácter — é para aqui que as suas moedas voltam se a troca falhar. A carteira não consegue verificar um endereço de outra cadeia por si.';

  @override
  String get walletSwapRefundVerifyAck =>
      'Verifiquei que o meu endereço de reembolso está correto.';

  @override
  String get walletSwapPayoutVerifyTitle =>
      'Verifique o seu endereço de receção';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Verifique-o carácter a carácter — é aqui que vai receber $asset. A carteira não consegue verificar um endereço de outra cadeia por si.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'Verifiquei que o meu endereço de receção está correto.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'A troca está desativada aqui. Qualquer ZEC já a caminho aparecerá na sua carteira após a próxima sincronização.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Introduza o valor que pretende trocar.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Introduza o seu endereço de reembolso na cadeia de origem.';

  @override
  String get walletSwapDepositTitle => 'Envie o seu pagamento';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Envie exatamente $amount $asset em $chain para o endereço abaixo.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Envie o valor exato. Enviar menos, ou enviar depois de a janela se fechar, significa que o fornecedor o reembolsa no seu endereço de reembolso.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Janela de depósito: faltam $time';
  }

  @override
  String get walletSwapDepositExpired =>
      'Esta janela de depósito fechou. Não envie fundos agora — inicie uma nova troca. Se já enviou, o fornecedor deverá reembolsar para o seu endereço de reembolso.';

  @override
  String get walletSwapDepositQrLabel => 'Código QR do endereço de depósito';

  @override
  String get walletSwapDepositAddressLabel => 'Endereço de depósito';

  @override
  String get walletSwapDepositCopy => 'Copiar endereço de depósito';

  @override
  String get walletSwapDepositCopied => 'Endereço de depósito copiado';

  @override
  String get walletSwapDepositMemoRequired =>
      'Este depósito requer um memo / tag';

  @override
  String get walletSwapDepositMemoWarning =>
      'TEM de incluir exatamente este memo com o seu depósito. Enviar sem ele — ou com o memo errado — pode causar a perda permanente dos seus fundos.';

  @override
  String get walletSwapDepositMemoLabel => 'Memo / tag do depósito';

  @override
  String get walletSwapDepositMemoCopy => 'Copiar memo';

  @override
  String get walletSwapDepositMemoCopied => 'Memo copiado';

  @override
  String get walletSwapDepositSent => 'Já enviei os fundos';

  @override
  String get walletSwapDepositBackTitle => 'Sair deste ecrã?';

  @override
  String get walletSwapDepositBackBody =>
      'Isto não cancela a sua troca — ela continua em segundo plano. Mas vai precisar do endereço de depósito para pagar, por isso copie-o primeiro, se ainda não o fez.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Isto não cancela a sua troca — ela continua em segundo plano. A janela de depósito fechou. Não envie fundos para o endereço de depósito agora. Se já enviou, o fornecedor deverá reembolsar para o seu endereço de reembolso.';

  @override
  String get walletSwapDepositBackStay => 'Ficar';

  @override
  String get walletSwapDepositBackLeave => 'Sair';

  @override
  String get walletReceive => 'Receber';

  @override
  String get walletReceiveSubtitle =>
      'Partilhe este endereço para receber ZEC. É seguro partilhá-lo publicamente.';

  @override
  String get walletReceiveCopy => 'Copiar endereço';

  @override
  String get walletReceiveCopied => 'Endereço copiado';

  @override
  String get walletReceiveUnavailable =>
      'A sua carteira ainda não está pronta.';

  @override
  String get walletReceiveError =>
      'Não foi possível carregar o seu endereço. Tente novamente.';

  @override
  String get walletReceivePreparing => 'A preparar o seu endereço…';

  @override
  String get walletReceivePreparingHint =>
      'A sua carteira prepara este endereço no seu dispositivo — isto pode demorar alguns instantes se a carteira estiver ocupada com outras tarefas.';

  @override
  String get walletReceiveRetry => 'Tentar novamente';

  @override
  String get walletReceiveQrLabel => 'Código QR do seu endereço de receção';

  @override
  String get walletReceiveTypeShielded => 'Protegido';

  @override
  String get walletReceiveTypeTransparent => 'Público';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Partilhe este endereço público para receber ZEC de um remetente que não consiga pagar a um endereço protegido.';

  @override
  String get walletReceiveTransparentWarning =>
      'Este é um endereço público: é visível na blockchain e liga os seus pagamentos entre si se for reutilizado. Prefira o seu endereço protegido; proteja estes fundos após os receber.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'Código QR do seu endereço de receção público';

  @override
  String get walletReceiveFreshAddress => 'Usar um novo endereço';

  @override
  String get walletReceiveFreshCaption =>
      'Novo endereço — não pode ser associado aos seus outros endereços. Os pagamentos para ele continuam a chegar a esta carteira, e os seus endereços anteriores continuam a funcionar. Não voltará a ser apresentado aqui — copie-o agora.';

  @override
  String get walletReceiveFreshError =>
      'Não foi possível criar um novo endereço. Tente novamente.';

  @override
  String get walletReceiveFreshBusy =>
      'A carteira está ocupada neste momento. Tente novamente com o novo endereço dentro de instantes.';

  @override
  String get walletReceiveShare => 'Partilhar';

  @override
  String get walletReceiveRequestAmount => 'Pedir valor';

  @override
  String get walletReceiveRequestAmountLabel => 'Valor (opcional)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Não voltará a ser apresentado aqui — copie-o agora.';

  @override
  String get walletSecurityMenuItem => 'Segurança…';

  @override
  String get securityTitle => 'Segurança';

  @override
  String get securityUnavailableBody =>
      'As definições de segurança da carteira são geridas por esta aplicação, e não pela própria carteira.';

  @override
  String get securityCustodySectionTitle => 'Custódia das chaves';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (hardware)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (hardware)';

  @override
  String get securityCustodyTierTee => 'Keystore de hardware (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Keystore de software';

  @override
  String get securityCustodyTierKeychain =>
      'Keychain (encriptado por software)';

  @override
  String get securityCustodyTierNone => 'Sem keystore de hardware';

  @override
  String get securityCustodyTierUnknown => 'Desconhecido';

  @override
  String get securityCustodyHardwareKey =>
      'A chave que bloqueia esta carteira fica no hardware seguro deste dispositivo e é eliminada com a carteira.';

  @override
  String get securityCustodyBestEffort =>
      'Eliminar remove as suas chaves de forma diligente, mas sem garantia absoluta; pode existir uma breve janela de recuperação forense até o dispositivo reutilizar o armazenamento. Para garantia total, utilize também a função de apagar todo o conteúdo do seu dispositivo.';

  @override
  String get securityCustodyProbeError =>
      'Não foi possível ler o estado de custódia. Puxe para atualizar e tente novamente.';

  @override
  String get securityDeleteWalletButton => 'Eliminar carteira';

  @override
  String get securityDeleteWalletSubtitle =>
      'Elimine esta carteira e a respetiva chave deste dispositivo. Os seus fundos permanecem na blockchain e podem ser restaurados a partir da sua frase de recuperação.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Elimine esta carteira e a respetiva chave deste dispositivo. Não contém quaisquer chaves de gasto, pelo que não existe aqui nada que necessite de cópia de segurança — volte a adicioná-la a qualquer momento com a sua chave de visualização.';

  @override
  String get securityDeleteDialogTitle => 'Eliminar esta carteira?';

  @override
  String get securityDeleteDialogBody =>
      'Isto remove a carteira e a respetiva chave deste dispositivo. Certifique-se de que fez uma cópia de segurança da sua frase de recuperação — é a ÚNICA forma de restaurar os seus fundos.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Isto remove a carteira e a respetiva chave deste dispositivo. Não contém quaisquer chaves de gasto, pelo que não existe aqui nada que necessite de cópia de segurança — pode voltar a adicioná-la mais tarde com a sua chave de visualização.';

  @override
  String get securityDeleteDialogConfirm => 'Eliminar';

  @override
  String get securityDeleteDialogCancel => 'Cancelar';

  @override
  String get securityDeleteFailedSnack =>
      'Não foi possível eliminar a carteira — a sua carteira não foi alterada. Tente novamente.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Conclua primeiro a mudança de servidor — ela termina ou para dentro de $seconds segundos. Depois tente eliminar a carteira novamente.';
  }

  @override
  String get walletParkedTitle => 'Guardado e pendente';

  @override
  String get walletParkedSubtitle =>
      'Estes pagamentos ainda não foram enviados. Os seus valores continuam a fazer parte do seu saldo.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Estes pagamentos ainda não foram enviados. Os seus valores continuam a fazer parte do seu saldo — exceto os que a sua carteira está a enviar, cujo valor pode já estar reservado.';

  @override
  String get walletParkedCancel => 'Cancelar';

  @override
  String get walletParkedPausedHint =>
      'Em pausa — este pagamento não será enviado por si só. Os seus fundos estão seguros. Envie-o agora, ou cancele-o.';

  @override
  String get walletParkedRetryStale =>
      'Este pagamento já não está à espera. Verifique os seus pagamentos pendentes e a sua atividade.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Este pagamento já não está à espera — a sua carteira pode já estar a enviá-lo. Verifique Guardado e pendente e a sua atividade.';

  @override
  String get walletReclaimExplainer =>
      'Os envios através de endereços de uso único estão bloqueados. Pode reabri-los — isto move um pequeno valor entre os seus próprios endereços e devolve-o.';

  @override
  String get walletReclaimButton => 'Reabrir envio';

  @override
  String get walletReclaimInProgress => 'A reabrir…';

  @override
  String get walletReclaimConfirmTitle =>
      'Reabrir o envio através de endereços de uso único?';

  @override
  String get walletReclaimConfirmBody =>
      'Isto move um pequeno valor entre os seus próprios endereços para libertar o envio através de endereços de uso único, e depois devolve-o. Custa um par de taxas de rede. Assim que confirmar, recupere o valor movido através de Recuperar agora.';

  @override
  String get walletReclaimConfirmCancel => 'Agora não';

  @override
  String get walletReclaimConfirmAction => 'Reabrir';

  @override
  String get walletReclaimStarted =>
      'Reabertura iniciada. Assim que confirmar, envie o pagamento em pausa e depois recupere o valor movido através de Recuperar agora.';

  @override
  String get walletReclaimNothing => 'Nada a reabrir neste momento.';

  @override
  String get walletReclaimNotBroadcast =>
      'Não foi possível confirmar que chegou à rede. Ainda pode ter sido concluída — aguarde um momento antes de tentar novamente.';

  @override
  String get walletReclaimNeedsFunds =>
      'Precisa de algum ZEC protegido para reabrir o envio.';

  @override
  String get walletReclaimFailed =>
      'Não foi possível reabrir o envio neste momento. Os seus fundos não foram alterados. Tente novamente.';

  @override
  String get walletReclaimUnknown =>
      'Reabertura concluída. Verifique os seus envios através de endereços de uso único e recupere um eventual valor movido através de Recuperar agora.';

  @override
  String get walletParkedError =>
      'Não foi possível carregar os seus pagamentos pendentes neste momento.';

  @override
  String get walletParkedErrorRetry => 'Tentar novamente';

  @override
  String get walletParkedErrorRetryInProgress => 'A tentar…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Cancelar este pagamento pendente?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Isto descarta o pagamento guardado. Ainda não foi enviado, por isso nada sai da sua carteira — mas esta ação não pode ser desfeita.';

  @override
  String get walletParkedCancelConfirmKeep => 'Manter';

  @override
  String get walletParkedCancelConfirmDiscard => 'Descartar pagamento';

  @override
  String get walletParkedCancelDone => 'Pagamento pendente cancelado.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Este pagamento pode já estar a caminho — verifique a sua atividade.';

  @override
  String get walletParkedCancelFailed =>
      'Não foi possível cancelar neste momento. O seu pagamento não foi alterado. Tente novamente.';

  @override
  String get walletRecoverNow => 'Recuperar agora';

  @override
  String get walletRecoverConfirmTitle =>
      'Recuperar para o seu saldo protegido?';

  @override
  String get walletRecoverConfirmBody =>
      'Isto verifica os seus endereços de uso único e move tudo o que for encontrado para o seu saldo privado protegido. É seguro executar novamente a qualquer momento.';

  @override
  String get walletRecoverConfirmCancel => 'Agora não';

  @override
  String get walletRecoverConfirmAction => 'Recuperar';

  @override
  String get walletRecoverInProgress => 'A recuperar…';

  @override
  String walletRecoverDone(String amount) {
    return 'A recuperar $amount para o seu saldo protegido.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'A recuperar $amount — alguns fundos ainda precisam de outra tentativa.';
  }

  @override
  String get walletRecoverRetry =>
      'Alguns fundos precisam de outra tentativa — execute a recuperação novamente.';

  @override
  String get walletRecoverTruncated =>
      'Nem todos os endereços de uso único foram verificados ainda — execute novamente para verificar o restante.';

  @override
  String get walletRecoverNothing => 'Nada a recuperar neste momento.';

  @override
  String get walletRecoverFailed =>
      'Não foi possível recuperar neste momento. Os seus fundos não foram alterados. Tente novamente.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount guardado e pendente · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Cancelar o pagamento de $amount guardado $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount em pausa · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount a preparar o envio · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'A sua carteira está a preparar este pagamento — o valor pode já estar reservado. Os seus fundos estão seguros. Se não terminar, volta à lista por si só.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'A sua carteira está a preparar este pagamento — o valor pode já estar reservado. Os seus fundos estão seguros, mas só poderá terminar quando a sua carteira voltar a sincronizar.';

  @override
  String get walletParkedSendNow => 'Enviar agora';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'A enviar o pagamento de $amount guardado $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Enviar agora o pagamento de $amount guardado $time';
  }

  @override
  String get walletParkedSendNowInProgress => 'A enviar…';

  @override
  String get walletParkedAuthorizeSent => 'A enviar o seu pagamento agora.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'A enviar o seu pagamento agora. Se não for concluído, a sua carteira só poderá terminá-lo quando voltar a sincronizar.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Ainda não está pronto para ser enviado. O seu pagamento está guardado e não foi alterado.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Ainda não está pronto para ser enviado. O seu pagamento está guardado e já não está em pausa — tente Enviar agora novamente mais tarde, ou cancele-o.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Não foi possível enviá-lo neste momento. O pagamento não foi alterado. Tente novamente.';

  @override
  String get walletTransparentFundsMenuItem => 'Fundos públicos…';

  @override
  String get walletTransparentFundsTitle => 'Fundos públicos';

  @override
  String get walletTransparentFundsIntro =>
      'Os fundos públicos são publicamente visíveis na blockchain — o valor, os endereços e o histórico das moedas.';

  @override
  String get walletExpertToggleLabel => 'Avançado: fundos públicos';

  @override
  String get walletExpertToggleDescription =>
      'Mostrar controlos avançados para manter fundos públicos e desativar a proteção automática.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Mostrar controlos avançados para manter fundos públicos.';

  @override
  String get walletAutoShieldToggleLabel => 'Proteger automaticamente';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Quando o seu saldo público atingir $minZec ZEC, é movido automaticamente para o seu saldo protegido. Com isto desativado, os fundos públicos permanecem publicamente visíveis até os proteger manualmente.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Não foi possível guardar a definição. Tente novamente.';

  @override
  String get walletAutoShieldIncomplete =>
      'A proteção automática não foi concluída — estes fundos continuam publicamente visíveis. Pode protegê-los agora.';

  @override
  String get walletSendPrivacyShielded =>
      'Pagamento protegido — o valor e o destinatário permanecem privados na blockchain.';

  @override
  String get walletSendPrivacyTransparent =>
      'Pagamento público — o valor e os endereços são visíveis na blockchain.';

  @override
  String get walletActivityPublicBadge => 'Publicamente visível na blockchain';

  @override
  String get walletShieldWalletEnded =>
      'A sessão da carteira terminou. Feche e reabra para tentar novamente.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Os novos fundos públicos são protegidos automaticamente para o seu saldo privado assim que atingirem $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'A proteção automática está desativada — os fundos públicos permanecem publicamente visíveis até os proteger.';

  @override
  String get walletMoveAutoShieldNote =>
      'A proteção automática está ativada: assim que estes fundos chegarem, serão novamente protegidos automaticamente (com uma nova taxa de rede). Para os manter públicos, desative primeiro a proteção automática em Fundos públicos.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Após este movimento, o seu saldo público será de $amount ZEC — abaixo dos $floor ZEC necessários para o voltar a proteger. Fica público até chegarem mais fundos.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Está a mover para o seu próprio endereço público. Este movimento permanece no registo público de forma permanente.';

  @override
  String get walletTxDetailVisibility => 'Visibilidade';

  @override
  String get walletTransparentFundsAutoDenied =>
      'A proteção automática está pausada nesta sessão — não foi aprovada. Ainda pode proteger manualmente.';

  @override
  String get walletDeepScanMenuItem =>
      'Verificar endereços de troca mais antigos…';

  @override
  String get walletMenuSyncNotRunningHint =>
      'A sincronização não está a decorrer agora.';

  @override
  String get walletDeepScanTitle => 'Verificar endereços de troca mais antigos';

  @override
  String get walletDeepScanBody =>
      'Se restaurou esta carteira e esta utilizava trocas com frequência, os fundos das suas trocas mais antigas podem exigir um passo adicional para serem encontrados. Esta verificação procura-os — tudo o que for encontrado aparece no seu saldo à medida que a carteira sincroniza.';

  @override
  String get walletDeepScanCoverage =>
      'Os seus endereços de troca mais antigos foram verificados até aqui. Se ainda faltam fundos de uma troca antiga, verifique mais atrás.';

  @override
  String get walletDeepScanCoveragePending =>
      'Ainda a verificar o intervalo atual — tudo o que for encontrado aparecerá no seu saldo. Isto pode demorar algum tempo.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Procura fundos das trocas mais antigas da sua carteira.';

  @override
  String get walletDeepScanCheckButton => 'Verificar endereços mais antigos';

  @override
  String get walletDeepScanCheckDeeperButton =>
      'Verificar endereços ainda mais antigos';

  @override
  String get walletDeepScanChecking => 'A verificar…';

  @override
  String get walletDeepScanClose => 'Fechar';

  @override
  String get walletDeepScanTorHint =>
      'De momento, não está ligado através do Tor. Para mais privacidade, considere aguardar até o Tor estar ativo antes de verificar.';

  @override
  String get walletDeepScanRescanBusy =>
      'Poderá verificar endereços de troca mais antigos assim que a nova análise terminar.';

  @override
  String get walletDeepScanRan =>
      'A verificar endereços de troca mais antigos — tudo o que for encontrado aparecerá no seu saldo.';

  @override
  String get walletDeepScanFailed =>
      'Não foi possível iniciar a verificação. Nada foi alterado — tente novamente.';

  @override
  String get walletDeepScanSlow =>
      'Isto está a demorar mais do que o habitual. Se os seus endereços de troca mais antigos foram verificados, tudo o que for encontrado aparecerá no seu saldo — volte a verificar dentro de instantes.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'A troca está desativada neste momento, pelo que isto não pode ser executado. Tente novamente quando a troca estiver disponível.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Ainda a verificar o último intervalo — pode demorar até cerca de dois dias, mas normalmente é bem menos. Termina por si só; verifique novamente mais tarde.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'De momento, não é possível confirmar a privacidade da sua ligação. Para mais privacidade, considere verificar assim que o Tor estiver ativo.';

  @override
  String get walletDeepScanBannerChecking =>
      'Ainda a verificar endereços de troca mais antigos — tudo o que for encontrado aparecerá no seu saldo.';

  @override
  String get walletRescanSwapPointer =>
      'Procura fundos de uma troca antiga? Uma nova análise não os encontrará — utilize antes “Verificar endereços de troca mais antigos”.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Restaurou uma carteira que utilizava trocas?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Se esta carteira teve um histórico de trocas muito longo, os fundos das suas trocas mais antigas podem exigir um passo adicional para serem encontrados. A maioria das carteiras não precisa de nada.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Verificar agora';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Dispensar';

  @override
  String walletTorHostPath(String transport) {
    return 'Através do caminho privado da sua aplicação ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Através do caminho privado da sua aplicação ($transport); o proxy pode associar as ligações';
  }

  @override
  String get walletTorHostOtherTransport => 'um caminho privado';

  @override
  String get walletTorHostDirect =>
      'Não privado (ligação direta da sua aplicação)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'O servidor guardado usa um endereço não cifrado, que o caminho privado da sua aplicação não consegue transportar. A usar $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Mais sobre $label';
  }

  @override
  String get walletSendPaste => 'Colar';

  @override
  String get walletSendScanQr => 'Ler código QR';

  @override
  String get walletSendRecipientGetsLabel => 'O destinatário recebe';

  @override
  String get walletSwapDepositCopyAmount => 'Copiar valor';

  @override
  String get walletSwapDepositAmountCopied => 'Valor copiado';

  @override
  String get walletScanOpenSettings => 'Abrir definições';

  @override
  String get walletScanOpenSettingsFailed =>
      'Não foi possível abrir as definições.';

  @override
  String get walletSendLeaveTitle => 'Ainda a enviar';

  @override
  String get walletSendLeaveBody =>
      'O pagamento continua se sair. Verá como terminou na sua atividade.';

  @override
  String get walletSendLeaveStay => 'Ficar';

  @override
  String get walletSendLeaveConfirm => 'Sair';

  @override
  String get walletSheetLeaveBody =>
      'Isto continua se sair. Verá como terminou na sua atividade.';

  @override
  String get walletLoadingLabel => 'Carregando';

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
      'Verifique antes de proteger novamente';

  @override
  String get walletShieldUnknownBody =>
      'Não foi possível confirmar esta proteção. Consulte Atividade antes de tentar novamente.';

  @override
  String get walletMoveUnknownTitle => 'Verifique antes de mover novamente';

  @override
  String get walletMoveUnknownBody =>
      'Não foi possível confirmar este movimento. Consulte Atividade antes de tentar novamente.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
