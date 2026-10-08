// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Spanish Castilian (`es`).
class WalletLocalizationsEs extends WalletLocalizations {
  WalletLocalizationsEs([String locale = 'es']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Ajustes';

  @override
  String get walletTitle => 'Billetera';

  @override
  String get walletNotSetUpTitle => 'La billetera aún no está configurada';

  @override
  String get walletNotSetUpBody =>
      'La configuración de la billetera estará disponible en una versión posterior. La configuración lo guiará para anotar su frase de recuperación antes de poder recibir fondos, de modo que nada quede en riesgo sin un respaldo.';

  @override
  String get walletStartupFailedTitle => 'La billetera no pudo iniciarse';

  @override
  String get walletStartupFailedBody =>
      'Algo impidió que la billetera se cargara en este dispositivo. Si ya tiene una billetera, sus fondos no se ven afectados: viven en la red de Zcash y pueden restaurarse con su frase de recuperación. Vuelva a intentarlo; si esto sigue ocurriendo, cierre la aplicación y vuelva a abrirla.';

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
  String get walletSpendableLabel => 'Disponible para gastar';

  @override
  String get walletArrivingLabel => 'Entrante';

  @override
  String get walletNotSpendableYetLabel => 'Aún no se puede gastar';

  @override
  String get walletActivityTitle => 'Actividad';

  @override
  String get walletActivityEmpty => 'Aún no hay actividad';

  @override
  String get walletActivityError => 'No se pudo cargar la actividad';

  @override
  String get walletActivityReceived => 'Recibido';

  @override
  String get walletActivitySent => 'Enviado';

  @override
  String get walletActivityPending => 'Pendiente';

  @override
  String get walletActivityQueued => 'En cola';

  @override
  String get walletActivityRetrying => 'Reintentando';

  @override
  String get walletActivitySaved => 'Guardada';

  @override
  String get walletActivityExpired => 'Vencida';

  @override
  String get walletActivityFailed => 'Fallida';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count confirmaciones',
      one: '1 confirmación',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count pagos recibidos',
      one: 'Pago recibido',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Mostrar detalles de la transacción';

  @override
  String get walletTxDetailStatus => 'Estado';

  @override
  String get walletTxDetailFee => 'Comisión de red';

  @override
  String get walletTxDetailDate => 'Fecha';

  @override
  String get walletTxDetailHeight => 'Altura del bloque';

  @override
  String get walletTxDetailMemo => 'Memo';

  @override
  String get walletTxDetailMemoAttached => 'Incluido';

  @override
  String get walletTxDetailTxid => 'ID de transacción';

  @override
  String get walletTxDetailCopyTxid => 'Copiar ID de transacción';

  @override
  String get walletTxDetailCopied => 'ID de transacción copiado';

  @override
  String get walletTxDetailClose => 'Cerrar';

  @override
  String get walletTxFundsKept => 'Ningún fondo salió de su billetera';

  @override
  String get walletTxExplainQueued =>
      'Guardado en este dispositivo, en \"Guardado y pendiente\" — puede enviarlo o cancelarlo allí.';

  @override
  String get walletTxExplainPending =>
      'Enviada a la red de Zcash: a la espera de confirmarse en un bloque.';

  @override
  String get walletTxExplainRetrying =>
      'Su billetera aún no pudo enviar esto a la red de Zcash. Conserva la transacción firmada y vuelve a intentarlo en cada sincronización hasta que salga o expire.';

  @override
  String get walletTxExplainSaved =>
      'Su billetera conserva esta transacción firmada, pero por ahora no la está enviando por sí sola.';

  @override
  String get walletTxExplainConfirmed => 'Confirmada en la red de Zcash.';

  @override
  String get walletTxExplainExpired =>
      'Esta transacción venció antes de que la red la confirmara, por lo que fue cancelada. El monto sigue siendo suyo para gastar.';

  @override
  String get walletTxExplainFailed =>
      'La red rechazó esta transacción, por lo que no se completó. El monto sigue siendo suyo para gastar.';

  @override
  String get walletTxExplainUnknown =>
      'No se puede determinar el estado actual de esta transacción. Se actualizará después de la próxima sincronización.';

  @override
  String get walletMenuTooltip => 'Más opciones';

  @override
  String get walletRescanMenuItem => 'Volver a escanear el historial…';

  @override
  String get walletCheckOneTimeMenuItem =>
      'Verificar direcciones de un solo uso…';

  @override
  String get walletRescanTitle => 'Vuelva a escanear su historial';

  @override
  String get walletRescanBody =>
      '¿Faltan fondos antiguos? Vuelva a escanear la cadena de bloques desde más atrás para recuperar depósitos que una fecha de inicio anterior omitió. Sus fondos y su frase de recuperación nunca corren riesgo.';

  @override
  String get walletRescanRangeTitle => 'Hasta dónde escanear hacia atrás';

  @override
  String get walletRescanRangeAll =>
      'Escanear todo su historial: lo más lento, pero recupera todo.';

  @override
  String get walletRescanRangeDefault =>
      'Escaneando desde el inicio de su billetera. ¿Siguen faltando fondos antiguos? Elija una fecha anterior o escanee todo el historial.';

  @override
  String get walletRescanRangeResolving => 'Preparando el rango recomendado…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Aproximadamente $blocks bloques por escanear.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Escaneando desde el $date en adelante. ¿Siguen faltando fondos antiguos? Elija una fecha anterior o escanee todo el historial.';
  }

  @override
  String get walletRescanPick => 'Elegir una fecha';

  @override
  String get walletRescanChange => 'Cambiar fecha';

  @override
  String get walletRescanScanAll => 'Escanear todo el historial';

  @override
  String get walletRescanDatePick => 'Fecha más antigua para escanear';

  @override
  String get walletRescanWarning =>
      'Esto vuelve a escanear la cadena de bloques. Las fechas recientes tardan minutos; escanear muy atrás puede tardar horas. La sincronización se ejecuta en segundo plano: puede seguir usando su billetera.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Un pago de esta billetera aún se está confirmando. La billetera suele rechazar el reescaneo hasta que se complete — puede intentarlo, pero espere que se rechace.';

  @override
  String get walletRescanConfirm => 'Iniciar reescaneo';

  @override
  String get walletRescanCancel => 'Cancelar';

  @override
  String get walletRescanRunning => 'Reconstruyendo…';

  @override
  String get walletRescanRebuildingAll =>
      'Reconstruyendo su historial: escaneando toda la cadena. Su saldo y actividad se completan a medida que avanza.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Reconstruyendo su historial desde el $date: su saldo y actividad se completan a medida que avanza.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Reconstruyendo su historial desde el inicio de su billetera: su saldo y actividad se completan a medida que avanza.';

  @override
  String get walletCatchUpBanner =>
      'Poniéndose al día: su saldo y actividad se completan a medida que la billetera se sincroniza. Todo lo que haya recibido está seguro.';

  @override
  String get walletCatchUpRescanBanner =>
      'Reconstruyendo su historial tras un reescaneo: su saldo y actividad se completan a medida que avanza. Todo lo que haya recibido está seguro.';

  @override
  String get walletRescanFailedNotice =>
      'No se pudo reescanear en este momento: sus fondos están seguros, aunque su saldo y su historial pueden tardar un poco en ponerse al día. Vuelva a intentarlo en un momento.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Un pago aún se está confirmando, por lo que el reescaneo está en pausa para proteger sus fondos. Su billetera no sufrió cambios: vuelva a intentarlo en un par de horas y mantenga la aplicación abierta y conectada.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'El reescaneo reconstruye su historial a medida que su billetera se sincroniza, y la sincronización no está en marcha en este momento. Su billetera no sufrió cambios: vuelva a intentarlo en cuanto la sincronización esté en marcha.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'No hay suficiente espacio libre para reconstruir el historial de su billetera: sus fondos están seguros, aunque su saldo y su historial pueden tardar un poco en ponerse al día. Libere espacio y vuelva a intentarlo.';

  @override
  String get walletRescanFailedDismiss => 'Descartar';

  @override
  String get walletActivityRebuilding => 'Reconstruyendo su historial…';

  @override
  String get walletActivityCatchingUp =>
      'Todavía poniéndose al día: todo lo que haya recibido aparecerá aquí.';

  @override
  String get walletActivitySyncNotRunning =>
      'Su saldo y su historial terminarán de cargarse en cuanto la sincronización esté en marcha.';

  @override
  String get walletActivityLoadMore => 'Cargar más';

  @override
  String get walletPendingChangeLabel => 'Cambio pendiente';

  @override
  String get walletTransparentLabel => 'Sin blindar (público)';

  @override
  String get walletTransparentNote =>
      'No se incluyen en \"Disponible para gastar\": blinde estos fondos para poder gastarlos. Hasta entonces, siguen siendo públicamente visibles en la cadena.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Estos fondos son públicamente visibles en la cadena.';

  @override
  String walletPoolShielded(String amount) {
    return 'Blindado $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Público $amount';
  }

  @override
  String get walletPoolAllShielded => 'Todo blindado · privado';

  @override
  String get walletPoolTapHint => 'Mostrar fondos públicos';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount de su saldo está en una dirección de un solo uso (recuperable).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount de su saldo está en una dirección de un solo uso.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Pagos por un total de $amount están reservados y todavía se están completando a través de direcciones de un solo uso que su billetera controla. No los envíe de nuevo.',
      one:
          '$amount está reservado para un pago que su billetera todavía está completando a través de una dirección de un solo uso que ella controla. No lo envíe de nuevo.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Pagos por un total de $amount están reservados y van a medio camino a través de direcciones de un solo uso que su billetera controla. Están en pausa hasta que su billetera vuelva a sincronizarse. No los envíe de nuevo.',
      one:
          '$amount está reservado para un pago que va a medio camino a través de una dirección de un solo uso que su billetera controla. Está en pausa hasta que su billetera vuelva a sincronizarse. No lo envíe de nuevo.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'No se pudo comprobar si un pago aún se está completando. Reintentando: mientras tanto, busque un pago pendiente en su actividad antes de volver a enviar.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount de su saldo está en una dirección de un solo uso (aún confirmando).';
  }

  @override
  String get walletShieldButton => 'Blindar';

  @override
  String get walletShieldSheetTitle => 'Blindar fondos públicos';

  @override
  String get walletShieldNote =>
      'Esto mueve fondos de su saldo público, visible en la cadena, a su saldo privado blindado.';

  @override
  String get walletShieldPreparing => 'Preparando…';

  @override
  String get walletShieldAmountLabel => 'Blindando';

  @override
  String get walletShieldFeeLabel => 'Comisión de red';

  @override
  String get walletShieldNetLabel => 'Llega blindado';

  @override
  String get walletShieldConfirmButton => 'Blindar ahora';

  @override
  String get walletShieldSubmitting => 'Blindando…';

  @override
  String get walletShieldNothingTitle => 'Aún no hay nada para blindar';

  @override
  String get walletShieldNothingBody =>
      'Estos fondos están por debajo del monto que vale la pena blindar en este momento: la comisión de red superaría el beneficio. Podrán blindarse cuando llegue un poco más.';

  @override
  String get walletShieldDoneTitle => 'Blindaje enviado';

  @override
  String get walletShieldDoneBody =>
      'Sus fondos se están moviendo a su saldo blindado. Se confirmará en la cadena en breve.';

  @override
  String get walletShieldSavedTitle => 'Guardado: completaremos el blindaje';

  @override
  String get walletShieldSavedBody =>
      'No pudimos conectar con la red en este momento. Su blindaje está guardado y su billetera lo completará en una próxima sincronización. No se pierde nada.';

  @override
  String get walletShieldAlreadyTitle => 'Ya enviado';

  @override
  String get walletShieldFailedTitle => 'No se pudo blindar en este momento';

  @override
  String get walletShieldStaleBody =>
      'La billetera aún se está sincronizando. Intente blindar de nuevo en un momento.';

  @override
  String get walletShieldTransientBody =>
      'No se pudo preparar el blindaje en este momento. Inténtalo de nuevo en un momento.';

  @override
  String get walletShieldStorageFullBody =>
      'No hay suficiente espacio libre para blindar en este momento. Libere espacio y vuelva a intentarlo. Sus fondos están seguros.';

  @override
  String get walletShieldClose => 'Cerrar';

  @override
  String get walletShieldRetry => 'Intentar de nuevo';

  @override
  String get walletMoveMenuItem => 'Mover a público…';

  @override
  String get walletMoveSheetTitle => 'Mover a público';

  @override
  String get walletMoveSheetSubtitle =>
      'Envíe ZEC blindado a su propia dirección pública; útil para un exchange que no acepta depósitos blindados.';

  @override
  String get walletMoveDestinationLabel => 'Su dirección pública';

  @override
  String walletMoveAvailable(String amount) {
    return 'Disponible para mover: $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Disponible para mover: $amount ZEC — su saldo todavía se está poniendo al día';
  }

  @override
  String get walletMoveDeshieldTitle =>
      'Este movimiento hace públicos sus fondos';

  @override
  String get walletMoveDeshieldBody =>
      'Mover a una dirección pública saca estos fondos de su saldo blindado: el monto y su dirección pública quedan públicamente visibles en la cadena de bloques de Zcash.';

  @override
  String get walletMoveWalletEnded =>
      'La sesión de la billetera terminó. Ciérrela y vuelva a abrirla para intentarlo de nuevo.';

  @override
  String get walletMoveLoading => 'Preparando…';

  @override
  String get walletMovePreparing => 'Verificando el monto…';

  @override
  String get walletMoveSubmitting => 'Moviendo…';

  @override
  String get walletMoveReviewButton => 'Revisar';

  @override
  String get walletMoveCancel => 'Cancelar';

  @override
  String get walletMoveReviewTitle => 'Revisar movimiento';

  @override
  String get walletMoveOwnAddressNote =>
      'Está moviendo fondos a su propia dirección pública. Puede volver a blindar estos fondos más adelante, pero este movimiento queda en el registro público de forma permanente.';

  @override
  String get walletMoveConfirmButton => 'Mover a público';

  @override
  String get walletMoveBackButton => 'Atrás';

  @override
  String get walletMoveDoneTitle => 'Movido a público';

  @override
  String get walletMoveDoneBody =>
      'Sus fondos se están moviendo a su dirección pública. Se confirmarán en la cadena en breve.';

  @override
  String get walletMoveSavedTitle => 'Guardado: completaremos el movimiento';

  @override
  String get walletMoveSavedBody =>
      'Este movimiento está guardado y su billetera lo enviará en una próxima sincronización. No se perdió nada.';

  @override
  String get walletMoveAlreadyTitle => 'Ya enviado';

  @override
  String get walletMoveAlreadyBody =>
      'Estos fondos ya fueron enviados y están en camino a su dirección pública.';

  @override
  String get walletMoveFailedTitle => 'No se pudo completar este movimiento';

  @override
  String get walletMoveNothingTitle => 'Aún no hay nada para mover';

  @override
  String get walletMoveNothingBody =>
      'No tiene saldo blindado disponible para mover en este momento. Una vez que los fondos se confirmen, podrá moverlos a su dirección pública.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Su billetera todavía se está poniendo al día — todo lo que haya recibido estará disponible para mover una vez que la sincronización se complete.';

  @override
  String get walletMoveCouldNotLoad =>
      'No se pudo cargar su dirección pública. Intente de nuevo.';

  @override
  String get walletMoveRetry => 'Intentar de nuevo';

  @override
  String get walletMoveClose => 'Cerrar';

  @override
  String get walletSnapshotUnavailable =>
      'No se pudo leer la billetera en este momento. Se actualizará por sí sola.';

  @override
  String get walletBalanceStale =>
      'No se pudo actualizar: se muestra su último saldo conocido.';

  @override
  String get walletSyncStartFailed =>
      'No se pudo iniciar la sincronización. Seguiremos intentándolo.';

  @override
  String get walletSyncRetry => 'Intentar de nuevo';

  @override
  String get walletSyncTryNow => 'Intentar ahora';

  @override
  String get walletSyncIdle => 'Aún no sincronizando';

  @override
  String get walletSyncIdleDetail =>
      'La sincronización comienza automáticamente.';

  @override
  String get walletSyncDisabled => 'Sincronización desactivada';

  @override
  String get walletSyncDisabledDetail =>
      'Activa la sincronización en la configuración de esta app para actualizar tu saldo.';

  @override
  String get walletSyncExplainDisabled =>
      'La sincronización está desactivada en la configuración de esta app. Sus fondos están seguros. Tu saldo y tu actividad muestran el último estado sincronizado y no se actualizarán hasta que se active la sincronización.';

  @override
  String get walletParkedSyncPausedNote =>
      'Su billetera no está sincronizando, así que estos pagos no se enviarán por sí solos. Use \"Enviar ahora\" para enviar uno usted mismo.';

  @override
  String get walletSyncPausedMoneyNote =>
      'En pausa hasta que su billetera vuelva a sincronizarse.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Conectando…';

  @override
  String get walletSyncStartingDetail =>
      'Conectando con la red de Zcash y preparando el escaneo.';

  @override
  String get walletSyncConnecting => 'Conectando…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Conectando… $percent%';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Escaneando $percent%';
  }

  @override
  String get walletSyncScanningEarly => 'Escaneando…';

  @override
  String get walletSyncSpendableReady => 'Los fondos están listos para gastar.';

  @override
  String get walletSyncCatchingUp =>
      'Poniéndose al día con la red: una sincronización inicial profunda puede tardar un tiempo. Puede seguir usando la app mientras termina';

  @override
  String walletSyncScanRemaining(String count) {
    return '$count bloques restantes';
  }

  @override
  String get walletSyncUpToDate => 'Actualizado';

  @override
  String get walletSyncOffline => 'Sin conexión';

  @override
  String get walletSyncOfflineDetail =>
      'Los envíos en cola permanecen guardados en \"Guardado y pendiente\".';

  @override
  String get walletSyncUnknown => 'Sincronizando…';

  @override
  String get walletSyncStalled => 'Sincronización en pausa';

  @override
  String get walletStallEndpoint =>
      'No se puede conectar con la red de Zcash en este momento. Seguiremos intentándolo automáticamente: verifique su conexión a internet, o es posible que el servidor no esté disponible temporalmente.';

  @override
  String get walletStallTor =>
      'La ruta privada de su app no está disponible, así que la billetera no se conecta. Revise la configuración de red de su app o desactive la ruta privada. La sincronización se reanudará en cuanto vuelva la ruta.';

  @override
  String get walletStallStorage =>
      'El almacenamiento del dispositivo está lleno. Libere espacio y la sincronización se reanudará.';

  @override
  String get walletStallReorg =>
      'La cadena se reorganizó; verificando de nuevo los bloques recientes.';

  @override
  String get walletStallInternal =>
      'Un problema local detuvo la sincronización. Si continúa ocurriendo, restaure desde su frase de recuperación.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Este servidor envió datos que no pueden ser correctos, así que la sincronización se detuvo. No es un problema de conexión: cambie a otro servidor. Si todos los servidores son rechazados, vuelva a escanear el historial: la billetera puede conservar un registro erróneo de un servidor anterior.';

  @override
  String get walletStallBirthdayInFuture =>
      'Esta billetera está configurada para empezar en un bloque que este servidor aún no ha alcanzado. Compruebe el bloque inicial configurado en esta billetera o pruebe con otro servidor.';

  @override
  String get walletStallStorageUnavailable =>
      'Sincronización en pausa en este dispositivo. Reintentando.';

  @override
  String get walletStallUnknown =>
      'La sincronización se detuvo por un motivo desconocido.';

  @override
  String get walletSyncBadgeHint => 'Mostrar detalles de sincronización';

  @override
  String get walletSyncSheetClose => 'Cerrar';

  @override
  String get walletSyncSheetProgress => 'Progreso';

  @override
  String get walletSyncSheetBlocksLeft => 'Bloques restantes';

  @override
  String get walletSyncSheetSyncedTo => 'Sincronizado hasta el bloque';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Al menos $blocks bloques por detrás',
      one: 'Al menos 1 bloque por detrás',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'La sincronización aún no comenzó; comienza automáticamente. No se necesita ninguna acción.';

  @override
  String get walletSyncExplainStartFailed =>
      'La sincronización no pudo iniciarse. Sus fondos están seguros: la billetera simplemente no está comprobando si hay actividad nueva. Vuelva a intentarlo abajo, o vuelva a abrir la aplicación.';

  @override
  String get walletSyncExplainStarting =>
      'La billetera está contactando la red de Zcash y preparando el escaneo. Esto suele tardar unos segundos.';

  @override
  String get walletSyncExplainConnecting =>
      'Estableciendo conexión con la red de Zcash.';

  @override
  String get walletSyncExplainScanning =>
      'La billetera está revisando los bloques de la cadena en busca de sus fondos. Su saldo y actividad se actualizan a medida que se encuentran nuevas transacciones; puede seguir usando la app mientras termina.';

  @override
  String get walletSyncExplainUpToDate =>
      'Totalmente sincronizada con la red de Zcash. Su saldo y actividad están al día.';

  @override
  String get walletSyncExplainStalled =>
      'La sincronización encontró un problema y está en pausa. Se reintenta automáticamente.';

  @override
  String get walletSyncExplainStalledOffline =>
      'No se puede conectar con la red de Zcash — es normal si está sin conexión, o es posible que el servidor no esté disponible temporalmente. Sus fondos están seguros: el saldo muestra el último estado sincronizado, y los envíos en cola permanecen guardados en \"Guardado y pendiente\". La conexión vuelve a intentarlo por sí sola.';

  @override
  String get walletSyncExplainOffline =>
      'No hay conexión de red. Sus fondos están seguros: el saldo muestra el último estado sincronizado, y los envíos en cola permanecen guardados en \"Guardado y pendiente\".';

  @override
  String get walletSyncExplainUnknown =>
      'La billetera se está sincronizando. Su saldo y actividad se actualizan a medida que avanza.';

  @override
  String get walletTorOff => 'Tor desactivado';

  @override
  String get walletTorBootstrapping => 'Iniciando la ruta privada…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return 'Iniciando $transport…';
  }

  @override
  String get walletTorActive => 'Tor activo';

  @override
  String get walletTorActiveUnverified => 'Tor activo (entorno no verificado)';

  @override
  String get walletTorActiveUnattested =>
      'Ruta privada en uso (privacidad no verificada)';

  @override
  String get walletTorFellBack => 'Tor no disponible: usando conexión directa';

  @override
  String get walletTorUnavailable => 'Ruta privada no disponible: sin conexión';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport no disponible: sin conexión';
  }

  @override
  String get walletTorUnanswered => 'Ruta privada conectada: no llega nada';

  @override
  String get walletTorUnansweredUnattested =>
      'Ruta privada conectada: no llega nada (privacidad no verificada)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport conectado: no llega nada';
  }

  @override
  String get walletTorUnansweredDirect =>
      'No privado (conexión directa de su app): no llega nada';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Conectado a través de $transport: no llega nada; el proxy puede vincular las conexiones';
  }

  @override
  String get walletTorUnknown =>
      'Estado de Tor desconocido: considérelo como no protegido';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Saldo (a partir del bloque $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Saldo · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Saldo (a partir del bloque $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Conexión';

  @override
  String get walletSyncSheetServer => 'Servidor';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Servidor, $host, abre el selector de servidor';
  }

  @override
  String get walletSyncServerSheetTitle => 'Servidor de sincronización';

  @override
  String get walletSyncServerInUse => 'En uso';

  @override
  String get walletSyncServerAppDefault => 'Predeterminado de la app';

  @override
  String get walletSyncServerCustom => 'Servidor personalizado…';

  @override
  String get walletSyncServerCustomHint => 'https://host:puerto';

  @override
  String get walletSyncServerCheck => 'Comprobar servidor';

  @override
  String get walletSyncServerChecking => 'Comprobando…';

  @override
  String get walletSyncServerUse => 'Usar este servidor';

  @override
  String get walletSyncServerSwitching => 'Cambiando…';

  @override
  String get walletSyncServerContinue => 'Continuar';

  @override
  String get walletSyncServerCancel => 'Cancelar';

  @override
  String get walletSyncServerTrustTitle => '¿Confiar en este servidor?';

  @override
  String get walletSyncServerTrustNotice =>
      'Confías en que este servidor informe tu saldo e historial y retransmita tus pagos. Verá tu dirección IP a menos que Tor esté activo, aproximadamente cuándo se creó tu monedero, las direcciones públicas que tu monedero consulta, las transacciones que busca y las transacciones que envías.';

  @override
  String get walletSyncServerKeyLabel => 'Clave de acceso (opcional)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'Cabecera de la clave';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Escribe la cabecera que espera tu servidor';

  @override
  String get walletSyncServerKeyInvalid =>
      'No se puede usar esta clave o cabecera';

  @override
  String get walletSyncServerKeySaved => 'Clave guardada';

  @override
  String get walletSyncServerKeyShow => 'Mostrar';

  @override
  String get walletSyncServerKeyHide => 'Ocultar';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Tu clave te identifica ante este servidor. Puede vincular tus pagos con tu monedero, incluso a través de Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Cambiar reinicia la sincronización en curso. Tu saldo e historial se conservan. Los fondos pueden aparecer como entrantes hasta que el escaneo del nuevo servidor se ponga al día.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Cambiar vuelve a conectar con el nuevo servidor. Tu saldo e historial se conservan.';

  @override
  String get walletSyncServerUnreachable =>
      'No se pudo contactar con este servidor. Revisa la dirección: si es correcta, o bien este servidor no responde, o bien tu app no puede llegar a él ahora mismo. Inténtalo de nuevo o elige otro servidor.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'No se pudo contactar con este servidor. La billetera no puede saber si este servidor no responde o si tu app no puede llegar a él ahora mismo. Elige otro servidor o inténtalo más tarde.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Este servidor está en otra red de Zcash.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Eso no parece una dirección de servidor. Usa https://host:puerto.';

  @override
  String get walletSyncServerNotOffered => 'Esta app no ofrece este servidor.';

  @override
  String get walletSyncServerBusy =>
      'El monedero está ocupado ahora mismo. Inténtalo en un momento.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'El servidor que elegiste ya no lo ofrece esta app. Se usa $host.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'No se pudo leer la elección de servidor guardada. Se usa $host.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'No se pudo cambiar; se sigue usando $host.';
  }

  @override
  String get walletTransportExplainDirect =>
      'El tráfico de la billetera se conecta directamente al servidor. El servidor puede ver su dirección IP.';

  @override
  String get walletTransportExplainTor =>
      'El tráfico de la billetera se enruta a través de la red Tor, que oculta su dirección IP al servidor.';

  @override
  String get walletTransportExplainBootstrapping =>
      'La ruta privada de su app se está iniciando. El tráfico de la billetera espera a que esté lista antes de conectarse.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport se está iniciando. El tráfico de la billetera espera a que esté listo antes de conectarse.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'No se pudo contactar con Tor, por lo que el tráfico volvió a una conexión directa. El servidor puede ver su dirección IP.';

  @override
  String get walletTransportExplainUnavailable =>
      'La ruta privada de su app no está disponible, así que la billetera no se conecta. Desactive la ruta privada o revise la configuración de red de su app.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport no está disponible, así que la billetera no se conecta. Desactívelo o revise la configuración de red de su app.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'La ruta privada aceptó la conexión, pero no llega nada desde hace un minuto. Puede ser la ruta o el servidor de la billetera: la billetera no puede saber cuál. Sigue intentándolo; si no se resuelve, pruebe otro servidor o revise la configuración de red de su app.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport aceptó la conexión, pero no llega nada desde hace un minuto. Puede ser la ruta o el servidor de la billetera: la billetera no puede saber cuál. Sigue intentándolo; si no se resuelve, pruebe otro servidor o revise la configuración de red de su app.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'El tráfico de la billetera se conecta directamente al servidor. El servidor puede ver su dirección IP. Se aceptó la conexión, pero no llega nada desde hace un minuto. Puede ser la ruta o el servidor de la billetera: la billetera no puede saber cuál. Sigue intentándolo; si no se resuelve, pruebe otro servidor o revise la configuración de red de su app.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'No se puede verificar la privacidad de esta conexión: considérela como no privada. Se aceptó la conexión, pero no llega nada desde hace un minuto. Puede ser la ruta o el servidor de la billetera: la billetera no puede saber cuál. Sigue intentándolo; si no se resuelve, pruebe otro servidor o revise la configuración de red de su app.';

  @override
  String get walletTransportExplainUnverified =>
      'No se puede verificar la privacidad de esta conexión: considérela como no privada.';

  @override
  String get walletTransportExplainHostProxy =>
      'El tráfico de la billetera se enruta a través del transporte de privacidad de esta app, que oculta su dirección IP al servidor.';

  @override
  String get walletOnboardingWelcomeTitle => 'Configure su billetera';

  @override
  String get walletOnboardingWelcomeBody =>
      'Cree una billetera nueva para recibir y guardar ZEC. Generaremos una frase de recuperación y lo guiaremos para respaldarla antes de que puedan llegar fondos, de modo que nada quede en riesgo sin un respaldo.';

  @override
  String get walletCreateButton => 'Crear una billetera nueva';

  @override
  String get walletRestoreButton => 'Restaurar desde una frase de recuperación';

  @override
  String get walletWatchOnlyButton => 'Ver una billetera (solo visualización)';

  @override
  String get walletWatchOnlyTitle => 'Ver una billetera';

  @override
  String get walletWatchOnlyBody =>
      'Pegue una clave de visualización para ver una billetera sin sus claves de gasto. Verá su saldo e historial, pero no podrá enviar fondos. Elija la fecha de inicio aproximada de la billetera para que sepamos hasta dónde retroceder.';

  @override
  String get walletWatchOnlyKeyLabel => 'Clave de visualización';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip =>
      'Escanear un código QR de la clave de visualización';

  @override
  String get walletWatchOnlyScanTitle => 'Escanear clave de visualización';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Apunte la cámara al código QR de la clave de visualización.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Cámara no disponible. Pegue la clave manualmente en su lugar.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Pegar en su lugar';

  @override
  String get walletWatchOnlyScanHint =>
      'O toque el botón de escaneo para leer un código QR de la clave de visualización.';

  @override
  String get walletWatchOnlyScanFilled => 'Clave de visualización escaneada.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Fecha de inicio de la billetera';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Escaneando desde el $date en adelante: los fondos recibidos antes de esa fecha no aparecerán. ¿Billetera más antigua? Elija una fecha anterior.';
  }

  @override
  String get walletWatchOnlyBirthdayPick =>
      'Elija la fecha de inicio de la billetera';

  @override
  String get walletWatchOnlyBirthdayChange => 'Cambiar fecha';

  @override
  String get walletWatchOnlySubmit => 'Ver esta billetera';

  @override
  String get walletWatchOnlyBack => 'Atrás';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Eso no parece ser una clave de visualización válida. Revísela e inténtelo de nuevo.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Esa clave de visualización es para una red diferente. No se puede usar aquí.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'Ya existe una billetera en este dispositivo. Vuelva atrás y ábrala en su lugar.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Esa fecha de inicio es demasiado reciente. Elija una fecha anterior.';

  @override
  String get walletRestoreTitle => 'Restaure su billetera';

  @override
  String get walletRestoreBody =>
      'Ingrese su frase de recuperación para restaurar su billetera: escriba o pegue las palabras en orden, separadas por espacios. Solo frases estándar: si su billetera usó una contraseña adicional (una \"vigesimoquinta palabra\"), esta app aún no puede restaurarla; vería una billetera vacía, no un error.';

  @override
  String get walletRestorePhraseHint =>
      'palabra uno  palabra dos  palabra tres  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count palabras',
      one: '1 palabra',
      zero: 'Aún no hay palabras',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'las frases de recuperación tienen 12, 15, 18, 21 o 24 palabras';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count palabras no son palabras de recuperación: corrija las resaltadas',
      one: '1 palabra no es una palabra de recuperación: corrija la resaltada',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'palabra $index: $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'palabra $index: no es una palabra de recuperación';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Quitar palabra $index';
  }

  @override
  String get walletRestoreSubmit => 'Restaurar billetera';

  @override
  String get walletRestoreBack => 'Atrás';

  @override
  String get walletRestoreBirthdayTitle => 'Hasta dónde escanear hacia atrás';

  @override
  String get walletRestoreBirthdayNone =>
      'Escanearemos todo su historial: más lento, pero no se omite nada.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Escaneando desde el $date en adelante: los fondos recibidos antes de esa fecha no aparecerán. ¿Billetera más antigua? Elija una fecha anterior o escanee todo el historial.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Elegir una fecha';

  @override
  String get walletRestoreBirthdayChange => 'Cambiar fecha';

  @override
  String get walletRestoreBirthdayClear => 'Escanear todo el historial';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'La palabra $index no es una palabra de recuperación. Revise su frase en busca de errores y vuelva a intentarlo.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Esa frase de recuperación no es válida. Revise las palabras y su orden, y vuelva a intentarlo.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Esa frase no coincide con la billetera de este dispositivo. Verifíquela e intente de nuevo.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'Ya existe una billetera en este dispositivo. Vuelva atrás para abrirla.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Esa fecha es demasiado reciente. Elija una fecha anterior o escanee todo.';

  @override
  String get walletGeneratingLabel => 'Creando su billetera…';

  @override
  String get walletOpeningLabel => 'Abriendo su billetera…';

  @override
  String get walletBackupTitle => 'Respalde su frase de recuperación';

  @override
  String get walletBackupBody =>
      'Estas palabras son la ÚNICA forma de recuperar su billetera y sus fondos. Anótelas en orden y guárdelas en un lugar seguro y privado. Nunca las comparta ni las guarde en línea: cualquiera que tenga estas palabras puede tomar sus fondos.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Las capturas de pantalla están desactivadas en esta pantalla.';

  @override
  String get walletBackupSecureNoteOther =>
      'Asegúrese de que nadie pueda ver su pantalla.';

  @override
  String get walletBackupReveal => 'Mostrar frase de recuperación';

  @override
  String get walletBackupRevealing => 'Preparando su frase de recuperación…';

  @override
  String get walletBackupRevealFailed =>
      'No se pudo mostrar su frase de recuperación en este momento. Asegúrese de que su dispositivo esté desbloqueado y vuelva a intentarlo.';

  @override
  String get walletBackupRetryReveal => 'Intentar de nuevo';

  @override
  String get walletBackupReauthFailed =>
      'No se pudo verificar su identidad. Vuelva a intentarlo.';

  @override
  String get walletBackupConfirmCheckbox =>
      'He anotado mi frase de recuperación y la he guardado de forma segura.';

  @override
  String get walletBackupContinue => 'Continuar';

  @override
  String get walletBackupSaveFailed =>
      'No se pudo guardar su confirmación. Vuelva a intentarlo.';

  @override
  String get walletBackupStartOver => 'Empezar de nuevo';

  @override
  String get walletBackupStartOverConfirmTitle =>
      '¿Empezar de nuevo sin esta billetera?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Esto elimina esta billetera del dispositivo y le devuelve al inicio. No se puede depositar nada a través de esta app antes de que termine la configuración.\n\nSi esta billetera alguna vez tuvo fondos, o si se restauró desde una frase de recuperación, solo esa frase puede restaurarla.';

  @override
  String get walletBackupStartOverConfirm => 'Eliminar y empezar de nuevo';

  @override
  String get walletBackupStartOverKeep => 'Conservar esta billetera';

  @override
  String get walletBackupSectionTitle => 'Frase de recuperación';

  @override
  String get walletBackupTileTitle => 'Respalde su frase de recuperación';

  @override
  String get walletBackupTileSubtitle =>
      'Muestre las palabras que pueden recuperar su billetera y sus fondos.';

  @override
  String get walletBackupScreenTitle => 'Frase de recuperación';

  @override
  String get walletBackupDone => 'Listo';

  @override
  String get walletBackupManagedTitle => 'Sin frase de recuperación propia';

  @override
  String get walletBackupManagedBody =>
      'Esta billetera se configuró usando su cuenta de la app que la instaló, por lo que no tiene una frase de recuperación propia. Sus fondos se recuperan junto con esa cuenta: use su respaldo para mantenerlos seguros.';

  @override
  String get walletExportViewingKeyTitle => 'Exportar clave de visualización';

  @override
  String get walletExportViewingKeyTileTitle =>
      'Exportar clave de visualización';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Comparta una copia de solo visualización de su billetera: puede ver su historial, pero no puede gastar sus fondos.';

  @override
  String get walletExportViewingKeyWarning =>
      'Esta clave permite que quien la tenga vea todo lo que esta billetera ha recibido y enviado hasta ahora — y todo lo que recibirá y enviará en el futuro. No permite gastar sus fondos ni recuperar su billetera. Compártala únicamente con alguien de su confianza a quien desee mostrarle su historial completo, como un contador o su propio segundo dispositivo. La única forma de dejar de compartirla más adelante es trasladar sus fondos a una billetera nueva.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Esta clave permite que quien la tenga vea todo lo que esta billetera ha recibido y enviado hasta ahora — y todo lo que recibirá y enviará en el futuro. No permite gastar sus fondos ni recuperar su billetera. Compártala únicamente con alguien de su confianza a quien desee mostrarle su historial completo, como un contador o su propio segundo dispositivo. Una vez compartida, no podrá dejar de compartirla.';

  @override
  String get walletExportViewingKeyReveal => 'Mostrar clave de visualización';

  @override
  String get walletExportViewingKeyRetry => 'Intentar de nuevo';

  @override
  String get walletExportViewingKeyRevealing =>
      'Preparando su clave de visualización…';

  @override
  String get walletExportViewingKeyFailed =>
      'No se pudo mostrar su clave de visualización en este momento. Vuelva a intentarlo en un momento.';

  @override
  String get walletExportViewingKeyQrLabel =>
      'Código QR de la clave de visualización';

  @override
  String get walletExportViewingKeyCopy => 'Copiar clave de visualización';

  @override
  String get walletExportViewingKeyCopied => 'Clave de visualización copiada';

  @override
  String get walletExportViewingKeyDone => 'Listo';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Las capturas de pantalla están desactivadas en esta pantalla.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Asegúrese de que nadie pueda ver su pantalla.';

  @override
  String get walletWatchOnlySectionTitle =>
      'Acerca de esta billetera de solo visualización';

  @override
  String get walletWatchOnlyAboutBody =>
      'Esta es una billetera de solo visualización. Se configuró a partir de una clave de visualización, por lo que puede ver su saldo y su historial, pero no contiene claves de gasto: no hay nada que respaldar aquí, y no puede enviar fondos.';

  @override
  String get walletWatchOnlyBadge => 'Solo visualización';

  @override
  String get walletOnboardingFailedTitle =>
      'No se pudo completar la configuración de la billetera';

  @override
  String get walletOnboardingRetry => 'Intentar de nuevo';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'El almacenamiento seguro de su teléfono no responde. Desbloquee su dispositivo y vuelva a intentarlo. Si esto sigue ocurriendo, reinicie el teléfono.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Esta billetera está abierta en otra ventana o app, o todavía está terminando una operación anterior. Cierre cualquier otra ventana que la esté usando, o espere un momento, y vuelva a intentarlo.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'La clave segura de esta billetera ya no está disponible, por lo que no se puede abrir en este dispositivo. Sus fondos están seguros: restaure desde su frase de recuperación para recuperarlos.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Restaurar desde frase de recuperación';

  @override
  String get walletOnboardingRecoverConfirmTitle =>
      '¿Restaurar esta billetera?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Asegúrese de tener su frase de recuperación antes de continuar: la necesitará en la siguiente pantalla para recuperar sus fondos. Sus fondos están seguros en la cadena de bloques y controlados por esa frase. Esto elimina los datos ilegibles de la billetera en este dispositivo para que pueda reconstruirse.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Cancelar';

  @override
  String get walletOnboardingFailedStorageFull =>
      'No hay suficiente espacio libre para configurar su billetera. Libere espacio y vuelva a intentarlo.';

  @override
  String get walletOnboardingFailedNoVault =>
      'Este dispositivo no tiene un almacén seguro de claves, por lo que la billetera no puede proteger su frase de recuperación aquí.';

  @override
  String get walletOnboardingFailedNetwork =>
      'No se pudo conectar con la red durante la configuración. Verifique su conexión y vuelva a intentarlo.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'La configuración de la billetera no terminó. Vuelva a intentarlo para completarla; no se perdió nada.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Algo salió mal al configurar su billetera. Vuelva a intentarlo.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'La configuración de la billetera de esta app es incorrecta, por lo que la billetera no puede iniciarse. Volver a intentarlo no ayudará: informe esto al desarrollador de la app. Sus fondos no se ven afectados.';

  @override
  String get walletSendButton => 'Enviar';

  @override
  String get walletSendSyncNotRunning =>
      'La sincronización no está en marcha: su saldo disponible para gastar no podrá actualizarse';

  @override
  String get walletSendWaitingForFunds =>
      'Aún sincronizando: podrá enviar en cuanto tenga saldo disponible para gastar';

  @override
  String get walletSendNoSpendableYet =>
      'Aún no hay saldo disponible para gastar';

  @override
  String get walletSendSyncUnavailable =>
      'Podrá enviar cuando se reanude la sincronización';

  @override
  String get walletSendTitle => 'Enviar';

  @override
  String get walletSendUnavailable =>
      'Su billetera no está lista en este momento. Vuelva atrás e intente de nuevo.';

  @override
  String get walletSendWatchOnly =>
      'Esta es una billetera de solo visualización. Puede mostrar saldos y recibir pagos, pero no tiene claves de gasto — así que no puede enviar.';

  @override
  String get walletSendExpiredTitle => 'Esta solicitud de pago expiró';

  @override
  String get walletSendExpiredBody =>
      'La pantalla de envío tardó más de cinco segundos en abrirse, así que se informó a la app de que no se envió nada. Esa respuesta es definitiva: esta solicitud no se puede pagar desde aquí. Para pagar, vuelva a empezar desde la app.';

  @override
  String get walletSendFaultWatchOnly =>
      'Esta es una billetera de solo visualización — no tiene claves de gasto, así que no puede enviar.';

  @override
  String walletSendAvailable(String amount) {
    return 'Disponible para enviar: $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Disponible para enviar: $amount ZEC — su saldo todavía se está poniendo al día';
  }

  @override
  String get walletSendRecipientLabel => 'Dirección del destinatario';

  @override
  String get walletSendRecipientHint =>
      'Dirección de Zcash (comienza con u, z o t)';

  @override
  String get walletSendRecipientLocked =>
      'El destinatario no se puede modificar aquí';

  @override
  String get walletSendAmountLabel => 'Monto (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Memo (opcional)';

  @override
  String get walletSendMemoHint =>
      'Solo se entrega a destinatarios blindados (privados)';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Los memos requieren un destinatario blindado. Esta dirección pública no puede recibir uno.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Este pago ya lleva una referencia de la app, así que no puede llevar además una nota escrita.';

  @override
  String get walletSendMachineMemoTitle =>
      'La app va a adjuntar una referencia';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Dice que es para: $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Queda con la transacción y no se puede quitar después. La cartera no puede comprobar qué contiene.';

  @override
  String get walletSendRecipientShielded => 'Blindada · privada';

  @override
  String get walletSendRecipientTransparent => 'Pública';

  @override
  String get walletSendRecipientInvalid =>
      'Esto no parece una dirección de Zcash válida.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Esta dirección es para una red de Zcash diferente.';

  @override
  String get walletSendReviewButton => 'Revisar pago';

  @override
  String get walletSendQueueButton => 'Poner en cola para enviar después';

  @override
  String get walletSendQueueHint =>
      'Un pago en cola espera en \"Guardado y pendiente\", donde puede enviarlo o cancelarlo. Su comisión de red se calcula cuando se envía.';

  @override
  String get walletSendPreparing => 'Preparando su pago…';

  @override
  String get walletSendSubmitting => 'Enviando…';

  @override
  String get walletSendQueuing => 'Poniendo en cola…';

  @override
  String get walletSendReviewTitle => 'Confirmar pago';

  @override
  String get walletSendTotalLabel => 'Total';

  @override
  String get walletSendFeeLabel => 'Comisión de red';

  @override
  String get walletSendChangeLabel => 'Cambio devuelto';

  @override
  String get walletSendDeshieldTitle => 'Este pago no es privado';

  @override
  String get walletSendDeshieldBody =>
      'Se envía a una dirección pública, por lo que el monto y el destinatario serán públicamente visibles en la cadena de bloques de Zcash.';

  @override
  String get walletSendPublicAckLabel => 'Entiendo que este pago será público.';

  @override
  String get walletSendConfirmButton => 'Enviar ahora';

  @override
  String get walletSendBackButton => 'Atrás';

  @override
  String get walletSendSelfSendNote =>
      'Está enviando a su propia billetera. La comisión de red igual se aplica.';

  @override
  String get walletSendLargeConfirmTitle => '¿Enviar un monto grande?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Esto es casi todo su saldo. Un pago enviado no se puede revertir.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Este es un pago grande. Un pago enviado no se puede revertir.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Este es un pago grande, casi todo su saldo. Un pago enviado no se puede revertir.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Enviar $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Volver';

  @override
  String get walletSendSentTitle => 'Pago enviado';

  @override
  String get walletSendSentBody => 'Su pago fue transmitido a la red.';

  @override
  String get walletSendSavedTitle => 'Guardado: completaremos el envío';

  @override
  String get walletSendSavedBody =>
      'Su pago no pudo salir en este momento, así que quedó guardado y su billetera lo enviará en una próxima sincronización. No se pierde nada.';

  @override
  String get walletSendKeptTitle => 'Guardada';

  @override
  String get walletSendKeptBody =>
      'Su billetera conserva esta transacción, pero no se ha comprometido a enviarla por sí sola. Consulte Actividad para ver en qué estado se encuentra.';

  @override
  String get walletSendPartialBody =>
      'Parte de su pago salió; su billetera completará el resto en una próxima sincronización. No se pierde nada.';

  @override
  String get walletSendInMotionTitle => 'Pago en curso';

  @override
  String get walletSendInMotionBody =>
      'Su pago ha comenzado y se está moviendo a través de una dirección de un solo uso que su billetera controla. No lo envíe de nuevo. Si no se completa, puede recuperar los fondos desde la pantalla de su billetera.';

  @override
  String get walletSendAlreadyTitle => 'Ya enviado';

  @override
  String get walletSendAlreadyBody =>
      'Este pago ya fue enviado: no se enviará dos veces.';

  @override
  String get walletSendFailedTitle => 'No se pudo completar el pago';

  @override
  String get walletSendFailedBody =>
      'Algo salió mal al completar este pago y no se envió nada. Puede intentarlo de nuevo.';

  @override
  String get walletSendTryAgain => 'Intentar de nuevo';

  @override
  String get walletSendDone => 'Listo';

  @override
  String get walletSendAnother => 'Enviar otro';

  @override
  String get walletSendQueuedTitle => 'En cola para enviar';

  @override
  String get walletSendQueuedBody =>
      'Este pago está guardado. Lo encontrará en \"Guardado y pendiente\", donde puede enviarlo ahora o cancelarlo.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Saldo disponible insuficiente: tiene $available ZEC y esto necesita $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'La red Zcash se actualizó y esta aplicación necesita una actualización antes de poder enviar. Tus fondos están seguros.';

  @override
  String get walletSyncUpToDateLimited =>
      'Al día hasta donde esta versión puede leer';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'La red Zcash se actualizó. Esta versión ha escaneado todo lo que puede leer, pero los bloques más recientes podrían contener fondos que aún no puede mostrar, y los memos de pagos recientes no están disponibles. Actualiza la aplicación para verlo todo.';

  @override
  String get walletSyncUpToDateDegraded =>
      'Al día, pero este servidor no sirve todos los pools';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Este servidor rechaza, retiene o informa incorrectamente uno de los pools protegidos de Zcash. Los fondos recibidos en ese pool no se pueden gastar a través de él, y el saldo mostrado es un mínimo. Cambie a otro servidor para usarlos; no es un problema de conexión.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool: este servidor se niega a servirlo';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool: este servidor está reteniendo parte de él';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool: este servidor lo está notificando incorrectamente';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool: se desconoce si este servidor lo sirve';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'Al día con este servidor, pero el servidor va por detrás de la red';

  @override
  String get walletSyncExplainEndpointBehind =>
      'La cadena de este servidor termina en un bloque que la red ya había superado antes de que se compilara esta versión de la app, así que su saldo solo está actualizado hasta ese bloque. Es posible que los pagos nuevos aún no aparezcan y que un pago enviado desde aquí no llegue. Cambie a otro servidor para ponerse al día; no es un problema de conexión.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'Esperando una actualización de la aplicación: tus fondos están seguros y no se ha enviado nada.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'Esperando un servidor que informe la versión de la red: cambia de servidor. Tus fondos están seguros y no se ha enviado nada.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'Esperando un servidor que informe la versión de la red. Si la fecha y la hora de este dispositivo son incorrectas, corrígelas primero y, después, cambia de servidor. Tus fondos están seguros y no se ha enviado nada.';

  @override
  String get walletSyncUnverified =>
      'Al día, pero este servidor no informa la versión de la red';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'Todavía puedes enviar durante aproximadamente $hours horas más; después, cambia de servidor.',
      one:
          'Todavía puedes enviar durante aproximadamente 1 hora más; después, cambia de servidor.',
      zero:
          'Todavía puedes enviar durante menos de una hora; después, cambia de servidor.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'Todavía puedes enviar durante aproximadamente $blocks bloques más; después, cambia de servidor.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Este servidor lleva $blocks bloques sin informar la versión de la red, así que esta aplicación no puede confirmar que sea seguro enviar. Cambia a otro servidor.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Este servidor lleva un día sin informar la versión de la red, así que esta aplicación no puede confirmar que sea seguro enviar. Si la fecha y la hora de este dispositivo son incorrectas, corrígelas primero y, después, cambia a un servidor que informe la versión de la red.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Este servidor nunca ha informado la versión de la red, así que esta aplicación no puede confirmar que sea seguro enviar. Cambia a otro servidor.';

  @override
  String get walletSyncExplainUnverified =>
      'Este servidor no dice en qué versión de la red Zcash está, así que esta aplicación no puede confirmar que un pago que firme será aceptado. Tu saldo está al día. Cambia a otro servidor: esto no es un problema de conexión.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Este servidor no dice en qué versión de la red Zcash está, así que esta aplicación no puede confirmar que un pago que firme será aceptado. Además ha seguido sirviendo bloques que esta billetera tuvo que deshacer después, así que tu saldo puede no estar al día. Cambia a otro servidor: esto no es un problema de conexión.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Este servidor también sigue sirviendo bloques que esta billetera luego tiene que deshacer: cambia de servidor.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Su saldo todavía se está poniendo al día — es posible que haya más disponible a medida que la billetera se sincroniza.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC todavía está llegando y estará disponible para gastar una vez que la cartera se ponga al día.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Ingrese un monto para enviar.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Ingrese el monto como un número, por ejemplo 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'ZEC tiene como máximo 8 decimales.';

  @override
  String get walletSendFaultAmountNotPositive =>
      'Ingrese un monto mayor que cero.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Ese monto es mayor que el suministro total de ZEC.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Esta app actualmente limita los envíos a $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Eso no parece una dirección de Zcash válida para esta red. Verifíquela e intente de nuevo.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Este destinatario no puede recibir un memo. Elimine el memo o envíe a una dirección blindada (privada).';

  @override
  String get walletSendFaultMemoTooLong =>
      'Su memo es demasiado largo. Acórtelo e intente de nuevo.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Ese memo no se puede enviar. Elimínelo e intente de nuevo.';

  @override
  String get walletSendFaultMemoConflict =>
      'No se pudo enviar este pago: la app le adjuntó dos notas. No se envió nada.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Esa dirección es para una red diferente.';

  @override
  String get walletSendFaultUriInvalid =>
      'No se pudo generar este pago. Verifique la dirección y el monto.';

  @override
  String get walletSendFaultNotSynced =>
      'Su billetera todavía no está suficientemente sincronizada. Espere a que la sincronización avance, o ponga esto en cola para enviarlo después.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Su billetera todavía no está suficientemente sincronizada. Espere a que la sincronización avance.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Su billetera todavía no está suficientemente sincronizada, y la sincronización no está en marcha en este momento. Consulte el estado de la sincronización en la pantalla de la billetera.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Los montos vencieron mientras revisaba. Vuelva a revisar el pago.';

  @override
  String get walletSendFaultQueueFull =>
      'Hay demasiados envíos esperando salir. Deje que se envíen primero y luego intente de nuevo.';

  @override
  String get walletSendFaultWalletBusy =>
      'La billetera está ocupada en este momento. Intente de nuevo en un momento.';

  @override
  String get walletSendFaultStorageFull =>
      'No hay suficiente espacio libre para completar este envío. Libere espacio y vuelva a intentarlo.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Hay demasiadas direcciones de un solo uso en uso en este momento. Es posible que algunas se liberen a medida que se confirmen las transferencias, pero esto podría no resolverse por sí solo. Sus fondos están seguros.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'No se pudo preparar este pago. Verifique los detalles e intente de nuevo.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'No se pudo preparar este pago en este momento. Inténtalo de nuevo en un momento.';

  @override
  String get walletSwapButton => 'Intercambiar';

  @override
  String get walletSwapTitle => 'Intercambiar ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'Su billetera no está lista en este momento. Vuelva atrás e intente de nuevo.';

  @override
  String get walletSwapUnavailableOff =>
      'El intercambio no está disponible en este momento.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Esta es una billetera de solo visualización — no puede intercambiar.';

  @override
  String get walletSwapDone => 'Listo';

  @override
  String get walletSwapBackToWallet => 'Volver a la billetera';

  @override
  String walletSwapAvailable(String amount) {
    return 'Disponible para intercambiar: $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Disponible para intercambiar: $amount ZEC — su saldo todavía se está poniendo al día';
  }

  @override
  String get walletSwapAssetLabel => 'Activo a recibir';

  @override
  String get walletSwapAmountLabel => 'Monto a intercambiar (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Dirección de destino';

  @override
  String get walletSwapDestinationHint =>
      'Su dirección de recepción en la cadena de destino';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Su dirección de recepción en $chain';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Una dirección de $chain: adonde se envía el activo intercambiado. Verifique bien que la cadena sea la correcta.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Escanear un código QR de dirección de destino';

  @override
  String get walletSwapTargetAssetHint => 'Seleccione un activo para recibir';

  @override
  String get walletSwapQuoteButton => 'Obtener cotización';

  @override
  String get walletSwapQuoting => 'Obteniendo una cotización…';

  @override
  String get walletSwapExecuting => 'Iniciando su intercambio…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Aún en proceso: el intercambio se está iniciando. Esto puede tardar hasta un minuto.';

  @override
  String get walletSwapReviewTitle => 'Confirmar intercambio';

  @override
  String get walletSwapYouSendLabel => 'Usted envía';

  @override
  String get walletSwapYouReceiveLabel => 'Usted recibe al menos';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Comisión de red';

  @override
  String get walletSwapNetworkFeeValue => 'Se añade al enviar el depósito';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Cotización válida por unos $time — confirme antes de que expire.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Cotización válida por menos de un minuto — confirme antes de que expire.';

  @override
  String get walletSwapQuoteExpired =>
      'Esta cotización expiró. Vuelva atrás y obtenga una nueva: su tasa ya no está garantizada, y enviar ahora arriesga un reembolso.';

  @override
  String get walletCountdownUnderMinute => 'menos de un minuto';

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
  String get walletSwapDeshieldTitle => 'Este intercambio no es privado';

  @override
  String get walletSwapDeshieldBody =>
      'Intercambiar hacia afuera desblinda su ZEC: el depósito es una transacción pública, y el lado del proveedor es público en su red.';

  @override
  String get walletSwapDiscloseTitle => 'Qué verá el proveedor del intercambio';

  @override
  String get walletSwapDiscloseAmounts => 'Los montos de ambos lados';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Que este ZEC y el activo que recibe son un mismo intercambio';

  @override
  String get walletSwapDiscloseDestination => 'Su dirección de destino';

  @override
  String get walletSwapDiscloseSource => 'Su dirección de origen';

  @override
  String get walletSwapDiscloseIp =>
      'Su dirección IP (a menos que enrute a través de Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'Otros detalles de este intercambio';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'Las propias transacciones del proveedor son públicas en su red';

  @override
  String get walletSwapAckLabel =>
      'Entiendo que el proveedor verá la información anterior.';

  @override
  String get walletSwapConfirmButton => 'Iniciar intercambio';

  @override
  String get walletSwapBackButton => 'Atrás';

  @override
  String get walletSwapStatusPendingTitle => 'Intercambio iniciado';

  @override
  String get walletSwapStatusCheckingTitle =>
      'Comprobando el estado del intercambio…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Su billetera está enviando el depósito de ZEC al proveedor. Si está brevemente sin conexión, se envía automáticamente en cuanto vuelva a conectarse — pero la ventana de envío es corta, y si se cierra antes, el intercambio simplemente termina y no se intercambia nada. Su ZEC sigue siendo suyo, y puede tardar hasta una hora en volver a aparecer como disponible.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'Esperando que llegue su depósito. Si aún no ha enviado los fondos desde su otra billetera, envíelos antes de que expire la cotización.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Este intercambio todavía está esperando su depósito. Las instrucciones de depósito ya no están disponibles en este dispositivo — si ya envió los fondos, se detectarán; si no lo ha hecho, deje que este intercambio expire e inicie uno nuevo.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'La ventana de depósito termina $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'La ventana de depósito ha pasado. Si el depósito no se envió a tiempo, el intercambio termina y su ZEC permanece en su billetera.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'La ventana de depósito ha pasado. Si aún no ha enviado su depósito, este intercambio simplemente termina — obtenga una nueva cotización cuando esté listo.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Intercambios en curso',
      one: 'Intercambio en curso',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'Su ZEC va camino al proveedor del intercambio.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'Esperando que su depósito llegue al proveedor del intercambio.';

  @override
  String get walletSwapInFlightRowGeneric => 'Hay un intercambio en curso.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'La ventana de depósito ha pasado — verifique el estado de este intercambio.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Este intercambio aún no ha alcanzado un resultado confirmado aquí — ábralo para verificarlo. Cualquier ZEC que regrese a esta billetera aparecerá en su saldo después de una sincronización.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Este intercambio aún no ha alcanzado un resultado confirmado aquí — ábralo para verificarlo. Cualquier ZEC que este intercambio entregue a esta billetera aparecerá en su saldo después de una sincronización.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Intercambio completado.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Intercambio reembolsado.';

  @override
  String get walletSwapRowOutcomeFailed => 'Intercambio no completado.';

  @override
  String get walletSwapRemove => 'Quitar';

  @override
  String get walletSwapRemoveTitle => '¿Quitar este intercambio de la lista?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Esto solo quita el intercambio de esta lista — no cancela el intercambio, y esta billetera dejará de hacer seguimiento de su reembolso. El ZEC reembolsado más adelante sigue perteneciendo a esta billetera; un reescaneo completo puede encontrarlo.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Esto solo quita el intercambio de esta lista — no cancela el intercambio, y esta billetera dejará de hacer seguimiento del ZEC entrante. El ZEC que llegue más adelante sigue perteneciendo a esta billetera; un reescaneo completo puede encontrarlo. Si el intercambio se reembolsa en su lugar, el reembolso vuelve en el activo que envió, fuera de esta billetera.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Esto solo quita el intercambio de esta lista — no cancela el intercambio, y esta billetera dejará de hacer seguimiento del ZEC que aún pueda estar llegando de él. El ZEC que llegue más adelante sigue perteneciendo a esta billetera; un reescaneo completo puede encontrarlo.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Esto quita el intercambio finalizado de la lista.';

  @override
  String get walletSwapRemoveCancel => 'Cancelar';

  @override
  String get walletSwapRemoveConfirm => 'Quitar';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Iniciado $time';
  }

  @override
  String get walletSwapViewSwap => 'Mostrar intercambio';

  @override
  String get walletSwapsInFlightError =>
      'No se pudieron cargar sus intercambios en curso en este momento.';

  @override
  String get walletSwapsInFlightRetry => 'Intentar de nuevo';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Intentando…';

  @override
  String get walletSwapStartAnother => 'Iniciar otro intercambio';

  @override
  String get walletSwapStatusUnderTitle => 'Esperando el depósito completo';

  @override
  String get walletSwapStatusUnderBody =>
      'Parte del depósito ha llegado. El resto se está completando, o el proveedor hará un reembolso.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Parte de su depósito ha llegado. Envíe el monto faltante antes de la fecha límite, o el proveedor reembolsará lo que llegó.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Recibido: $received; aún falta $missing. La ventana de depósito termina: $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Depósito recibido';

  @override
  String get walletSwapStatusDetectedBody =>
      'El proveedor recibió su depósito y procesará el intercambio.';

  @override
  String get walletSwapStatusProcessingTitle => 'Procesando su intercambio';

  @override
  String get walletSwapStatusProcessingBody =>
      'El proveedor está completando su intercambio.';

  @override
  String get walletSwapStatusSuccessTitle => 'Intercambio completo';

  @override
  String get walletSwapStatusSuccessBody =>
      'Su intercambio finalizó correctamente.';

  @override
  String get walletSwapStatusRefundedTitle => 'Intercambio reembolsado';

  @override
  String get walletSwapStatusRefundedBody =>
      'El intercambio no se completó, por lo que el proveedor devolvió los fondos a su dirección de reembolso.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'El intercambio no se completó, por lo que el proveedor devolvió su ZEC a esta billetera. Llega como fondos sin blindar y aparece en su saldo después de la próxima sincronización de la billetera — esto puede tardar un poco.';

  @override
  String get walletSwapStatusFailedTitle => 'Intercambio fallido';

  @override
  String get walletSwapStatusFailedBody =>
      'El intercambio no se pudo completar. Cualquier fondo depositado se liquida o se reembolsa del lado del proveedor.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Intercambio no encontrado';

  @override
  String get walletSwapStatusNotFoundBody =>
      'El proveedor ya no tiene registro de este intercambio: lo más probable es que haya expirado. Si se realizó un depósito, el proveedor debería reembolsarlo a la dirección de reembolso. El intercambio permanece en su lista, y esta billetera sigue haciendo seguimiento de su ZEC por si todavía llega; puede quitarlo de la lista en cualquier momento.';

  @override
  String get walletSwapStatusUnknownTitle => 'Estado no disponible';

  @override
  String get walletSwapStatusUnknownBody =>
      'No podemos leer el estado de este intercambio en este momento.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'Seguimiento no disponible';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'El intercambio está desactivado, por lo que no podemos hacer seguimiento aquí. Cualquier fondo se liquida o se reembolsa del lado del proveedor.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'El intercambio está desactivado aquí, por lo que no se puede hacer seguimiento a este intercambio en este momento. Si se reembolsó, el ZEC regresa a esta billetera — aparecerá en su saldo después de que se reactive el intercambio y la billetera se sincronice.';

  @override
  String get walletSwapTrackingError =>
      'No pudimos hacer seguimiento de este intercambio.';

  @override
  String get walletSwapTrackingErrorBody =>
      'No pudimos abrir el seguimiento de este intercambio. Es posible que el intercambio en sí siga en curso — cualquier fondo depositado se liquida o se reembolsa del lado del proveedor.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Ingrese la dirección donde desea recibir el activo intercambiado.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Esa dirección de destino no es válida para este activo. Verifíquela e intente de nuevo.';

  @override
  String get walletSwapFaultExpired =>
      'Esta cotización expiró. Obtenga una nueva para continuar.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'El precio del proveedor se movió fuera de su límite, por lo que el intercambio se detuvo antes de que se moviera algo. Intente de nuevo.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'El límite de deslizamiento es demasiado alto para un intercambio seguro. Intente de nuevo.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'El proveedor del intercambio no está disponible en este momento. Intente de nuevo en un momento.';

  @override
  String get walletSwapFaultConnection =>
      'No se pudo conectar con el servicio de intercambio. Verifique su conexión a internet e intente de nuevo.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'El proveedor del intercambio devolvió una respuesta inesperada, por lo que el intercambio se detuvo. Intente de nuevo.';

  @override
  String get walletSwapFaultSwapOff =>
      'El intercambio está desactivado en este momento.';

  @override
  String get walletSwapFaultDepositFailed =>
      'No pudimos enviar su depósito, por lo que nada salió de su billetera. Obtenga una nueva cotización para volver a intentarlo.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Ya hay un intercambio en curso. Podrá iniciar uno nuevo una vez que se liquide por completo o su cotización expire; esto puede tardar un tiempo.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Esta billetera aún no puede configurar una dirección de reembolso — esto suele significar solo que la primera sincronización no ha terminado. Espere a que la sincronización se complete y vuelva a intentarlo.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Esta billetera aún no puede configurar una dirección de recepción para este intercambio — esto suele significar solo que la primera sincronización no ha terminado. Espere a que la sincronización se complete y vuelva a intentarlo.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'El intercambio no pudo iniciarse a tiempo — la conexión pudo ser lenta, o la billetera estaba ocupada. Obtenga una nueva cotización e intente de nuevo.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'La billetera está ocupada por un momento. Intente de nuevo.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Esta cotización no coincide con la que emitió su billetera, así que no se envió nada. Obtenga una cotización nueva y vuelva a intentarlo.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Este intercambio necesita aproximadamente $needed ZEC, incluida la comisión de red, pero solo $spendable ZEC está disponible para gastar en este momento.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Esta app actualmente limita los intercambios a $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Este intercambio necesita aproximadamente $needed ZEC, incluida la comisión de red, pero solo $spendable ZEC está disponible para gastar en este momento. Su saldo todavía se está poniendo al día — es posible que pronto haya más disponible.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'La billetera no pudo registrar este intercambio de forma segura, por lo que nada se movió. Intente de nuevo.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Esa solicitud de intercambio no se pudo procesar. Obtenga una nueva cotización e intente de nuevo.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'No se pudo obtener una cotización de intercambio. Verifique los detalles e intente de nuevo.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Su billetera no está lista en este momento. Vuelva atrás e intente de nuevo.';

  @override
  String get walletSwapDirectionBuy => 'Comprar ZEC';

  @override
  String get walletSwapDirectionSell => 'Vender ZEC';

  @override
  String get walletSwapRefundLabel => 'Su dirección de reembolso';

  @override
  String get walletSwapRefundHint =>
      'Adonde vuelven sus monedas si el intercambio falla';

  @override
  String get walletSwapRefundHelper =>
      'En la cadena desde la que envía, no una dirección de Zcash.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Su dirección de reembolso en $chain';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Una dirección de $chain: adonde vuelven sus monedas si el intercambio falla. No una dirección de Zcash.';
  }

  @override
  String get walletSwapRefundInfoTitle => 'Sobre su dirección de reembolso';

  @override
  String get walletSwapRefundInfoBody =>
      'Si el intercambio no se puede completar, el proveedor devuelve sus monedas a esta dirección en la cadena desde la que pagó. Ingrese una dirección que usted controle: la billetera no puede verificar una dirección externa por usted, así que revísela con cuidado.';

  @override
  String get walletSwapRefundScanTooltip =>
      'Escanear un código QR de dirección de reembolso';

  @override
  String get walletSwapScanTitle => 'Escanear dirección';

  @override
  String get walletSwapScanInstruction =>
      'Apunte la cámara al código QR de la dirección.';

  @override
  String get walletSwapScanManualEntry => 'Ingresar manualmente';

  @override
  String get walletSwapScanCancel => 'Cancelar';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Cámara no disponible. Ingrese la dirección manualmente abajo.';

  @override
  String get walletSwapSourceAssetLabel => 'Activo desde el que intercambiar';

  @override
  String get walletSwapSourceAssetHint => 'Seleccione un activo';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Monto a enviar ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Monto a enviar';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol en $chain';
  }

  @override
  String get walletSwapPickerTitle =>
      'Elija un activo desde el cual intercambiar';

  @override
  String get walletSwapPickerTitleReceive => 'Elija un activo para recibir';

  @override
  String get walletSwapPickerStale =>
      'No se pudo actualizar la lista de activos: se muestra la última lista conocida.';

  @override
  String get walletSwapPickerEmpty =>
      'No hay activos disponibles para intercambiar en este momento. Intente de nuevo más tarde.';

  @override
  String get walletSwapPickerSearchHint => 'Buscar por nombre o cadena';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Ningún activo coincide con \"$query\".';
  }

  @override
  String get walletSwapPickerError =>
      'No se pudo cargar la lista de activos. Verifique su conexión e intente de nuevo.';

  @override
  String get walletSwapPickerRetry => 'Intentar de nuevo';

  @override
  String get walletSwapSlippageLabel => 'Tolerancia de deslizamiento';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value%';
  }

  @override
  String get walletSwapSlippageCustom => 'Personalizado';

  @override
  String get walletSwapSlippageCustomLabel => 'Deslizamiento personalizado';

  @override
  String get walletSwapSlippageMayFail =>
      'Muy bajo: el intercambio puede fallar si el precio se mueve.';

  @override
  String get walletSwapSlippageNormal => 'Una tolerancia segura.';

  @override
  String get walletSwapSlippageRisky =>
      'Alto: podría recibir notablemente menos de lo cotizado.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Demasiado alto: el intercambio será rechazado. Redúzcalo a 10% o menos.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Recibirá al menos $zec ZEC: su piso de deslizamiento del $slippage%. El monto final no bajará de esto.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'Recibe ZEC en su propia dirección';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Hasta que lo blinde (un toque, con un aviso al llegar), el monto recibido está brevemente público y visible en la cadena. Una entrega pequeña puede permanecer pública hasta acumularse.';

  @override
  String get walletSwapRefundVerifyTitle =>
      'Verifique su dirección de reembolso';

  @override
  String get walletSwapRefundVerifyBody =>
      'Revísela carácter por carácter: aquí es adonde vuelven sus monedas si el intercambio falla. La billetera no puede verificar una dirección externa por usted.';

  @override
  String get walletSwapRefundVerifyAck =>
      'He verificado que mi dirección de reembolso es correcta.';

  @override
  String get walletSwapPayoutVerifyTitle =>
      'Verifique su dirección de recepción';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Revísela carácter por carácter: aquí es donde recibirá $asset. La billetera no puede verificar una dirección externa por usted.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'He verificado que mi dirección de recepción es correcta.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'El intercambio está desactivado aquí. Cualquier ZEC que ya esté en camino aparecerá en su billetera después de su próxima sincronización.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Ingrese el monto que desea intercambiar.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Ingrese su dirección de reembolso en la cadena de origen.';

  @override
  String get walletSwapDepositTitle => 'Envíe su pago';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Envíe exactamente $amount $asset en $chain a la dirección de abajo.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Envíe el monto exacto. Si envía menos, o si envía después de que se cierre la ventana, el proveedor le reembolsará a su dirección de reembolso.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Ventana de depósito: quedan $time';
  }

  @override
  String get walletSwapDepositExpired =>
      'Esta ventana de depósito se cerró. No envíe fondos ahora: inicie un nuevo intercambio. Si ya los envió, el proveedor debería reembolsar a su dirección de reembolso.';

  @override
  String get walletSwapDepositQrLabel =>
      'Código QR de la dirección de depósito';

  @override
  String get walletSwapDepositAddressLabel => 'Dirección de depósito';

  @override
  String get walletSwapDepositCopy => 'Copiar dirección de depósito';

  @override
  String get walletSwapDepositCopied => 'Dirección de depósito copiada';

  @override
  String get walletSwapDepositMemoRequired =>
      'Este depósito necesita un memo o tag';

  @override
  String get walletSwapDepositMemoWarning =>
      'DEBE incluir exactamente este memo con su depósito. Enviarlo sin él, o con el memo incorrecto, puede causar la pérdida permanente de sus fondos.';

  @override
  String get walletSwapDepositMemoLabel => 'Memo / tag de depósito';

  @override
  String get walletSwapDepositMemoCopy => 'Copiar memo';

  @override
  String get walletSwapDepositMemoCopied => 'Memo copiado';

  @override
  String get walletSwapDepositSent => 'Ya envié los fondos';

  @override
  String get walletSwapDepositBackTitle => '¿Salir de esta pantalla?';

  @override
  String get walletSwapDepositBackBody =>
      'Esto no cancelará su intercambio: continúa en segundo plano. Pero necesitará la dirección de depósito para pagar, así que cópiela primero si aún no lo hizo.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Esto no cancelará su intercambio: continúa en segundo plano. La ventana de depósito se cerró. No envíe fondos a la dirección de depósito ahora. Si ya los envió, el proveedor debería reembolsar a su dirección de reembolso.';

  @override
  String get walletSwapDepositBackStay => 'Quedarse';

  @override
  String get walletSwapDepositBackLeave => 'Salir';

  @override
  String get walletReceive => 'Recibir';

  @override
  String get walletReceiveSubtitle =>
      'Comparta esta dirección para recibir ZEC. Es seguro compartirla públicamente.';

  @override
  String get walletReceiveCopy => 'Copiar dirección';

  @override
  String get walletReceiveCopied => 'Dirección copiada';

  @override
  String get walletReceiveUnavailable => 'Su billetera aún no está lista.';

  @override
  String get walletReceiveError =>
      'No pudimos cargar su dirección. Vuelva a intentarlo.';

  @override
  String get walletReceivePreparing => 'Preparando su dirección…';

  @override
  String get walletReceivePreparingHint =>
      'Su billetera prepara esta dirección en su dispositivo — esto puede tardar un momento si la billetera está ocupada con otro trabajo.';

  @override
  String get walletReceiveRetry => 'Intentar de nuevo';

  @override
  String get walletReceiveQrLabel => 'Código QR de su dirección de recepción';

  @override
  String get walletReceiveTypeShielded => 'Blindada';

  @override
  String get walletReceiveTypeTransparent => 'Pública';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Comparta esta dirección pública para recibir ZEC de un remitente que no puede pagar a una dirección blindada.';

  @override
  String get walletReceiveTransparentWarning =>
      'Esta es una dirección pública: es visible en la cadena y vincula sus pagos si se reutiliza. Prefiera su dirección blindada; blinde estos fondos después de recibirlos.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'Código QR de su dirección pública de recepción';

  @override
  String get walletReceiveFreshAddress => 'Usar una dirección nueva';

  @override
  String get walletReceiveFreshCaption =>
      'Dirección nueva — no se puede vincular con sus otras direcciones. Los pagos a ella igual llegan a esta billetera, y sus direcciones anteriores siguen funcionando. No volverá a mostrarse aquí — cópiela ahora.';

  @override
  String get walletReceiveFreshError =>
      'No pudimos crear una dirección nueva. Vuelva a intentarlo.';

  @override
  String get walletReceiveFreshBusy =>
      'La billetera está ocupada en este momento. Vuelva a intentarlo con la dirección nueva en unos instantes.';

  @override
  String get walletReceiveShare => 'Compartir';

  @override
  String get walletReceiveRequestAmount => 'Solicitar monto';

  @override
  String get walletReceiveRequestAmountLabel => 'Monto (opcional)';

  @override
  String get walletReceiveFreshCopyNow =>
      'No volverá a mostrarse aquí — cópiela ahora.';

  @override
  String get walletSecurityMenuItem => 'Seguridad…';

  @override
  String get securityTitle => 'Seguridad';

  @override
  String get securityUnavailableBody =>
      'La configuración de seguridad de la billetera la administra esta app, no la billetera en sí.';

  @override
  String get securityCustodySectionTitle => 'Custodia de claves';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (hardware)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (hardware)';

  @override
  String get securityCustodyTierTee => 'Almacén de claves por hardware (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Almacén de claves por software';

  @override
  String get securityCustodyTierKeychain => 'Keychain (cifrado por software)';

  @override
  String get securityCustodyTierNone => 'Sin almacén de claves por hardware';

  @override
  String get securityCustodyTierUnknown => 'Desconocido';

  @override
  String get securityCustodyHardwareKey =>
      'La clave que bloquea esta billetera se guarda en el hardware seguro de este dispositivo y se elimina con la billetera.';

  @override
  String get securityCustodyBestEffort =>
      'Eliminar quita sus claves de la mejor manera posible; puede quedar una breve ventana de recuperación forense hasta que el dispositivo reutilice el almacenamiento. Para una garantía total, utilice también la función de borrado completo de su dispositivo.';

  @override
  String get securityCustodyProbeError =>
      'No se pudo leer el estado de custodia. Vuelva atrás e intente de nuevo.';

  @override
  String get securityDeleteWalletButton => 'Eliminar billetera';

  @override
  String get securityDeleteWalletSubtitle =>
      'Elimine esta billetera y su clave de este dispositivo. Sus fondos permanecen en la cadena y son restaurables desde su frase de recuperación.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Elimine esta billetera y su clave de este dispositivo. No contiene claves de gasto, por lo que no hay nada que respaldar — puede volver a añadirla en cualquier momento con su clave de visualización.';

  @override
  String get securityDeleteDialogTitle => '¿Eliminar esta billetera?';

  @override
  String get securityDeleteDialogBody =>
      'Esto elimina la billetera y su clave de este dispositivo. Asegúrese de haber respaldado su frase de recuperación: es la ÚNICA forma de restaurar sus fondos.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Esto elimina la billetera y su clave de este dispositivo. No contiene claves de gasto, por lo que no es necesario respaldar nada — puede volver a añadirla más adelante con su clave de visualización.';

  @override
  String get securityDeleteDialogConfirm => 'Eliminar';

  @override
  String get securityDeleteDialogCancel => 'Cancelar';

  @override
  String get securityDeleteFailedSnack =>
      'No se pudo eliminar la billetera: su billetera no sufrió cambios. Intente de nuevo.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Primero termine el cambio de servidor: se completa o se detiene en un máximo de $seconds segundos. Después intente eliminar la billetera de nuevo.';
  }

  @override
  String get walletParkedTitle => 'Guardado y pendiente';

  @override
  String get walletParkedSubtitle =>
      'Estos pagos aún no se han enviado. Sus montos siguen siendo parte de su saldo.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Estos pagos aún no se han enviado. Sus montos siguen siendo parte de su saldo — excepto los que su billetera está enviando, cuyo monto es posible que ya esté reservado.';

  @override
  String get walletParkedCancel => 'Cancelar';

  @override
  String get walletParkedPausedHint =>
      'Pausado: este pago no se enviará por sí solo. Sus fondos están seguros. Envíelo ahora, o cancélelo.';

  @override
  String get walletParkedRetryStale =>
      'Este pago ya no está en espera. Revise sus pagos pendientes y su actividad.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Este pago ya no está en espera — es posible que su billetera ya lo esté enviando. Revise \"Guardado y pendiente\" y su actividad.';

  @override
  String get walletReclaimExplainer =>
      'Los envíos por dirección de un solo uso están atascados. Puede reabrirlos: esto mueve un monto pequeño entre sus propias direcciones y se lo devuelve.';

  @override
  String get walletReclaimButton => 'Reabrir envío';

  @override
  String get walletReclaimInProgress => 'Reabriendo…';

  @override
  String get walletReclaimConfirmTitle =>
      '¿Reabrir el envío por dirección de un solo uso?';

  @override
  String get walletReclaimConfirmBody =>
      'Esto mueve un monto pequeño entre sus propias direcciones para liberar el envío por dirección de un solo uso, y luego se lo devuelve. Cuesta un par de comisiones de red. Una vez confirmado, recupere el monto movido con \"Recuperar ahora\".';

  @override
  String get walletReclaimConfirmCancel => 'Ahora no';

  @override
  String get walletReclaimConfirmAction => 'Reabrir';

  @override
  String get walletReclaimStarted =>
      'Reapertura iniciada. Una vez que esto se confirme, envíe el pago pausado y luego recupere el monto movido con \"Recuperar ahora\".';

  @override
  String get walletReclaimNothing => 'Nada para reabrir en este momento.';

  @override
  String get walletReclaimNotBroadcast =>
      'No se pudo confirmar que esto llegó a la red. Es posible que de todos modos se haya transmitido. Vuelva a intentarlo en un momento.';

  @override
  String get walletReclaimNeedsFunds =>
      'Necesita algo de ZEC blindado para reabrir el envío.';

  @override
  String get walletReclaimFailed =>
      'No se pudo reabrir el envío en este momento. Sus fondos no sufrieron cambios. Intente de nuevo.';

  @override
  String get walletReclaimUnknown =>
      'Reapertura finalizada. Revise sus envíos por dirección de un solo uso y recupere cualquier monto movido con \"Recuperar ahora\".';

  @override
  String get walletParkedError =>
      'No se pudieron cargar sus pagos pendientes en este momento.';

  @override
  String get walletParkedErrorRetry => 'Intentar de nuevo';

  @override
  String get walletParkedErrorRetryInProgress => 'Intentando…';

  @override
  String get walletParkedCancelConfirmTitle => '¿Cancelar este pago pendiente?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Esto descarta el pago guardado. No ha sido enviado, por lo que nada sale de su billetera, pero esto no se puede deshacer.';

  @override
  String get walletParkedCancelConfirmKeep => 'Conservarlo';

  @override
  String get walletParkedCancelConfirmDiscard => 'Descartar pago';

  @override
  String get walletParkedCancelDone => 'Pago pendiente cancelado.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Este pago puede que ya esté en camino: revise su actividad.';

  @override
  String get walletParkedCancelFailed =>
      'No se pudo cancelar en este momento. Su pago no sufrió cambios. Intente de nuevo.';

  @override
  String get walletRecoverNow => 'Recuperar ahora';

  @override
  String get walletRecoverConfirmTitle => '¿Recuperar a su saldo blindado?';

  @override
  String get walletRecoverConfirmBody =>
      'Esto verifica sus direcciones de un solo uso y mueve todo lo encontrado a su saldo privado blindado. Es seguro ejecutarlo de nuevo en cualquier momento.';

  @override
  String get walletRecoverConfirmCancel => 'Ahora no';

  @override
  String get walletRecoverConfirmAction => 'Recuperar';

  @override
  String get walletRecoverInProgress => 'Recuperando…';

  @override
  String walletRecoverDone(String amount) {
    return 'Recuperando $amount a su saldo blindado.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return 'Recuperando $amount: algunos fondos aún necesitan otro intento.';
  }

  @override
  String get walletRecoverRetry =>
      'Algunos fondos necesitan otro intento: vuelva a ejecutar la recuperación.';

  @override
  String get walletRecoverTruncated =>
      'Aún no se verificaron todas las direcciones de un solo uso: vuelva a ejecutar la recuperación para verificar el resto.';

  @override
  String get walletRecoverNothing => 'Nada para recuperar en este momento.';

  @override
  String get walletRecoverFailed =>
      'No se pudo recuperar en este momento. Sus fondos no sufrieron cambios. Intente de nuevo.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount guardado y pendiente · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Cancelar el pago de $amount guardado $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount pausado · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount en preparación para el envío · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Su billetera está preparando este pago — es posible que su monto ya esté reservado. Sus fondos están seguros. Si no termina, vuelve a la lista por sí solo.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Su billetera está preparando este pago — es posible que su monto ya esté reservado. Sus fondos están seguros, pero solo podrá completarse cuando su billetera vuelva a sincronizar.';

  @override
  String get walletParkedSendNow => 'Enviar ahora';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Enviando el pago de $amount guardado $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Enviar ahora el pago de $amount guardado $time';
  }

  @override
  String get walletParkedSendNowInProgress => 'Enviando…';

  @override
  String get walletParkedAuthorizeSent => 'Enviando su pago ahora.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Enviando su pago ahora. Si no se completa, su billetera solo podrá terminarlo una vez que vuelva a sincronizar.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Aún no está listo para enviarse. Su pago está guardado y no ha cambiado.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Aún no está listo para enviarse. Su pago está guardado y ya no está pausado — vuelva a intentarlo más tarde con \"Enviar ahora\", o cancélelo.';

  @override
  String get walletParkedAuthorizeFailed =>
      'No se pudo enviar en este momento. Su pago no sufrió cambios. Intente de nuevo.';

  @override
  String get walletTransparentFundsMenuItem => 'Fondos públicos…';

  @override
  String get walletTransparentFundsTitle => 'Fondos públicos';

  @override
  String get walletTransparentFundsIntro =>
      'Los fondos públicos son públicamente visibles en la cadena de bloques: el monto, las direcciones y el historial de las monedas.';

  @override
  String get walletExpertToggleLabel => 'Avanzado: fondos públicos';

  @override
  String get walletExpertToggleDescription =>
      'Mostrar controles avanzados para mantener fondos públicos y desactivar el blindaje automático.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Mostrar controles avanzados para mantener fondos públicos.';

  @override
  String get walletAutoShieldToggleLabel => 'Blindar automáticamente';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Cuando su saldo público alcanza los $minZec ZEC, se mueve automáticamente a su saldo blindado. Si esto está desactivado, los fondos públicos permanecen públicamente visibles hasta que usted mismo los blinde.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'No se pudo guardar la configuración. Intente de nuevo.';

  @override
  String get walletAutoShieldIncomplete =>
      'El blindaje automático no se completó: estos fondos siguen siendo públicamente visibles. Puede blindarlos ahora.';

  @override
  String get walletSendPrivacyShielded =>
      'Pago blindado: el monto y el destinatario permanecen privados en la cadena.';

  @override
  String get walletSendPrivacyTransparent =>
      'Pago público: el monto y las direcciones son visibles en la cadena de bloques.';

  @override
  String get walletActivityPublicBadge => 'Públicamente visible en la cadena';

  @override
  String get walletShieldWalletEnded =>
      'La sesión de la billetera terminó. Ciérrela y vuelva a abrirla para intentarlo de nuevo.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Los nuevos fondos públicos se blindan automáticamente hacia su saldo privado en cuanto alcanzan los $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'El blindaje automático está desactivado: los fondos públicos permanecen públicamente visibles hasta que usted los blinde.';

  @override
  String get walletMoveAutoShieldNote =>
      'El blindaje automático está activado: en cuanto lleguen estos fondos, se blindarán de nuevo automáticamente (por otra comisión). Para mantenerlos públicos, primero desactive el blindaje automático en Fondos públicos.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Tras este movimiento, su saldo público será de $amount ZEC, por debajo de los $floor ZEC necesarios para volver a blindarlo. Seguirá siendo público hasta que llegue más.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Está moviendo fondos a su propia dirección pública. Este movimiento queda en el registro público de forma permanente.';

  @override
  String get walletTxDetailVisibility => 'Visibilidad';

  @override
  String get walletTransparentFundsAutoDenied =>
      'El blindaje automático está pausado para esta sesión: no fue aprobado. Puede seguir blindando manualmente.';

  @override
  String get walletDeepScanMenuItem =>
      'Revisar direcciones de intercambio anteriores…';

  @override
  String get walletMenuSyncNotRunningHint =>
      'La sincronización no está en marcha en este momento.';

  @override
  String get walletDeepScanTitle =>
      'Revisar direcciones de intercambio anteriores';

  @override
  String get walletDeepScanBody =>
      'Si restauró esta billetera y esta usaba intercambios con frecuencia, el dinero de sus intercambios más antiguos puede necesitar un paso adicional para encontrarse. Esto lo revisa — cualquier fondo que se encuentre aparecerá en su saldo a medida que su billetera se sincroniza.';

  @override
  String get walletDeepScanCoverage =>
      'Sus direcciones de intercambio anteriores están revisadas hasta aquí. Si todavía falta dinero de un intercambio antiguo, revise aún más atrás.';

  @override
  String get walletDeepScanCoveragePending =>
      'Aún se está revisando el rango actual — cualquier fondo que se encuentre aparecerá en su saldo. Esto puede tardar un poco.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Busca fondos de los intercambios más antiguos de su billetera.';

  @override
  String get walletDeepScanCheckButton => 'Revisar direcciones anteriores';

  @override
  String get walletDeepScanCheckDeeperButton =>
      'Revisar direcciones aún más antiguas';

  @override
  String get walletDeepScanChecking => 'Revisando…';

  @override
  String get walletDeepScanClose => 'Cerrar';

  @override
  String get walletDeepScanTorHint =>
      'En este momento no está conectado mediante Tor. Para mayor privacidad, considere esperar hasta que Tor esté activo antes de revisar.';

  @override
  String get walletDeepScanRescanBusy =>
      'Podrá revisar direcciones de intercambio anteriores en cuanto finalice el reescaneo.';

  @override
  String get walletDeepScanRan =>
      'Se están revisando direcciones de intercambio anteriores — cualquier fondo que se encuentre aparecerá en su saldo.';

  @override
  String get walletDeepScanFailed =>
      'No se pudo iniciar la revisión. Nada cambió — vuelva a intentarlo.';

  @override
  String get walletDeepScanSlow =>
      'Esto está tardando más de lo habitual. Si se revisaron sus direcciones de intercambio anteriores, cualquier fondo que se encuentre aparecerá en su saldo — vuelva a revisar en breve.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'El intercambio está desactivado en este momento, por lo que esto no se puede ejecutar. Vuelva a intentarlo cuando el intercambio esté disponible.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'Aún se está revisando el último rango — esto puede tardar hasta un par de días, aunque normalmente mucho menos. Termina por sí solo; vuelva a revisar más tarde.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'Aún no podemos confirmar la privacidad de su conexión. Para mayor privacidad, considere esperar hasta que Tor esté activo antes de revisar.';

  @override
  String get walletDeepScanBannerChecking =>
      'Aún se están revisando direcciones de intercambio anteriores — cualquier fondo que se encuentre aparecerá en su saldo.';

  @override
  String get walletRescanSwapPointer =>
      '¿Busca fondos de un intercambio antiguo? Un reescaneo no los encontrará — use “Revisar direcciones de intercambio anteriores” en su lugar.';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      '¿Restauró una billetera que usaba intercambios?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Si esta billetera tuvo un historial de intercambios muy largo, el dinero de sus intercambios más antiguos puede necesitar un paso adicional para encontrarse. La mayoría de las billeteras no necesitan nada.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Revisar ahora';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Descartar';

  @override
  String walletTorHostPath(String transport) {
    return 'A través de la ruta privada de su app ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'A través de la ruta privada de su app ($transport); el proxy puede vincular las conexiones';
  }

  @override
  String get walletTorHostOtherTransport => 'una ruta privada';

  @override
  String get walletTorHostDirect => 'No privado (conexión directa de su app)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Su servidor guardado usa una dirección sin cifrar, que la ruta privada de su app no puede transportar. Se usa $host.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'Más sobre $label';
  }

  @override
  String get walletSendPaste => 'Pegar';

  @override
  String get walletSendScanQr => 'Escanear código QR';

  @override
  String get walletSendRecipientGetsLabel => 'El destinatario recibe';

  @override
  String get walletSwapDepositCopyAmount => 'Copiar importe';

  @override
  String get walletSwapDepositAmountCopied => 'Importe copiado';

  @override
  String get walletScanOpenSettings => 'Abrir ajustes';

  @override
  String get walletScanOpenSettingsFailed =>
      'No se pudieron abrir los ajustes.';

  @override
  String get walletSendLeaveTitle => 'Enviando todavía';

  @override
  String get walletSendLeaveBody =>
      'Tu pago continúa si sales. Verás cómo terminó en tu actividad.';

  @override
  String get walletSendLeaveStay => 'Quedarse';

  @override
  String get walletSendLeaveConfirm => 'Salir';

  @override
  String get walletSheetLeaveBody =>
      'Esto continúa si sales. Verás cómo terminó en tu actividad.';

  @override
  String get walletLoadingLabel => 'Cargando';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'Compruebe antes de volver a blindar';

  @override
  String get walletShieldUnknownBody =>
      'No pudimos confirmar este blindaje. Consulte Actividad antes de intentarlo de nuevo.';

  @override
  String get walletMoveUnknownTitle => 'Compruebe antes de volver a mover';

  @override
  String get walletMoveUnknownBody =>
      'No pudimos confirmar este movimiento. Consulte Actividad antes de intentarlo de nuevo.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
