// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'wallet_localizations.dart';

// ignore_for_file: type=lint

/// The translations for French (`fr`).
class WalletLocalizationsFr extends WalletLocalizations {
  WalletLocalizationsFr([String locale = 'fr']) : super(locale);

  @override
  String get walletAppearanceMenuItem => 'Réglages';

  @override
  String get walletTitle => 'Portefeuille';

  @override
  String get walletNotSetUpTitle => 'Portefeuille pas encore configuré';

  @override
  String get walletNotSetUpBody =>
      'La configuration du portefeuille arrivera dans une version ultérieure. Elle vous guidera pour noter votre phrase de récupération avant de pouvoir recevoir des fonds — ainsi rien n\'est jamais exposé sans sauvegarde.';

  @override
  String get walletStartupFailedTitle => 'Le portefeuille n\'a pas pu démarrer';

  @override
  String get walletStartupFailedBody =>
      'Quelque chose a empêché le portefeuille de se charger sur cet appareil. Si vous avez déjà un portefeuille, ses fonds ne sont pas affectés — ils se trouvent sur le réseau Zcash et peuvent être restaurés avec votre phrase de récupération. Réessayez ; si cela continue, fermez puis rouvrez l\'application.';

  @override
  String get walletBalanceLabel => 'Solde';

  @override
  String get walletHideBalance => 'Masquer le solde';

  @override
  String get walletShowBalance => 'Afficher le solde';

  @override
  String get walletBalanceHiddenAmount => 'Solde masqué';

  @override
  String walletAmount(String amount) {
    return '$amount ZEC';
  }

  @override
  String get walletSpendableLabel => 'Disponible maintenant';

  @override
  String get walletArrivingLabel => 'En arrivée';

  @override
  String get walletNotSpendableYetLabel => 'Pas encore dépensable';

  @override
  String get walletActivityTitle => 'Activité';

  @override
  String get walletActivityEmpty => 'Aucune activité pour l\'instant';

  @override
  String get walletActivityError => 'Impossible de charger l\'activité';

  @override
  String get walletActivityReceived => 'Reçu';

  @override
  String get walletActivitySent => 'Envoyé';

  @override
  String get walletActivityPending => 'En attente';

  @override
  String get walletActivityQueued => 'En file d\'attente';

  @override
  String get walletActivityRetrying => 'Nouvelle tentative';

  @override
  String get walletActivitySaved => 'Enregistrée';

  @override
  String get walletActivityExpired => 'Expiré';

  @override
  String get walletActivityFailed => 'Échoué';

  @override
  String walletActivityConfirmations(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count confirmations',
      one: '1 confirmation',
    );
    return '$_temp0';
  }

  @override
  String walletPaymentReceived(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count paiements reçus',
      one: 'Paiement reçu',
    );
    return '$_temp0';
  }

  @override
  String get walletActivityRowHint => 'Afficher les détails de la transaction';

  @override
  String get walletTxDetailStatus => 'Statut';

  @override
  String get walletTxDetailFee => 'Frais de réseau';

  @override
  String get walletTxDetailDate => 'Date';

  @override
  String get walletTxDetailHeight => 'Hauteur de bloc';

  @override
  String get walletTxDetailMemo => 'Mémo';

  @override
  String get walletTxDetailMemoAttached => 'Inclus';

  @override
  String get walletTxDetailTxid => 'ID de transaction';

  @override
  String get walletTxDetailCopyTxid => 'Copier l\'ID de transaction';

  @override
  String get walletTxDetailCopied => 'ID de transaction copié';

  @override
  String get walletTxDetailClose => 'Fermer';

  @override
  String get walletTxFundsKept => 'Aucun fonds n\'a quitté votre portefeuille';

  @override
  String get walletTxExplainQueued =>
      'Enregistré sur cet appareil, dans « Enregistré et en attente » — vous pouvez l\'envoyer ou l\'annuler à cet endroit.';

  @override
  String get walletTxExplainPending =>
      'Envoyé au réseau Zcash — en attente de confirmation dans un bloc.';

  @override
  String get walletTxExplainRetrying =>
      'Votre portefeuille n\'a pas encore pu envoyer ceci au réseau Zcash. Il conserve la transaction signée et réessaie à chaque synchronisation jusqu\'à ce qu\'elle passe ou expire.';

  @override
  String get walletTxExplainSaved =>
      'Votre portefeuille a conservé cette transaction signée, mais ne l\'envoie pas de lui-même pour le moment.';

  @override
  String get walletTxExplainConfirmed => 'Confirmé sur le réseau Zcash.';

  @override
  String get walletTxExplainExpired =>
      'Cette transaction a expiré avant d\'être confirmée par le réseau ; elle a donc été annulée. Le montant reste disponible pour vous.';

  @override
  String get walletTxExplainFailed =>
      'Le réseau a rejeté cette transaction, elle n\'a donc pas abouti. Le montant reste disponible pour vous.';

  @override
  String get walletTxExplainUnknown =>
      'Le statut actuel de cette transaction ne peut pas être déterminé. Il sera mis à jour après la prochaine synchronisation.';

  @override
  String get walletMenuTooltip => 'Plus d\'options';

  @override
  String get walletRescanMenuItem => 'Réanalyser l\'historique…';

  @override
  String get walletCheckOneTimeMenuItem =>
      'Vérifier les adresses à usage unique…';

  @override
  String get walletRescanTitle => 'Réanalyser votre historique';

  @override
  String get walletRescanBody =>
      'Des fonds plus anciens manquent ? Réanalysez la blockchain depuis une date plus reculée pour récupérer les dépôts qu\'une date de départ trop récente aurait ignorés. Vos fonds et votre phrase de récupération ne sont jamais exposés.';

  @override
  String get walletRescanRangeTitle => 'Jusqu\'où remonter pour l\'analyse';

  @override
  String get walletRescanRangeAll =>
      'Analyser tout votre historique — le plus lent, mais récupère tout.';

  @override
  String get walletRescanRangeDefault =>
      'Analyse à partir du début de votre portefeuille. Des fonds plus anciens manquent toujours ? Choisissez une date antérieure, ou analysez tout l\'historique.';

  @override
  String get walletRescanRangeResolving =>
      'Préparation de la plage recommandée…';

  @override
  String walletRescanEstimate(String blocks) {
    return 'Environ $blocks blocs à analyser.';
  }

  @override
  String walletRescanRangeChosen(String date) {
    return 'Analyse à partir du $date. Des fonds plus anciens manquent toujours ? Choisissez une date antérieure, ou analysez tout l\'historique.';
  }

  @override
  String get walletRescanPick => 'Choisir une date';

  @override
  String get walletRescanChange => 'Changer la date';

  @override
  String get walletRescanScanAll => 'Analyser tout l\'historique';

  @override
  String get walletRescanDatePick => 'Date la plus ancienne à analyser';

  @override
  String get walletRescanWarning =>
      'Cela réanalyse la blockchain. Les dates récentes prennent quelques minutes ; remonter loin peut prendre des heures. La synchronisation s\'exécute en arrière-plan — vous pouvez continuer à utiliser votre portefeuille.';

  @override
  String get walletRescanSettlingAdvisory =>
      'Un paiement de ce portefeuille est encore en cours de confirmation. Le portefeuille refuse généralement la réanalyse tant que ce paiement n\'est pas terminé — vous pouvez essayer, mais attendez-vous à un refus.';

  @override
  String get walletRescanConfirm => 'Démarrer la réanalyse';

  @override
  String get walletRescanCancel => 'Annuler';

  @override
  String get walletRescanRunning => 'Reconstruction…';

  @override
  String get walletRescanRebuildingAll =>
      'Reconstruction de votre historique — analyse de toute la chaîne. Votre solde et votre activité se complètent au fur et à mesure.';

  @override
  String walletRescanRebuildingFrom(String date) {
    return 'Reconstruction de votre historique depuis le $date — votre solde et votre activité se complètent au fur et à mesure.';
  }

  @override
  String get walletRescanRebuildingDefault =>
      'Reconstruction de votre historique depuis le début de votre portefeuille — votre solde et votre activité se complètent au fur et à mesure.';

  @override
  String get walletCatchUpBanner =>
      'Rattrapage en cours — votre solde et votre activité se complètent à mesure que le portefeuille se synchronise. Tout ce que vous avez reçu est en sécurité.';

  @override
  String get walletCatchUpRescanBanner =>
      'Reconstruction de votre historique après une réanalyse — votre solde et votre activité se complètent au fur et à mesure. Tout ce que vous avez reçu est en sécurité.';

  @override
  String get walletRescanFailedNotice =>
      'Impossible de réanalyser pour le moment — vos fonds sont en sécurité, même si votre solde et votre historique peuvent mettre un peu de temps à se remettre à jour. Réessayez dans un instant.';

  @override
  String get walletRescanBlockedSettlingNotice =>
      'Un paiement est encore en cours de confirmation, la réanalyse est donc suspendue pour protéger vos fonds. Votre portefeuille est inchangé — réessayez dans quelques heures et laissez l\'application ouverte et connectée pendant ce temps.';

  @override
  String get walletRescanBlockedSyncNotRunningNotice =>
      'La réanalyse reconstruit votre historique à mesure que votre portefeuille se synchronise, et la synchronisation n\'est pas en cours pour le moment. Votre portefeuille est inchangé — réessayez une fois que la synchronisation sera en cours.';

  @override
  String get walletRescanNeedsSpaceNotice =>
      'Il n\'y a pas assez d\'espace libre pour reconstruire l\'historique de votre portefeuille — vos fonds sont en sécurité, même si votre solde et votre historique peuvent mettre un peu de temps à se remettre à jour. Libérez de l\'espace et réessayez.';

  @override
  String get walletRescanFailedDismiss => 'Ignorer';

  @override
  String get walletActivityRebuilding => 'Reconstruction de votre historique…';

  @override
  String get walletActivityCatchingUp =>
      'Rattrapage toujours en cours — tout ce que vous avez reçu s’affichera ici.';

  @override
  String get walletActivitySyncNotRunning =>
      'Votre solde et votre historique termineront de se charger une fois que la synchronisation sera en cours.';

  @override
  String get walletActivityLoadMore => 'Charger plus';

  @override
  String get walletPendingChangeLabel => 'Monnaie en attente';

  @override
  String get walletTransparentLabel => 'Non protégé (public)';

  @override
  String get walletTransparentNote =>
      'Non inclus dans « Disponible maintenant » — protégez ces fonds pour pouvoir les dépenser. Jusque-là, ils restent visibles publiquement sur la chaîne.';

  @override
  String get walletTransparentNoteWatchOnly =>
      'Ces fonds sont visibles publiquement sur la chaîne.';

  @override
  String walletPoolShielded(String amount) {
    return 'Protégé $amount';
  }

  @override
  String walletPoolTransparent(String amount) {
    return 'Public $amount';
  }

  @override
  String get walletPoolAllShielded => 'Tout protégé · privé';

  @override
  String get walletPoolTapHint => 'Afficher les fonds publics';

  @override
  String walletRecoverableEphemeralNote(String amount) {
    return '$amount de votre solde se trouve sur une adresse à usage unique (récupérable).';
  }

  @override
  String walletRecoverableEphemeralNoteWatchOnly(String amount) {
    return '$amount de votre solde se trouve sur une adresse à usage unique.';
  }

  @override
  String walletInFlightNote(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Des paiements totalisant $amount sont réservés et toujours en cours de finalisation via des adresses à usage unique contrôlées par votre portefeuille. Ne les envoyez pas de nouveau.',
      one:
          '$amount est réservé pour un paiement que votre portefeuille est encore en train de finaliser via une adresse à usage unique qu\'il contrôle. Ne l\'envoyez pas de nouveau.',
    );
    return '$_temp0';
  }

  @override
  String walletInFlightNoteSyncPaused(num count, String amount) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          'Des paiements totalisant $amount sont réservés et à mi-chemin via des adresses à usage unique contrôlées par votre portefeuille. Ils sont en pause jusqu\'à ce que votre portefeuille se synchronise à nouveau. Ne les envoyez pas de nouveau.',
      one:
          '$amount est réservé pour un paiement qui est à mi-chemin via une adresse à usage unique que votre portefeuille contrôle. Il est en pause jusqu\'à ce que votre portefeuille se synchronise à nouveau. Ne l\'envoyez pas de nouveau.',
    );
    return '$_temp0';
  }

  @override
  String get walletInFlightReadError =>
      'Impossible de vérifier si un paiement est encore en cours. Nouvel essai en cours — d\'ici là, cherchez un paiement en attente dans votre activité avant d\'envoyer à nouveau.';

  @override
  String walletRecoverableEphemeralConfirmingNote(String amount) {
    return '$amount de votre solde se trouve sur une adresse à usage unique (confirmation en cours).';
  }

  @override
  String get walletShieldButton => 'Protéger';

  @override
  String get walletShieldSheetTitle => 'Protéger les fonds publics';

  @override
  String get walletShieldNote =>
      'Cette opération déplace des fonds de votre solde public, visible sur la chaîne, vers votre solde privé protégé.';

  @override
  String get walletShieldPreparing => 'Préparation…';

  @override
  String get walletShieldAmountLabel => 'Montant à protéger';

  @override
  String get walletShieldFeeLabel => 'Frais de réseau';

  @override
  String get walletShieldNetLabel => 'Montant net protégé';

  @override
  String get walletShieldConfirmButton => 'Protéger maintenant';

  @override
  String get walletShieldSubmitting => 'Protection en cours…';

  @override
  String get walletShieldNothingTitle => 'Rien à protéger pour l\'instant';

  @override
  String get walletShieldNothingBody =>
      'Ces fonds sont actuellement inférieurs au montant qu\'il vaut la peine de protéger — les frais de réseau dépasseraient le bénéfice. Ils pourront être protégés une fois qu\'un peu plus sera arrivé.';

  @override
  String get walletShieldDoneTitle => 'Protection soumise';

  @override
  String get walletShieldDoneBody =>
      'Vos fonds sont en cours de transfert vers votre solde protégé. La confirmation sur la chaîne arrivera sous peu.';

  @override
  String get walletShieldSavedTitle =>
      'Enregistré — la protection sera finalisée';

  @override
  String get walletShieldSavedBody =>
      'Impossible de joindre le réseau pour le moment. Votre protection est enregistrée, et votre portefeuille la terminera lors d\'une prochaine synchronisation. Rien n\'est perdu.';

  @override
  String get walletShieldAlreadyTitle => 'Déjà soumis';

  @override
  String get walletShieldFailedTitle => 'Impossible de protéger pour le moment';

  @override
  String get walletShieldStaleBody =>
      'Le portefeuille est encore en synchronisation. Réessayez de protéger dans un instant.';

  @override
  String get walletShieldTransientBody =>
      'Impossible de préparer le blindage pour le moment. Réessayez dans un instant.';

  @override
  String get walletShieldStorageFullBody =>
      'Il n\'y a pas assez d\'espace libre pour protéger maintenant. Libérez de l\'espace et réessayez. Vos fonds sont en sécurité.';

  @override
  String get walletShieldClose => 'Fermer';

  @override
  String get walletShieldRetry => 'Réessayer';

  @override
  String get walletMoveMenuItem => 'Déplacer vers public…';

  @override
  String get walletMoveSheetTitle => 'Déplacer vers public';

  @override
  String get walletMoveSheetSubtitle =>
      'Envoyez du ZEC protégé vers votre propre adresse publique — utile pour une plateforme d\'échange qui n\'accepte pas les dépôts protégés.';

  @override
  String get walletMoveDestinationLabel => 'Votre adresse publique';

  @override
  String walletMoveAvailable(String amount) {
    return 'Disponible à déplacer : $amount ZEC';
  }

  @override
  String walletMoveAvailableCatchingUp(String amount) {
    return 'Disponible à déplacer : $amount ZEC — votre solde rattrape encore son retard';
  }

  @override
  String get walletMoveDeshieldTitle => 'Ce déplacement rend vos fonds publics';

  @override
  String get walletMoveDeshieldBody =>
      'Déplacer ces fonds vers une adresse publique les retire de votre solde protégé — le montant et votre adresse publique deviennent visibles publiquement sur la blockchain Zcash.';

  @override
  String get walletMoveWalletEnded =>
      'La session du portefeuille s\'est terminée. Fermez puis rouvrez pour réessayer.';

  @override
  String get walletMoveLoading => 'Préparation…';

  @override
  String get walletMovePreparing => 'Vérification du montant…';

  @override
  String get walletMoveSubmitting => 'Déplacement en cours…';

  @override
  String get walletMoveReviewButton => 'Vérifier';

  @override
  String get walletMoveCancel => 'Annuler';

  @override
  String get walletMoveReviewTitle => 'Vérifier le déplacement';

  @override
  String get walletMoveOwnAddressNote =>
      'Vous déplacez des fonds vers votre propre adresse publique. Vous pourrez protéger ces fonds à nouveau plus tard, mais ce déplacement reste inscrit publiquement de façon permanente.';

  @override
  String get walletMoveConfirmButton => 'Déplacer vers public';

  @override
  String get walletMoveBackButton => 'Retour';

  @override
  String get walletMoveDoneTitle => 'Déplacé vers public';

  @override
  String get walletMoveDoneBody =>
      'Vos fonds sont en cours de transfert vers votre adresse publique. La confirmation sur la chaîne arrivera sous peu.';

  @override
  String get walletMoveSavedTitle =>
      'Enregistré — le déplacement sera finalisé';

  @override
  String get walletMoveSavedBody =>
      'Ce déplacement est enregistré, et votre portefeuille l\'enverra lors d\'une prochaine synchronisation. Rien n\'a été perdu.';

  @override
  String get walletMoveAlreadyTitle => 'Déjà soumis';

  @override
  String get walletMoveAlreadyBody =>
      'Ces fonds ont déjà été soumis et sont en route vers votre adresse publique.';

  @override
  String get walletMoveFailedTitle => 'Impossible de terminer ce déplacement';

  @override
  String get walletMoveNothingTitle => 'Rien à déplacer pour l\'instant';

  @override
  String get walletMoveNothingBody =>
      'Vous n\'avez actuellement aucun solde protégé disponible à déplacer. Une fois les fonds confirmés, vous pourrez les déplacer vers votre adresse publique.';

  @override
  String get walletMoveNothingCatchingUpBody =>
      'Votre portefeuille rattrape encore son retard — tout ce que vous avez reçu deviendra disponible à déplacer une fois la synchronisation terminée.';

  @override
  String get walletMoveCouldNotLoad =>
      'Impossible de charger votre adresse publique. Réessayez.';

  @override
  String get walletMoveRetry => 'Réessayer';

  @override
  String get walletMoveClose => 'Fermer';

  @override
  String get walletSnapshotUnavailable =>
      'Impossible de lire le portefeuille pour le moment. Il se rafraîchira automatiquement.';

  @override
  String get walletBalanceStale =>
      'Impossible d\'actualiser — affichage de votre dernier solde connu.';

  @override
  String get walletSyncStartFailed =>
      'Impossible de démarrer la synchronisation. Nous continuons d\'essayer.';

  @override
  String get walletSyncRetry => 'Réessayer';

  @override
  String get walletSyncTryNow => 'Essayer maintenant';

  @override
  String get walletSyncIdle => 'Synchronisation pas encore démarrée';

  @override
  String get walletSyncIdleDetail =>
      'La synchronisation démarre automatiquement.';

  @override
  String get walletSyncDisabled => 'Synchronisation désactivée';

  @override
  String get walletSyncDisabledDetail =>
      'Activez la synchronisation dans les paramètres de cette application pour mettre à jour votre solde.';

  @override
  String get walletSyncExplainDisabled =>
      'La synchronisation est désactivée dans les paramètres de cette application. Vos fonds sont en sécurité. Votre solde et votre activité affichent le dernier état synchronisé et ne seront pas mis à jour tant que la synchronisation n\'est pas réactivée.';

  @override
  String get walletParkedSyncPausedNote =>
      'Votre portefeuille ne se synchronise pas, ces paiements ne seront donc pas envoyés tout seuls. Utilisez « Envoyer maintenant » pour en envoyer un vous-même.';

  @override
  String get walletSyncPausedMoneyNote =>
      'En pause jusqu\'à ce que votre portefeuille se synchronise à nouveau.';

  @override
  String walletSyncPausedJoin(String body, String note) {
    return '$body $note';
  }

  @override
  String get walletSyncStarting => 'Connexion…';

  @override
  String get walletSyncStartingDetail =>
      'Connexion au réseau Zcash et préparation de l\'analyse.';

  @override
  String get walletSyncConnecting => 'Connexion…';

  @override
  String walletSyncConnectingPercent(int percent) {
    return 'Connexion… $percent %';
  }

  @override
  String walletSyncScanning(int percent) {
    return 'Analyse en cours : $percent %';
  }

  @override
  String get walletSyncScanningEarly => 'Analyse en cours…';

  @override
  String get walletSyncSpendableReady =>
      'Les fonds sont prêts à être dépensés.';

  @override
  String get walletSyncCatchingUp =>
      'Rattrapage du réseau en cours — une synchronisation initiale complète peut prendre du temps. Vous pouvez continuer à utiliser l\'application en attendant';

  @override
  String walletSyncScanRemaining(String count) {
    return '$count blocs restants';
  }

  @override
  String get walletSyncUpToDate => 'À jour';

  @override
  String get walletSyncOffline => 'Hors ligne';

  @override
  String get walletSyncOfflineDetail =>
      'Les envois en file d\'attente restent enregistrés dans « Enregistré et en attente ».';

  @override
  String get walletSyncUnknown => 'Synchronisation…';

  @override
  String get walletSyncStalled => 'Synchronisation en pause';

  @override
  String get walletStallEndpoint =>
      'Impossible de joindre le réseau Zcash pour le moment. Nous continuons d\'essayer automatiquement — vérifiez votre connexion internet, ou le serveur est peut-être temporairement indisponible.';

  @override
  String get walletStallTor =>
      'Le chemin privé de votre application est indisponible, le portefeuille ne se connecte donc pas. Vérifiez les paramètres réseau de votre application ou désactivez le chemin privé. La synchronisation reprendra dès que le chemin sera de retour.';

  @override
  String get walletStallStorage =>
      'Le stockage de l\'appareil est saturé. Libérez de l\'espace pour reprendre la synchronisation.';

  @override
  String get walletStallReorg =>
      'La chaîne s\'est réorganisée ; vérification des blocs récents en cours.';

  @override
  String get walletStallInternal =>
      'Un problème local a interrompu la synchronisation. Si cela se reproduit, restaurez à partir de votre phrase de récupération.';

  @override
  String get walletStallEndpointMisbehaving =>
      'Ce serveur a envoyé des données qui ne peuvent pas être correctes, la synchronisation s\'est donc arrêtée. Ce n\'est pas un problème de connexion — passez à un autre serveur. Si tous les serveurs sont refusés, réanalysez l\'historique : le portefeuille conserve peut-être un enregistrement erroné d\'un serveur précédent.';

  @override
  String get walletStallBirthdayInFuture =>
      'Ce portefeuille est configuré pour démarrer à un bloc que ce serveur n\'a pas encore atteint. Vérifiez le bloc de départ configuré pour ce portefeuille, ou essayez un autre serveur.';

  @override
  String get walletStallStorageUnavailable =>
      'Synchronisation en pause sur cet appareil. Nouvelle tentative en cours.';

  @override
  String get walletStallUnknown =>
      'La synchronisation s\'est arrêtée pour une raison inconnue.';

  @override
  String get walletSyncBadgeHint =>
      'Afficher les détails de la synchronisation';

  @override
  String get walletSyncSheetClose => 'Fermer';

  @override
  String get walletSyncSheetProgress => 'Progression';

  @override
  String get walletSyncSheetBlocksLeft => 'Blocs restants';

  @override
  String get walletSyncSheetSyncedTo => 'Synchronisé jusqu\'au bloc';

  @override
  String walletSyncSheetBehindBy(int count, String blocks) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'En retard d\'au moins $blocks blocs',
      one: 'En retard d\'au moins 1 bloc',
    );
    return '$_temp0';
  }

  @override
  String get walletSyncExplainIdle =>
      'La synchronisation n\'a pas encore démarré — elle démarre automatiquement. Aucune action requise.';

  @override
  String get walletSyncExplainStartFailed =>
      'La synchronisation n\'a pas pu démarrer. Vos fonds sont en sécurité — le portefeuille ne vérifie simplement pas les nouvelles activités. Réessayez ci-dessous, ou rouvrez l\'application.';

  @override
  String get walletSyncExplainStarting =>
      'Le portefeuille contacte le réseau Zcash et se prépare à l\'analyse. Cela prend généralement quelques secondes.';

  @override
  String get walletSyncExplainConnecting =>
      'Établissement d\'une connexion au réseau Zcash.';

  @override
  String get walletSyncExplainScanning =>
      'Le portefeuille examine les blocs de la blockchain à la recherche de vos fonds. Votre solde et votre activité se mettent à jour au fur et à mesure que de nouvelles transactions sont trouvées — vous pouvez continuer à utiliser l\'application en attendant la fin de l\'opération.';

  @override
  String get walletSyncExplainUpToDate =>
      'Entièrement synchronisé avec le réseau Zcash. Votre solde et votre activité sont à jour.';

  @override
  String get walletSyncExplainStalled =>
      'La synchronisation a rencontré un problème et est en pause. Elle réessaie automatiquement.';

  @override
  String get walletSyncExplainStalledOffline =>
      'Impossible de joindre le réseau Zcash — c\'est normal si vous êtes hors ligne, ou le serveur est peut-être temporairement indisponible. Vos fonds sont en sécurité : le solde affiché correspond au dernier état synchronisé, et les envois en file d\'attente restent enregistrés dans « Enregistré et en attente ». La connexion réessaie automatiquement.';

  @override
  String get walletSyncExplainOffline =>
      'Aucune connexion réseau. Vos fonds sont en sécurité — le solde affiché correspond au dernier état synchronisé, et les envois en file d\'attente restent enregistrés dans « Enregistré et en attente ».';

  @override
  String get walletSyncExplainUnknown =>
      'Le portefeuille est en cours de synchronisation. Votre solde et votre activité se mettent à jour au fur et à mesure.';

  @override
  String get walletTorOff => 'Tor désactivé';

  @override
  String get walletTorBootstrapping => 'Démarrage du chemin privé…';

  @override
  String walletTorBootstrappingNamed(String transport) {
    return 'Démarrage de $transport…';
  }

  @override
  String get walletTorActive => 'Tor actif';

  @override
  String get walletTorActiveUnverified =>
      'Tor actif (environnement non vérifié)';

  @override
  String get walletTorActiveUnattested =>
      'Chemin privé utilisé (confidentialité non vérifiée)';

  @override
  String get walletTorFellBack =>
      'Tor indisponible — utilisation d\'une connexion directe';

  @override
  String get walletTorUnavailable => 'Chemin privé indisponible — non connecté';

  @override
  String walletTorUnavailableNamed(String transport) {
    return '$transport indisponible — non connecté';
  }

  @override
  String get walletTorUnanswered => 'Chemin privé connecté — rien ne revient';

  @override
  String get walletTorUnansweredUnattested =>
      'Chemin privé connecté — rien ne revient (confidentialité non vérifiée)';

  @override
  String walletTorUnansweredNamed(String transport) {
    return '$transport connecté — rien ne revient';
  }

  @override
  String get walletTorUnansweredDirect =>
      'Non privé (connexion directe de votre application) — rien ne revient';

  @override
  String walletTorUnansweredLinkable(String transport) {
    return 'Connecté via $transport — rien ne revient ; le proxy peut relier les connexions';
  }

  @override
  String get walletTorUnknown =>
      'Statut de Tor inconnu — à considérer comme non protégé';

  @override
  String walletBalanceHeaderAsOf(String height) {
    return 'Solde (au bloc $height)';
  }

  @override
  String walletBalanceHeaderAt(String time) {
    return 'Solde · $time';
  }

  @override
  String walletBalanceHeaderAsOfAt(String height, String time) {
    return 'Solde (au bloc $height, $time)';
  }

  @override
  String get walletSyncSheetConnection => 'Connexion';

  @override
  String get walletSyncSheetServer => 'Serveur';

  @override
  String walletSyncServerRowSemantics(String host) {
    return 'Serveur, $host, ouvre le sélecteur de serveur';
  }

  @override
  String get walletSyncServerSheetTitle => 'Serveur de synchronisation';

  @override
  String get walletSyncServerInUse => 'Utilisé';

  @override
  String get walletSyncServerAppDefault => 'Par défaut de l\'application';

  @override
  String get walletSyncServerCustom => 'Serveur personnalisé…';

  @override
  String get walletSyncServerCustomHint => 'https://hôte:port';

  @override
  String get walletSyncServerCheck => 'Vérifier le serveur';

  @override
  String get walletSyncServerChecking => 'Vérification…';

  @override
  String get walletSyncServerUse => 'Utiliser ce serveur';

  @override
  String get walletSyncServerSwitching => 'Changement en cours…';

  @override
  String get walletSyncServerContinue => 'Continuer';

  @override
  String get walletSyncServerCancel => 'Annuler';

  @override
  String get walletSyncServerTrustTitle => 'Faire confiance à ce serveur ?';

  @override
  String get walletSyncServerTrustNotice =>
      'Vous faites confiance à ce serveur pour indiquer votre solde et votre historique et relayer vos paiements. Il verra votre adresse IP sauf si Tor est actif, à peu près quand votre portefeuille a été créé, les adresses publiques que votre portefeuille vérifie, les transactions qu\'il consulte, et les transactions que vous envoyez.';

  @override
  String get walletSyncServerKeyLabel => 'Clé d\'accès (facultative)';

  @override
  String get walletSyncServerKeyHeaderLabel => 'En-tête de la clé';

  @override
  String get walletSyncServerKeyHeaderNeeded =>
      'Saisissez l\'en-tête attendu par votre serveur';

  @override
  String get walletSyncServerKeyInvalid =>
      'Cette clé ou cet en-tête ne peut pas être utilisé';

  @override
  String get walletSyncServerKeySaved => 'Clé enregistrée';

  @override
  String get walletSyncServerKeyShow => 'Afficher';

  @override
  String get walletSyncServerKeyHide => 'Masquer';

  @override
  String get walletSyncServerTrustNoticeKey =>
      'Votre clé vous identifie auprès de ce serveur. Elle peut relier vos paiements à votre portefeuille, même via Tor.';

  @override
  String get walletSyncServerSwitchNotice =>
      'Changer relance la synchronisation en cours. Votre solde et votre historique sont conservés. Les fonds peuvent apparaître en arrivée jusqu\'à ce que l\'analyse du nouveau serveur ait rattrapé son retard.';

  @override
  String get walletSyncServerSwitchNoticeAtTip =>
      'Changer reconnecte au nouveau serveur. Votre solde et votre historique sont conservés.';

  @override
  String get walletSyncServerUnreachable =>
      'Impossible de joindre ce serveur. Vérifiez l\'adresse — et si elle est correcte, soit ce serveur ne répond pas, soit votre application n\'arrive pas à l\'atteindre en ce moment. Réessayez, ou choisissez un autre serveur.';

  @override
  String get walletSyncServerUnreachableOffered =>
      'Impossible de joindre ce serveur. Le portefeuille ne peut pas savoir si ce serveur ne répond pas ou si votre application n\'arrive pas à l\'atteindre en ce moment. Choisissez un autre serveur, ou réessayez plus tard.';

  @override
  String get walletSyncServerWrongNetwork =>
      'Ce serveur est sur un autre réseau Zcash.';

  @override
  String get walletSyncServerInvalidUrl =>
      'Cela ne ressemble pas à une adresse de serveur. Utilisez https://hôte:port.';

  @override
  String get walletSyncServerNotOffered =>
      'Ce serveur n\'est pas proposé par cette application.';

  @override
  String get walletSyncServerBusy =>
      'Le portefeuille est occupé pour le moment. Réessayez dans un instant.';

  @override
  String walletSyncServerFallbackNotOffered(String host) {
    return 'Le serveur choisi n\'est plus proposé par cette application. $host est utilisé.';
  }

  @override
  String walletSyncServerFallbackUnreadable(String host) {
    return 'Le choix de serveur mémorisé n\'a pas pu être lu. $host est utilisé.';
  }

  @override
  String walletSyncServerSwitchFailedRecovered(String host) {
    return 'Changement impossible — $host reste utilisé.';
  }

  @override
  String get walletTransportExplainDirect =>
      'Le trafic du portefeuille se connecte directement au serveur. Le serveur peut voir votre adresse IP.';

  @override
  String get walletTransportExplainTor =>
      'Le trafic du portefeuille est acheminé via le réseau Tor, ce qui masque votre adresse IP au serveur.';

  @override
  String get walletTransportExplainBootstrapping =>
      'Le chemin privé de votre application est en cours de démarrage. Le trafic du portefeuille attend qu\'il soit prêt avant de se connecter.';

  @override
  String walletTransportExplainBootstrappingNamed(String transport) {
    return '$transport est en cours de démarrage. Le trafic du portefeuille attend qu\'il soit prêt avant de se connecter.';
  }

  @override
  String get walletTransportExplainFellBack =>
      'Tor n\'a pas pu être joint, le trafic est donc repassé en connexion directe. Le serveur peut voir votre adresse IP.';

  @override
  String get walletTransportExplainUnavailable =>
      'Le chemin privé de votre application est indisponible, le portefeuille ne se connecte donc pas. Désactivez le chemin privé ou vérifiez les paramètres réseau de votre application.';

  @override
  String walletTransportExplainUnavailableNamed(String transport) {
    return '$transport est indisponible, le portefeuille ne se connecte donc pas. Désactivez-le ou vérifiez les paramètres réseau de votre application.';
  }

  @override
  String get walletTransportExplainUnanswered =>
      'Le chemin privé a accepté la connexion, mais rien n\'est revenu depuis une minute. Cela peut venir du chemin ou du serveur du portefeuille — le portefeuille ne peut pas le savoir. Il continue d\'essayer ; si cela persiste, essayez un autre serveur ou vérifiez les paramètres réseau de votre application.';

  @override
  String walletTransportExplainUnansweredNamed(String transport) {
    return '$transport a accepté la connexion, mais rien n\'est revenu depuis une minute. Cela peut venir du chemin ou du serveur du portefeuille — le portefeuille ne peut pas le savoir. Il continue d\'essayer ; si cela persiste, essayez un autre serveur ou vérifiez les paramètres réseau de votre application.';
  }

  @override
  String get walletTransportExplainUnansweredDirect =>
      'Le trafic du portefeuille se connecte directement au serveur. Le serveur peut voir votre adresse IP. La connexion a été acceptée, mais rien n\'est revenu depuis une minute. Cela peut venir du chemin ou du serveur du portefeuille — le portefeuille ne peut pas le savoir. Il continue d\'essayer ; si cela persiste, essayez un autre serveur ou vérifiez les paramètres réseau de votre application.';

  @override
  String get walletTransportExplainUnansweredUnverified =>
      'La confidentialité de cette connexion ne peut pas être vérifiée — considérez-la comme non privée. La connexion a été acceptée, mais rien n\'est revenu depuis une minute. Cela peut venir du chemin ou du serveur du portefeuille — le portefeuille ne peut pas le savoir. Il continue d\'essayer ; si cela persiste, essayez un autre serveur ou vérifiez les paramètres réseau de votre application.';

  @override
  String get walletTransportExplainUnverified =>
      'La confidentialité de cette connexion ne peut pas être vérifiée — considérez-la comme non privée.';

  @override
  String get walletTransportExplainHostProxy =>
      'Le trafic du portefeuille est acheminé via le transport de confidentialité de cette application, ce qui masque votre adresse IP au serveur.';

  @override
  String get walletOnboardingWelcomeTitle => 'Configurer votre portefeuille';

  @override
  String get walletOnboardingWelcomeBody =>
      'Créez un nouveau portefeuille pour recevoir et conserver du ZEC. Nous générerons une phrase de récupération et vous guiderons pour la sauvegarder avant l\'arrivée de tout fonds — ainsi rien n\'est jamais exposé sans sauvegarde.';

  @override
  String get walletCreateButton => 'Créer un nouveau portefeuille';

  @override
  String get walletRestoreButton =>
      'Restaurer à partir d\'une phrase de récupération';

  @override
  String get walletWatchOnlyButton =>
      'Consulter un portefeuille (lecture seule)';

  @override
  String get walletWatchOnlyTitle => 'Consulter un portefeuille';

  @override
  String get walletWatchOnlyBody =>
      'Collez une clé de consultation pour consulter un portefeuille sans ses clés de dépense. Vous verrez son solde et son historique, mais vous ne pourrez pas envoyer de fonds. Choisissez la date de début approximative du portefeuille afin que nous sachions jusqu\'où remonter.';

  @override
  String get walletWatchOnlyKeyLabel => 'Clé de consultation';

  @override
  String get walletWatchOnlyKeyHint => 'uview1…';

  @override
  String get walletWatchOnlyScanTooltip =>
      'Scanner un code QR de clé de consultation';

  @override
  String get walletWatchOnlyScanTitle => 'Scanner une clé de consultation';

  @override
  String get walletWatchOnlyScanInstruction =>
      'Pointez votre caméra vers le code QR de la clé de consultation.';

  @override
  String get walletWatchOnlyScanCameraUnavailable =>
      'Caméra indisponible. Collez la clé manuellement à la place.';

  @override
  String get walletWatchOnlyScanManualEntry => 'Coller à la place';

  @override
  String get walletWatchOnlyScanHint =>
      'Ou appuyez sur le bouton de scan pour lire un code QR de clé de consultation.';

  @override
  String get walletWatchOnlyScanFilled => 'Clé de consultation scannée.';

  @override
  String get walletWatchOnlyBirthdayTitle => 'Date de début du portefeuille';

  @override
  String walletWatchOnlyBirthdayChosen(String date) {
    return 'Analyse à partir du $date — les fonds reçus avant cette date n\'apparaîtront pas. Portefeuille plus ancien ? Choisissez une date antérieure.';
  }

  @override
  String get walletWatchOnlyBirthdayPick =>
      'Choisir la date de début du portefeuille';

  @override
  String get walletWatchOnlyBirthdayChange => 'Modifier la date';

  @override
  String get walletWatchOnlySubmit => 'Consulter ce portefeuille';

  @override
  String get walletWatchOnlyBack => 'Retour';

  @override
  String get walletWatchOnlyFaultInvalidKey =>
      'Cela ne ressemble pas à une clé de consultation valide. Vérifiez-la et réessayez.';

  @override
  String get walletWatchOnlyFaultNetworkMismatch =>
      'Cette clé de consultation est destinée à un autre réseau. Elle ne peut pas être utilisée ici.';

  @override
  String get walletWatchOnlyFaultAlreadyExists =>
      'Un portefeuille existe déjà sur cet appareil. Revenez en arrière et ouvrez-le à la place.';

  @override
  String get walletWatchOnlyFaultBirthdayTooRecent =>
      'Cette date de début est trop récente. Choisissez une date antérieure.';

  @override
  String get walletRestoreTitle => 'Restaurer votre portefeuille';

  @override
  String get walletRestoreBody =>
      'Saisissez votre phrase de récupération pour restaurer votre portefeuille — tapez ou collez les mots dans l\'ordre, séparés par des espaces. Phrases standard uniquement : si votre portefeuille utilisait une phrase secrète supplémentaire (un « 25e mot »), cette application ne peut pas encore la restaurer — vous verriez un portefeuille vide, et non une erreur.';

  @override
  String get walletRestorePhraseHint => 'mot un  mot deux  mot trois  …';

  @override
  String walletRestoreWordCount(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$count mots',
      one: '1 mot',
      zero: 'Aucun mot pour l\'instant',
    );
    return '$_temp0';
  }

  @override
  String get walletRestoreLengthHint =>
      'les phrases de récupération comptent 12, 15, 18, 21 ou 24 mots';

  @override
  String walletRestoreSomeWordsInvalid(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$count mots ne sont pas des mots de récupération — corrigez ceux en surbrillance',
      one:
          '1 mot n\'est pas un mot de récupération — corrigez celui en surbrillance',
    );
    return '$_temp0';
  }

  @override
  String walletRestorePillSemantics(int index, String word) {
    return 'mot $index : $word';
  }

  @override
  String walletRestorePillSemanticsInvalid(int index) {
    return 'mot $index : pas un mot de récupération';
  }

  @override
  String walletRestoreRemoveWord(int index) {
    return 'Supprimer le mot $index';
  }

  @override
  String get walletRestoreSubmit => 'Restaurer le portefeuille';

  @override
  String get walletRestoreBack => 'Retour';

  @override
  String get walletRestoreBirthdayTitle => 'Jusqu\'où remonter pour l\'analyse';

  @override
  String get walletRestoreBirthdayNone =>
      'Nous analyserons tout votre historique — plus lent, mais rien n\'est manqué.';

  @override
  String walletRestoreBirthdayChosen(String date) {
    return 'Analyse à partir du $date — les fonds reçus avant cette date n\'apparaîtront pas. Portefeuille plus ancien ? Choisissez une date antérieure, ou analysez tout l\'historique.';
  }

  @override
  String get walletRestoreBirthdayPick => 'Choisir une date';

  @override
  String get walletRestoreBirthdayChange => 'Changer la date';

  @override
  String get walletRestoreBirthdayClear => 'Analyser tout l\'historique';

  @override
  String walletRestoreFaultInvalidWord(int index) {
    return 'Le mot $index n\'est pas un mot de récupération. Vérifiez les fautes de frappe dans votre phrase, puis réessayez.';
  }

  @override
  String get walletRestoreFaultInvalidPhrase =>
      'Cette phrase de récupération n\'est pas valide. Vérifiez les mots et leur ordre, puis réessayez.';

  @override
  String get walletRestoreFaultSeedMismatch =>
      'Cette phrase ne correspond pas au portefeuille présent sur cet appareil. Vérifiez-la et réessayez.';

  @override
  String get walletRestoreFaultAlreadyExists =>
      'Un portefeuille existe déjà sur cet appareil. Revenez en arrière pour l\'ouvrir.';

  @override
  String get walletRestoreFaultBirthdayTooRecent =>
      'Cette date est trop récente. Choisissez une date antérieure, ou analysez tout l\'historique.';

  @override
  String get walletGeneratingLabel => 'Création de votre portefeuille…';

  @override
  String get walletOpeningLabel => 'Ouverture de votre portefeuille…';

  @override
  String get walletBackupTitle => 'Sauvegardez votre phrase de récupération';

  @override
  String get walletBackupBody =>
      'Ces mots constituent le SEUL moyen de récupérer votre portefeuille et vos fonds. Notez-les dans l\'ordre et conservez-les dans un endroit sûr et privé. Ne les partagez jamais et ne les stockez jamais en ligne — toute personne disposant de ces mots peut s\'emparer de vos fonds.';

  @override
  String get walletBackupSecureNoteAndroid =>
      'Les captures d\'écran sont désactivées sur cet écran.';

  @override
  String get walletBackupSecureNoteOther =>
      'Assurez-vous que personne ne peut voir votre écran.';

  @override
  String get walletBackupReveal => 'Afficher la phrase de récupération';

  @override
  String get walletBackupRevealing =>
      'Préparation de votre phrase de récupération…';

  @override
  String get walletBackupRevealFailed =>
      'Impossible d\'afficher votre phrase de récupération pour le moment. Assurez-vous que votre appareil est déverrouillé, puis réessayez.';

  @override
  String get walletBackupRetryReveal => 'Réessayer';

  @override
  String get walletBackupReauthFailed =>
      'Impossible de vérifier votre identité. Veuillez réessayer.';

  @override
  String get walletBackupConfirmCheckbox =>
      'J\'ai noté ma phrase de récupération et je l\'ai rangée en lieu sûr.';

  @override
  String get walletBackupContinue => 'Continuer';

  @override
  String get walletBackupSaveFailed =>
      'Impossible d\'enregistrer votre confirmation. Veuillez réessayer.';

  @override
  String get walletBackupStartOver => 'Recommencer';

  @override
  String get walletBackupStartOverConfirmTitle =>
      'Recommencer sans ce portefeuille ?';

  @override
  String get walletBackupStartOverConfirmBody =>
      'Cette action supprime ce portefeuille de l\'appareil et vous ramène au début. Rien ne peut être déposé via cette application avant la fin de la configuration.\n\nSi ce portefeuille a un jour contenu des fonds — ou s\'il a été restauré à partir d\'une phrase de récupération — seule cette phrase peut le restaurer.';

  @override
  String get walletBackupStartOverConfirm => 'Supprimer et recommencer';

  @override
  String get walletBackupStartOverKeep => 'Conserver ce portefeuille';

  @override
  String get walletBackupSectionTitle => 'Phrase de récupération';

  @override
  String get walletBackupTileTitle =>
      'Sauvegardez votre phrase de récupération';

  @override
  String get walletBackupTileSubtitle =>
      'Afficher les mots qui permettent de récupérer votre portefeuille et vos fonds.';

  @override
  String get walletBackupScreenTitle => 'Phrase de récupération';

  @override
  String get walletBackupDone => 'Terminé';

  @override
  String get walletBackupManagedTitle =>
      'Aucune phrase de récupération distincte';

  @override
  String get walletBackupManagedBody =>
      'Ce portefeuille a été configuré à partir de votre compte dans l\'application qui l\'a installé, il n\'a donc pas de phrase de récupération qui lui soit propre. Vos fonds sont récupérés en même temps que ce compte — utilisez sa sauvegarde pour les protéger.';

  @override
  String get walletExportViewingKeyTitle => 'Exporter la clé de consultation';

  @override
  String get walletExportViewingKeyTileTitle =>
      'Exporter la clé de consultation';

  @override
  String get walletExportViewingKeyTileSubtitle =>
      'Partagez une copie en lecture seule de votre portefeuille : elle peut voir votre historique, mais ne peut pas dépenser vos fonds.';

  @override
  String get walletExportViewingKeyWarning =>
      'Cette clé permet à quiconque la détient de voir tout ce que ce portefeuille a déjà reçu et envoyé — ainsi que tout ce qu\'il recevra et enverra à l\'avenir. Elle ne permet pas de dépenser vos fonds et ne permet pas de récupérer votre portefeuille. Ne la partagez qu\'avec une personne de confiance à qui vous souhaitez montrer l\'intégralité de votre historique, comme un comptable ou votre propre second appareil. Le seul moyen de révoquer ce partage par la suite est de transférer vos fonds vers un nouveau portefeuille.';

  @override
  String get walletExportViewingKeyWarningWatchOnly =>
      'Cette clé permet à quiconque la détient de voir tout ce que ce portefeuille a déjà reçu et envoyé — ainsi que tout ce qu\'il recevra et enverra à l\'avenir. Elle ne permet pas de dépenser vos fonds et ne permet pas de récupérer votre portefeuille. Ne la partagez qu\'avec une personne de confiance à qui vous souhaitez montrer l\'intégralité de votre historique, comme un comptable ou votre propre second appareil. Une fois ce partage effectué, il ne peut plus être révoqué.';

  @override
  String get walletExportViewingKeyReveal => 'Afficher la clé de consultation';

  @override
  String get walletExportViewingKeyRetry => 'Réessayer';

  @override
  String get walletExportViewingKeyRevealing =>
      'Préparation de votre clé de consultation…';

  @override
  String get walletExportViewingKeyFailed =>
      'Impossible d\'afficher votre clé de consultation pour le moment. Réessayez dans un instant.';

  @override
  String get walletExportViewingKeyQrLabel =>
      'Code QR de la clé de consultation';

  @override
  String get walletExportViewingKeyCopy => 'Copier la clé de consultation';

  @override
  String get walletExportViewingKeyCopied => 'Clé de consultation copiée';

  @override
  String get walletExportViewingKeyDone => 'Terminé';

  @override
  String get walletExportViewingKeySecureNoteAndroid =>
      'Les captures d\'écran sont désactivées sur cet écran.';

  @override
  String get walletExportViewingKeySecureNoteOther =>
      'Assurez-vous que personne ne peut voir votre écran.';

  @override
  String get walletWatchOnlySectionTitle =>
      'À propos de ce portefeuille en lecture seule';

  @override
  String get walletWatchOnlyAboutBody =>
      'Ceci est un portefeuille en lecture seule. Il a été configuré à partir d\'une clé de consultation : il peut donc voir votre solde et votre historique, mais ne détient aucune clé de dépense — il n\'y a rien à sauvegarder ici, et il ne peut pas envoyer de fonds.';

  @override
  String get walletWatchOnlyBadge => 'Lecture seule';

  @override
  String get walletOnboardingFailedTitle =>
      'La configuration du portefeuille n\'a pas pu se terminer';

  @override
  String get walletOnboardingRetry => 'Réessayer';

  @override
  String get walletOnboardingFailedDeviceLocked =>
      'Le stockage sécurisé de votre téléphone ne répond pas. Déverrouillez votre appareil et réessayez. Si le problème persiste, redémarrez votre téléphone.';

  @override
  String get walletOnboardingFailedAlreadyOpen =>
      'Ce portefeuille est ouvert dans une autre fenêtre ou application, ou termine encore une opération précédente. Fermez toute autre fenêtre l\'utilisant — ou patientez un instant — puis réessayez.';

  @override
  String get walletOnboardingFailedNeedsRecovery =>
      'La clé sécurisée de ce portefeuille n\'est plus disponible, il ne peut donc pas être ouvert sur cet appareil. Vos fonds sont en sécurité — restaurez à partir de votre phrase de récupération pour les récupérer.';

  @override
  String get walletOnboardingFailedRestoreAction =>
      'Restaurer à partir de la phrase de récupération';

  @override
  String get walletOnboardingRecoverConfirmTitle =>
      'Restaurer ce portefeuille ?';

  @override
  String get walletOnboardingRecoverConfirmBody =>
      'Assurez-vous d\'avoir votre phrase de récupération avant de continuer — vous en aurez besoin à l\'écran suivant pour récupérer vos fonds. Vos fonds sont en sécurité sur la blockchain et contrôlés par cette phrase. Cette opération supprime de cet appareil les données de portefeuille illisibles afin qu\'il puisse être reconstruit.';

  @override
  String get walletOnboardingRecoverConfirmCancel => 'Annuler';

  @override
  String get walletOnboardingFailedStorageFull =>
      'Il n\'y a pas assez d\'espace libre pour configurer votre portefeuille. Libérez de l\'espace et réessayez.';

  @override
  String get walletOnboardingFailedNoVault =>
      'Cet appareil ne dispose d\'aucun stockage de clés sécurisé, le portefeuille ne peut donc pas protéger votre phrase de récupération ici.';

  @override
  String get walletOnboardingFailedNetwork =>
      'Impossible de joindre le réseau pendant la configuration. Vérifiez votre connexion et réessayez.';

  @override
  String get walletOnboardingFailedInterruptedSetup =>
      'La configuration du portefeuille ne s\'est pas terminée. Réessayez pour la terminer — rien n\'a été perdu.';

  @override
  String get walletOnboardingFailedUnknown =>
      'Une erreur s\'est produite lors de la configuration de votre portefeuille. Réessayez.';

  @override
  String get walletOnboardingFailedConfiguration =>
      'La configuration du portefeuille de cette application est incorrecte, le portefeuille ne peut donc pas démarrer. Réessayer n\'y changera rien — merci de signaler ce problème au développeur de l\'application. Vos fonds ne sont pas affectés.';

  @override
  String get walletSendButton => 'Envoyer';

  @override
  String get walletSendSyncNotRunning =>
      'La synchronisation n\'est pas en cours — votre solde disponible ne pourra pas se mettre à jour';

  @override
  String get walletSendWaitingForFunds =>
      'Synchronisation encore en cours — vous pourrez envoyer dès que vous aurez un solde disponible';

  @override
  String get walletSendNoSpendableYet =>
      'Aucun solde disponible pour l\'instant';

  @override
  String get walletSendSyncUnavailable =>
      'Vous pourrez envoyer une fois la synchronisation reprise';

  @override
  String get walletSendTitle => 'Envoyer';

  @override
  String get walletSendUnavailable =>
      'Votre portefeuille n\'est pas prêt pour le moment. Revenez en arrière et réessayez.';

  @override
  String get walletSendWatchOnly =>
      'Ce portefeuille est en lecture seule. Il peut afficher le solde et recevoir des paiements, mais il ne détient aucune clé de dépense — il ne peut donc pas envoyer.';

  @override
  String get walletSendExpiredTitle => 'Cette demande de paiement a expiré';

  @override
  String get walletSendExpiredBody =>
      'L\'écran d\'envoi a mis plus de cinq secondes à s\'ouvrir ; l\'application a donc été informée que rien n\'a été envoyé. Cette réponse est définitive : cette demande ne peut pas être payée d\'ici. Pour payer, recommencez depuis l\'application.';

  @override
  String get walletSendFaultWatchOnly =>
      'Ce portefeuille est en lecture seule — il ne détient aucune clé de dépense, il ne peut donc pas envoyer.';

  @override
  String walletSendAvailable(String amount) {
    return 'Disponible à envoyer : $amount ZEC';
  }

  @override
  String walletSendAvailableCatchingUp(String amount) {
    return 'Disponible à envoyer : $amount ZEC — votre solde rattrape encore son retard';
  }

  @override
  String get walletSendRecipientLabel => 'Adresse du destinataire';

  @override
  String get walletSendRecipientHint =>
      'Adresse Zcash (commence par u, z ou t)';

  @override
  String get walletSendRecipientLocked =>
      'Le destinataire ne peut pas être modifié ici';

  @override
  String get walletSendAmountLabel => 'Montant (ZEC)';

  @override
  String get walletSendAmountHint => '0.00';

  @override
  String get walletSendMemoLabel => 'Mémo (facultatif)';

  @override
  String get walletSendMemoHint =>
      'Livré uniquement aux destinataires protégés (privés)';

  @override
  String get walletSendMemoTransparentDisabled =>
      'Les mémos nécessitent un destinataire protégé. Cette adresse publique ne peut pas en recevoir.';

  @override
  String get walletSendMemoMachineDisabled =>
      'Ce paiement porte déjà une référence de l\'application, il ne peut donc pas contenir aussi une note écrite.';

  @override
  String get walletSendMachineMemoTitle => 'L\'application joint une référence';

  @override
  String walletSendMachineMemoPurpose(String purpose) {
    return 'Elle indique que c\'est pour : $purpose';
  }

  @override
  String get walletSendMachineMemoLimit =>
      'Elle reste attachée à la transaction et ne pourra pas être retirée. Le portefeuille ne peut pas vérifier son contenu.';

  @override
  String get walletSendRecipientShielded => 'Protégée · privée';

  @override
  String get walletSendRecipientTransparent => 'Publique';

  @override
  String get walletSendRecipientInvalid =>
      'Cela ne ressemble pas à une adresse Zcash valide.';

  @override
  String get walletSendRecipientWrongNetwork =>
      'Cette adresse appartient à un réseau Zcash différent.';

  @override
  String get walletSendReviewButton => 'Vérifier le paiement';

  @override
  String get walletSendQueueButton => 'Mettre en file pour envoyer plus tard';

  @override
  String get walletSendQueueHint =>
      'Un paiement en file d\'attente se trouve dans « Enregistré et en attente », où vous pouvez l\'envoyer ou l\'annuler. Ses frais de réseau sont calculés au moment de l\'envoi.';

  @override
  String get walletSendPreparing => 'Préparation de votre paiement…';

  @override
  String get walletSendSubmitting => 'Envoi en cours…';

  @override
  String get walletSendQueuing => 'Mise en file d\'attente…';

  @override
  String get walletSendReviewTitle => 'Confirmer le paiement';

  @override
  String get walletSendTotalLabel => 'Total';

  @override
  String get walletSendFeeLabel => 'Frais de réseau';

  @override
  String get walletSendChangeLabel => 'Monnaie rendue';

  @override
  String get walletSendDeshieldTitle => 'Ce paiement n\'est pas privé';

  @override
  String get walletSendDeshieldBody =>
      'Il est envoyé vers une adresse publique, le montant et le destinataire seront donc visibles publiquement sur la blockchain Zcash.';

  @override
  String get walletSendPublicAckLabel =>
      'Je comprends que ce paiement sera public.';

  @override
  String get walletSendConfirmButton => 'Envoyer maintenant';

  @override
  String get walletSendBackButton => 'Retour';

  @override
  String get walletSendSelfSendNote =>
      'Vous envoyez vers votre propre portefeuille. Les frais de réseau s\'appliquent tout de même.';

  @override
  String get walletSendLargeConfirmTitle => 'Envoyer un montant important ?';

  @override
  String get walletSendLargeConfirmNearTotal =>
      'Il s\'agit de la quasi-totalité de votre solde. Un paiement envoyé ne peut pas être annulé.';

  @override
  String get walletSendLargeConfirmOverThreshold =>
      'Il s\'agit d\'un paiement important. Un paiement envoyé ne peut pas être annulé.';

  @override
  String get walletSendLargeConfirmBoth =>
      'Il s\'agit d\'un paiement important — la quasi-totalité de votre solde. Un paiement envoyé ne peut pas être annulé.';

  @override
  String walletSendLargeConfirmAction(String amount) {
    return 'Envoyer $amount';
  }

  @override
  String get walletSendLargeConfirmCancel => 'Revenir en arrière';

  @override
  String get walletSendSentTitle => 'Paiement envoyé';

  @override
  String get walletSendSentBody =>
      'Votre paiement a été diffusé sur le réseau.';

  @override
  String get walletSendSavedTitle => 'Enregistré — l\'envoi sera finalisé';

  @override
  String get walletSendSavedBody =>
      'Votre paiement n\'a pas pu partir pour le moment ; il est donc enregistré, et votre portefeuille l\'enverra lors d\'une prochaine synchronisation. Rien n\'est perdu.';

  @override
  String get walletSendKeptTitle => 'Enregistrée';

  @override
  String get walletSendKeptBody =>
      'Votre portefeuille a conservé cette transaction, mais ne s\'est pas engagé à l\'envoyer de lui-même. Consultez Activité pour voir où elle en est.';

  @override
  String get walletSendPartialBody =>
      'Une partie de votre paiement est partie ; votre portefeuille terminera le reste lors d\'une prochaine synchronisation. Rien n\'est perdu.';

  @override
  String get walletSendInMotionTitle => 'Paiement en cours';

  @override
  String get walletSendInMotionBody =>
      'Votre paiement a débuté et transite par une adresse à usage unique contrôlée par votre portefeuille. Ne l\'envoyez pas de nouveau. S\'il ne se termine pas, vous pouvez récupérer les fonds depuis l\'écran de votre portefeuille.';

  @override
  String get walletSendAlreadyTitle => 'Déjà soumis';

  @override
  String get walletSendAlreadyBody =>
      'Ce paiement a déjà été soumis — il ne sera pas envoyé deux fois.';

  @override
  String get walletSendFailedTitle => 'Impossible de finaliser le paiement';

  @override
  String get walletSendFailedBody =>
      'Une erreur s\'est produite lors de la finalisation de ce paiement et rien n\'a été envoyé. Vous pouvez réessayer.';

  @override
  String get walletSendTryAgain => 'Réessayer';

  @override
  String get walletSendDone => 'Terminé';

  @override
  String get walletSendAnother => 'Envoyer un autre paiement';

  @override
  String get walletSendQueuedTitle => 'En file d\'attente pour envoi';

  @override
  String get walletSendQueuedBody =>
      'Ce paiement est enregistré. Vous le retrouverez dans « Enregistré et en attente », où vous pouvez l\'envoyer maintenant ou l\'annuler.';

  @override
  String walletSendFaultInsufficient(String available, String required) {
    return 'Solde disponible insuffisant — vous avez $available ZEC et ce paiement nécessite $required ZEC.';
  }

  @override
  String get walletSendFaultNetworkUpgrade =>
      'Le réseau Zcash a été mis à niveau et cette application doit être mise à jour avant de pouvoir envoyer. Vos fonds sont en sécurité.';

  @override
  String get walletSyncUpToDateLimited =>
      'À jour dans la limite de ce que cette version peut lire';

  @override
  String get walletSyncExplainUpToDateLimited =>
      'Le réseau Zcash a été mis à niveau. Cette version a analysé tout ce qu\'elle peut lire, mais des blocs plus récents peuvent contenir des fonds qu\'elle ne peut pas encore afficher, et les mémos des paiements récents sont indisponibles. Mettez l\'application à jour pour tout voir.';

  @override
  String get walletSyncUpToDateDegraded =>
      'À jour, mais ce serveur ne dessert pas tous les pools';

  @override
  String get walletSyncExplainUpToDateDegraded =>
      'Ce serveur refuse, retient ou signale incorrectement l\'un des pools protégés de Zcash. Les fonds reçus dans ce pool ne peuvent pas être dépensés via ce serveur, et le solde affiché est un minimum. Passez à un autre serveur pour les utiliser — ce n\'est pas un problème de connexion.';

  @override
  String walletSyncPoolUnsupported(String pool) {
    return '$pool : ce serveur refuse de le servir';
  }

  @override
  String walletSyncPoolWithheld(String pool) {
    return '$pool : ce serveur en retient une partie';
  }

  @override
  String walletSyncPoolHeightViolation(String pool) {
    return '$pool : ce serveur le signale incorrectement';
  }

  @override
  String walletSyncPoolUnknown(String pool) {
    return '$pool : on ignore si ce serveur le sert';
  }

  @override
  String get walletPoolSapling => 'Sapling';

  @override
  String get walletPoolOrchard => 'Orchard';

  @override
  String get walletPoolIronwood => 'Ironwood';

  @override
  String get walletSyncEndpointBehind =>
      'À jour avec ce serveur, mais le serveur est en retard sur le réseau';

  @override
  String get walletSyncExplainEndpointBehind =>
      'La chaîne de ce serveur s\'arrête à un bloc que le réseau avait déjà dépassé avant la compilation de cette version de l\'application ; votre solde n\'est donc à jour que jusqu\'à ce bloc. Les nouveaux paiements reçus peuvent ne pas encore apparaître, et un paiement envoyé d\'ici peut ne pas aboutir. Passez à un autre serveur pour rattraper le retard — ce n\'est pas un problème de connexion.';

  @override
  String get walletParkedBlockedByNetworkUpgrade =>
      'En attente d\'une mise à jour de l\'application — vos fonds sont en sécurité et rien n\'a été envoyé.';

  @override
  String get walletParkedBlockedByServerSilent =>
      'En attente d\'un serveur qui indique la version du réseau — changez de serveur. Vos fonds sont en sécurité et rien n\'a été envoyé.';

  @override
  String get walletParkedBlockedByServerSilentClock =>
      'En attente d\'un serveur qui indique la version du réseau. Si la date et l\'heure de cet appareil sont incorrectes, corrigez-les d\'abord, puis changez de serveur. Vos fonds sont en sécurité et rien n\'a été envoyé.';

  @override
  String get walletSyncUnverified =>
      'À jour, mais ce serveur n\'indique pas la version du réseau';

  @override
  String walletSyncGraceLeftHours(int hours) {
    String _temp0 = intl.Intl.pluralLogic(
      hours,
      locale: localeName,
      other:
          'L\'envoi fonctionne encore pendant environ $hours heures — changez ensuite de serveur.',
      one:
          'L\'envoi fonctionne encore pendant environ 1 heure — changez ensuite de serveur.',
      zero:
          'L\'envoi fonctionne encore pendant moins d\'une heure — changez ensuite de serveur.',
    );
    return '$_temp0';
  }

  @override
  String walletSyncGraceLeftBlocks(String blocks) {
    return 'L\'envoi fonctionne encore pendant environ $blocks blocs — changez ensuite de serveur.';
  }

  @override
  String walletSyncGraceEndedBlocks(String blocks) {
    return 'Ce serveur n\'a pas indiqué la version du réseau depuis $blocks blocs, cette application ne peut donc pas confirmer qu\'un envoi est sûr. Passez à un autre serveur.';
  }

  @override
  String get walletSyncGraceEndedClock =>
      'Ce serveur n\'a pas indiqué la version du réseau depuis un jour, cette application ne peut donc pas confirmer qu\'un envoi est sûr. Si la date et l\'heure de cet appareil sont incorrectes, corrigez-les d\'abord, puis passez à un serveur qui indique la version du réseau.';

  @override
  String get walletSyncGraceNeverConfirmed =>
      'Ce serveur n\'a jamais indiqué la version du réseau, cette application ne peut donc pas confirmer qu\'un envoi est sûr. Passez à un autre serveur.';

  @override
  String get walletSyncExplainUnverified =>
      'Ce serveur ne dit pas sur quelle version du réseau Zcash il se trouve, cette application ne peut donc pas confirmer qu\'un paiement qu\'elle signe sera accepté. Votre solde est à jour. Passez à un autre serveur — ce n\'est pas un problème de connexion.';

  @override
  String get walletSyncExplainUnverifiedStreak =>
      'Ce serveur ne dit pas sur quelle version du réseau Zcash il se trouve, cette application ne peut donc pas confirmer qu\'un paiement qu\'elle signe sera accepté. Il a aussi continué à servir des blocs que ce portefeuille a ensuite dû annuler, votre solde n\'est donc peut-être pas à jour. Passez à un autre serveur — ce n\'est pas un problème de connexion.';

  @override
  String get walletSyncUnverifiedStreakDetail =>
      'Ce serveur continue aussi à servir des blocs que ce portefeuille doit ensuite annuler — changez de serveur.';

  @override
  String get walletSendFaultInsufficientCatchingUp =>
      'Votre solde rattrape encore son retard — d\'autres fonds pourraient devenir disponibles à mesure que le portefeuille se synchronise.';

  @override
  String walletSendFaultInsufficientPending(String pending) {
    return '$pending ZEC sont encore en cours d\'arrivée et seront disponibles une fois que le portefeuille aura rattrapé son retard.';
  }

  @override
  String get walletSendFaultAmountEmpty => 'Saisissez un montant à envoyer.';

  @override
  String get walletSendFaultAmountNotANumber =>
      'Saisissez le montant sous forme de nombre, par exemple 0.25.';

  @override
  String get walletSendFaultAmountDecimals =>
      'Le ZEC comporte au maximum 8 décimales.';

  @override
  String get walletSendFaultAmountNotPositive =>
      'Saisissez un montant supérieur à zéro.';

  @override
  String get walletSendFaultAmountOutOfRange =>
      'Ce montant dépasse l\'offre totale de ZEC.';

  @override
  String walletSendFaultOverCeiling(String limit) {
    return 'Cette application limite actuellement les envois à $limit ZEC.';
  }

  @override
  String get walletSendFaultAddressInvalid =>
      'Cela ne ressemble pas à une adresse Zcash valide pour ce réseau. Vérifiez-la et réessayez.';

  @override
  String get walletSendFaultMemoToTransparent =>
      'Ce destinataire ne peut pas recevoir de mémo. Supprimez le mémo, ou envoyez vers une adresse protégée (privée).';

  @override
  String get walletSendFaultMemoTooLong =>
      'Votre mémo est trop long. Raccourcissez-le et réessayez.';

  @override
  String get walletSendFaultMemoNotSendable =>
      'Ce mémo ne peut pas être envoyé. Supprimez-le et réessayez.';

  @override
  String get walletSendFaultMemoConflict =>
      'Impossible d\'envoyer ce paiement — l\'application y a joint deux notes. Rien n\'a été envoyé.';

  @override
  String get walletSendFaultNetworkMismatch =>
      'Cette adresse appartient à un réseau différent.';

  @override
  String get walletSendFaultUriInvalid =>
      'Impossible de créer ce paiement. Vérifiez l\'adresse et le montant.';

  @override
  String get walletSendFaultNotSynced =>
      'Votre portefeuille n\'est pas encore assez synchronisé. Attendez que la synchronisation rattrape son retard, ou mettez ce paiement en file pour l\'envoyer plus tard.';

  @override
  String get walletSendFaultNotSyncedNoQueue =>
      'Votre portefeuille n\'est pas encore assez synchronisé. Attendez que la synchronisation rattrape son retard.';

  @override
  String get walletSendFaultNotSyncedSyncNotRunning =>
      'Votre portefeuille n\'est pas encore assez synchronisé, et la synchronisation n\'est pas en cours pour le moment. Vérifiez l\'état de la synchronisation sur l\'écran de votre portefeuille.';

  @override
  String get walletSendFaultAmountsExpired =>
      'Les montants ont expiré pendant votre vérification. Veuillez vérifier le paiement à nouveau.';

  @override
  String get walletSendFaultQueueFull =>
      'Trop d\'envois sont en attente de départ. Laissez-les partir d\'abord, puis réessayez.';

  @override
  String get walletSendFaultWalletBusy =>
      'Le portefeuille est occupé pour le moment. Réessayez dans un instant.';

  @override
  String get walletSendFaultStorageFull =>
      'Il n\'y a pas assez d\'espace libre pour terminer cet envoi. Libérez de l\'espace et réessayez.';

  @override
  String get walletSendFaultOneTimeAddressLimit =>
      'Trop d\'adresses à usage unique sont actuellement utilisées. Certaines peuvent se libérer à mesure que les transferts se confirment, mais cela peut ne pas se résoudre tout seul. Vos fonds sont en sécurité.';

  @override
  String get walletSendFaultCouldNotPrepare =>
      'Impossible de préparer ce paiement. Vérifiez les détails et réessayez.';

  @override
  String get walletSendFaultCouldNotPrepareTransient =>
      'Impossible de préparer ce paiement pour le moment. Réessayez dans un instant.';

  @override
  String get walletSwapButton => 'Échanger';

  @override
  String get walletSwapTitle => 'Échanger du ZEC';

  @override
  String get walletSwapUnavailableWallet =>
      'Votre portefeuille n\'est pas prêt pour le moment. Revenez en arrière et réessayez.';

  @override
  String get walletSwapUnavailableOff =>
      'L\'échange n\'est pas disponible pour le moment.';

  @override
  String get walletSwapUnavailableWatchOnly =>
      'Ce portefeuille est en lecture seule — il ne peut pas échanger.';

  @override
  String get walletSwapDone => 'Terminé';

  @override
  String get walletSwapBackToWallet => 'Retour au portefeuille';

  @override
  String walletSwapAvailable(String amount) {
    return 'Disponible à échanger : $amount ZEC';
  }

  @override
  String walletSwapAvailableCatchingUp(String amount) {
    return 'Disponible à échanger : $amount ZEC — votre solde rattrape encore son retard';
  }

  @override
  String get walletSwapAssetLabel => 'Actif à recevoir';

  @override
  String get walletSwapAmountLabel => 'Montant à échanger (ZEC)';

  @override
  String get walletSwapAmountHint => '0.00';

  @override
  String get walletSwapDestinationLabel => 'Adresse de destination';

  @override
  String get walletSwapDestinationHint =>
      'Votre adresse de réception sur la chaîne de destination';

  @override
  String walletSwapDestinationLabelChain(String chain) {
    return 'Votre adresse de réception $chain';
  }

  @override
  String walletSwapDestinationHelperChain(String chain) {
    return 'Une adresse $chain — c\'est là que votre actif échangé sera envoyé. Vérifiez bien que la chaîne est correcte.';
  }

  @override
  String get walletSwapDestinationScanTooltip =>
      'Scanner un code QR d\'adresse de destination';

  @override
  String get walletSwapTargetAssetHint => 'Sélectionnez un actif à recevoir';

  @override
  String get walletSwapQuoteButton => 'Obtenir un devis';

  @override
  String get walletSwapQuoting => 'Obtention du devis…';

  @override
  String get walletSwapExecuting => 'Démarrage de votre échange…';

  @override
  String get walletSwapExecuteStillWorking =>
      'Toujours en cours — l\'échange démarre. Cela peut prendre jusqu\'à une minute.';

  @override
  String get walletSwapReviewTitle => 'Confirmer l\'échange';

  @override
  String get walletSwapYouSendLabel => 'Vous envoyez';

  @override
  String get walletSwapYouReceiveLabel => 'Vous recevez au moins';

  @override
  String walletSwapReceiveValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String get walletSwapNetworkFeeLabel => 'Frais de réseau';

  @override
  String get walletSwapNetworkFeeValue => 'Ajoutés lors de l\'envoi du dépôt';

  @override
  String walletSwapQuoteExpiresIn(String time) {
    return 'Devis valable encore environ $time — confirmez avant son expiration.';
  }

  @override
  String get walletSwapQuoteExpiresUnderMinute =>
      'Devis valable encore moins d\'une minute — confirmez avant son expiration.';

  @override
  String get walletSwapQuoteExpired =>
      'Ce devis a expiré. Revenez en arrière pour en obtenir un nouveau — son taux n\'est plus garanti, et envoyer maintenant risque d\'entraîner un remboursement.';

  @override
  String get walletCountdownUnderMinute => 'moins d\'une minute';

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
  String get walletSwapDeshieldTitle => 'Cet échange n\'est pas privé';

  @override
  String get walletSwapDeshieldBody =>
      'Échanger votre ZEC le rend non protégé — le dépôt est une transaction publique, et le côté du prestataire est public sur son réseau.';

  @override
  String get walletSwapDiscloseTitle =>
      'Ce que le prestataire d\'échange verra';

  @override
  String get walletSwapDiscloseAmounts => 'Les montants des deux côtés';

  @override
  String get walletSwapDiscloseCrossLink =>
      'Que ce ZEC et l\'actif que vous recevez font partie d\'un même échange';

  @override
  String get walletSwapDiscloseDestination => 'Votre adresse de destination';

  @override
  String get walletSwapDiscloseSource => 'Votre adresse source';

  @override
  String get walletSwapDiscloseIp =>
      'Votre adresse IP (sauf si vous passez par Tor)';

  @override
  String get walletSwapDiscloseGeneric => 'D\'autres détails de cet échange';

  @override
  String get walletSwapDiscloseProviderLegsPublic =>
      'Les propres transactions du prestataire sont publiques sur son réseau';

  @override
  String get walletSwapAckLabel =>
      'Je comprends que le prestataire verra les informations ci-dessus.';

  @override
  String get walletSwapConfirmButton => 'Démarrer l\'échange';

  @override
  String get walletSwapBackButton => 'Retour';

  @override
  String get walletSwapStatusPendingTitle => 'Échange démarré';

  @override
  String get walletSwapStatusCheckingTitle =>
      'Vérification de l\'état de l\'échange…';

  @override
  String get walletSwapStatusPendingBodyOutOfZec =>
      'Votre portefeuille envoie le dépôt de ZEC au prestataire. Si vous êtes brièvement hors ligne, il sera envoyé automatiquement dès que vous serez de nouveau en ligne — mais la fenêtre d\'envoi est courte, et si elle se referme avant, l\'échange prend simplement fin et rien n\'est échangé. Votre ZEC vous reste acquis, et il peut prendre jusqu\'à une heure avant de réapparaître comme disponible.';

  @override
  String get walletSwapStatusPendingBodyIntoZec =>
      'En attente de l\'arrivée de votre dépôt. Si vous n\'avez pas encore envoyé les fonds depuis votre autre portefeuille, envoyez-les avant l\'expiration du devis.';

  @override
  String get walletSwapStatusPendingBodyIntoZecReattached =>
      'Cet échange attend toujours son dépôt. Les instructions de dépôt ne sont plus disponibles sur cet appareil — si vous avez déjà envoyé les fonds, ils seront détectés ; sinon, laissez cet échange expirer et démarrez-en un nouveau.';

  @override
  String walletSwapPendingWindowEndsAt(String time) {
    return 'La fenêtre de dépôt se termine : $time.';
  }

  @override
  String get walletSwapPendingWindowPassedOutOfZec =>
      'La fenêtre de dépôt est passée. Si le dépôt n\'a pas été envoyé à temps, l\'échange prend fin et votre ZEC reste dans votre portefeuille.';

  @override
  String get walletSwapPendingWindowPassedIntoZec =>
      'La fenêtre de dépôt est passée. Si vous n\'avez pas envoyé votre dépôt, cet échange prend simplement fin — obtenez un nouveau devis quand vous serez prêt.';

  @override
  String walletSwapsInFlightTitle(int count) {
    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Échanges en cours',
      one: 'Échange en cours',
    );
    return '$_temp0';
  }

  @override
  String get walletSwapInFlightRowOutOfZec =>
      'Votre ZEC est en route vers le prestataire.';

  @override
  String get walletSwapInFlightRowIntoZec =>
      'En attente de l\'arrivée de votre dépôt chez le prestataire.';

  @override
  String get walletSwapInFlightRowGeneric => 'Un échange est en cours.';

  @override
  String get walletSwapInFlightRowPastWindow =>
      'La fenêtre de dépôt est passée — vérifiez le statut de cet échange.';

  @override
  String get walletSwapInFlightRowOverdue =>
      'Cet échange n\'a pas encore atteint un résultat confirmé ici — ouvrez-le pour vérifier. Tout ZEC revenant à ce portefeuille apparaît dans votre solde après une synchronisation.';

  @override
  String get walletSwapInFlightRowOverdueIntoZec =>
      'Cet échange n\'a pas encore atteint un résultat confirmé ici — ouvrez-le pour vérifier. Tout ZEC livré par cet échange à ce portefeuille apparaît dans votre solde après une synchronisation.';

  @override
  String get walletSwapRowOutcomeSuccess => 'Échange terminé.';

  @override
  String get walletSwapRowOutcomeRefunded => 'Échange remboursé.';

  @override
  String get walletSwapRowOutcomeFailed => 'Échange non terminé.';

  @override
  String get walletSwapRemove => 'Supprimer';

  @override
  String get walletSwapRemoveTitle => 'Supprimer cet échange de la liste ?';

  @override
  String get walletSwapRemoveBodyInFlight =>
      'Cela retire uniquement l\'échange de cette liste — cela n\'annule pas l\'échange, et ce portefeuille cessera de suivre son remboursement. Le ZEC remboursé plus tard appartient toujours à ce portefeuille ; une nouvelle analyse complète peut le retrouver.';

  @override
  String get walletSwapRemoveBodyInFlightIntoZec =>
      'Cela retire uniquement l\'échange de cette liste — cela n\'annule pas l\'échange, et ce portefeuille cessera de suivre le ZEC entrant. Le ZEC livré plus tard appartient toujours à ce portefeuille ; une nouvelle analyse complète peut le retrouver. Si l\'échange est remboursé à la place, le remboursement revient dans l\'actif que vous avez envoyé, en dehors de ce portefeuille.';

  @override
  String get walletSwapRemoveBodyInFlightUnknown =>
      'Cela retire uniquement l\'échange de cette liste — cela n\'annule pas l\'échange, et ce portefeuille cessera de suivre le ZEC qui en provient encore. Le ZEC qui arrive plus tard appartient toujours à ce portefeuille ; une nouvelle analyse complète peut le retrouver.';

  @override
  String get walletSwapRemoveBodyDone =>
      'Cela retire l\'échange terminé de la liste.';

  @override
  String get walletSwapRemoveCancel => 'Annuler';

  @override
  String get walletSwapRemoveConfirm => 'Supprimer';

  @override
  String walletSwapInFlightStarted(String time) {
    return 'Démarré $time';
  }

  @override
  String get walletSwapViewSwap => 'Afficher l\'échange';

  @override
  String get walletSwapsInFlightError =>
      'Impossible de charger vos échanges en cours pour le moment.';

  @override
  String get walletSwapsInFlightRetry => 'Réessayer';

  @override
  String get walletSwapsInFlightRetryInProgress => 'Tentative en cours…';

  @override
  String get walletSwapStartAnother => 'Démarrer un autre échange';

  @override
  String get walletSwapStatusUnderTitle => 'En attente du dépôt complet';

  @override
  String get walletSwapStatusUnderBody =>
      'Une partie du dépôt est arrivée. Le reste est en cours, ou le prestataire procédera à un remboursement.';

  @override
  String get walletSwapStatusUnderBodyIntoZec =>
      'Une partie de votre dépôt est arrivée. Envoyez le montant manquant avant l\'échéance, ou le prestataire remboursera ce qui est arrivé.';

  @override
  String walletSwapStatusUnderDetail(
    String received,
    String missing,
    String time,
  ) {
    return 'Reçu : $received ; $missing manquant encore. La fenêtre de dépôt se termine : $time.';
  }

  @override
  String get walletSwapStatusDetectedTitle => 'Dépôt reçu';

  @override
  String get walletSwapStatusDetectedBody =>
      'Le prestataire a reçu votre dépôt et va traiter l\'échange.';

  @override
  String get walletSwapStatusProcessingTitle => 'Traitement de votre échange';

  @override
  String get walletSwapStatusProcessingBody =>
      'Le prestataire finalise votre échange.';

  @override
  String get walletSwapStatusSuccessTitle => 'Échange terminé';

  @override
  String get walletSwapStatusSuccessBody =>
      'Votre échange s\'est terminé avec succès.';

  @override
  String get walletSwapStatusRefundedTitle => 'Échange remboursé';

  @override
  String get walletSwapStatusRefundedBody =>
      'L\'échange ne s\'est pas terminé, le prestataire a donc renvoyé les fonds à votre adresse de remboursement.';

  @override
  String get walletSwapStatusRefundedBodyOutOfZec =>
      'L\'échange ne s\'est pas terminé, le prestataire a donc renvoyé votre ZEC vers ce portefeuille. Il arrive sous forme de fonds non protégés et apparaît dans votre solde après la prochaine synchronisation du portefeuille — cela peut prendre un peu de temps.';

  @override
  String get walletSwapStatusFailedTitle => 'Échange échoué';

  @override
  String get walletSwapStatusFailedBody =>
      'L\'échange n\'a pas pu être finalisé. Les fonds déposés seront réglés ou remboursés du côté du prestataire.';

  @override
  String get walletSwapStatusNotFoundTitle => 'Échange introuvable';

  @override
  String get walletSwapStatusNotFoundBody =>
      'Le prestataire n\'a plus d\'enregistrement de cet échange — il a probablement expiré. Si un dépôt a été effectué, le prestataire devrait le rembourser à l\'adresse de remboursement. L\'échange reste dans votre liste, et ce portefeuille continue de suivre son ZEC au cas où il arriverait encore ; vous pouvez le supprimer de la liste à tout moment.';

  @override
  String get walletSwapStatusUnknownTitle => 'Statut indisponible';

  @override
  String get walletSwapStatusUnknownBody =>
      'Impossible de lire le statut de cet échange pour le moment.';

  @override
  String get walletSwapTrackingUnavailableTitle => 'Suivi indisponible';

  @override
  String get walletSwapTrackingUnavailableBody =>
      'L\'échange est désactivé, nous ne pouvons donc pas suivre cette opération ici. Les fonds seront réglés ou remboursés du côté du prestataire.';

  @override
  String get walletSwapTrackingUnavailableBodyOutOfZec =>
      'L\'échange est désactivé ici, cet échange ne peut donc pas être suivi pour le moment. S\'il a été remboursé, le ZEC revient à ce portefeuille — il apparaît dans votre solde une fois l\'échange réactivé et le portefeuille synchronisé.';

  @override
  String get walletSwapTrackingError => 'Impossible de suivre cet échange.';

  @override
  String get walletSwapTrackingErrorBody =>
      'Impossible d\'ouvrir le suivi de cet échange. L\'échange lui-même est peut-être toujours en cours — les fonds déposés seront réglés ou remboursés du côté du prestataire.';

  @override
  String get walletSwapFaultDestinationRequired =>
      'Saisissez l\'adresse à laquelle vous souhaitez recevoir l\'actif échangé.';

  @override
  String get walletSwapFaultDestinationInvalid =>
      'Cette adresse de destination n\'est pas valide pour cet actif. Vérifiez-la et réessayez.';

  @override
  String get walletSwapFaultExpired =>
      'Ce devis a expiré. Obtenez un nouveau devis pour continuer.';

  @override
  String get walletSwapFaultOutOfBounds =>
      'Le prix du prestataire est sorti de votre limite, l\'échange a donc été arrêté avant tout mouvement de fonds. Réessayez.';

  @override
  String get walletSwapFaultSlippageTooHigh =>
      'La limite de glissement est trop élevée pour un échange sûr. Réessayez.';

  @override
  String get walletSwapFaultProviderUnavailable =>
      'Le prestataire d\'échange est indisponible pour le moment. Réessayez dans un instant.';

  @override
  String get walletSwapFaultConnection =>
      'Impossible de joindre le service d\'échange. Veuillez vérifier votre connexion internet et réessayer.';

  @override
  String get walletSwapFaultProviderMisbehaved =>
      'Le prestataire d\'échange a renvoyé une réponse inattendue, l\'échange a donc été arrêté. Réessayez.';

  @override
  String get walletSwapFaultSwapOff =>
      'L\'échange est désactivé pour le moment.';

  @override
  String get walletSwapFaultDepositFailed =>
      'Nous n\'avons pas pu envoyer votre dépôt, aucun fonds n\'a donc quitté votre portefeuille. Obtenez un nouveau devis pour réessayer.';

  @override
  String get walletSwapFaultAlreadyInFlight =>
      'Un échange est déjà en cours. Vous pourrez en démarrer un nouveau une fois celui-ci intégralement réglé ou son devis expiré — cela peut prendre un certain temps.';

  @override
  String get walletSwapFaultRefundUnavailable =>
      'Ce portefeuille ne peut pas encore configurer d\'adresse de remboursement — cela signifie généralement que la première synchronisation n\'est pas terminée. Attendez que la synchronisation se termine, puis réessayez.';

  @override
  String get walletSwapFaultDestinationUnavailable =>
      'Ce portefeuille ne peut pas encore configurer d\'adresse de réception pour cet échange — cela signifie généralement que la première synchronisation n\'est pas terminée. Attendez que la synchronisation se termine, puis réessayez.';

  @override
  String get walletSwapFaultExecuteTimeout =>
      'L\'échange n\'a pas pu démarrer à temps — la connexion était peut-être lente, ou le portefeuille était occupé. Obtenez un nouveau devis et réessayez.';

  @override
  String get walletSwapFaultStoreBusyRetry =>
      'Le portefeuille est occupé pour le moment. Réessayez.';

  @override
  String get walletSwapFaultTermsDiffer =>
      'Ce devis ne correspond pas à celui que votre portefeuille a émis ; rien n\'a donc été envoyé. Obtenez un nouveau devis et réessayez.';

  @override
  String walletSwapFaultInsufficient(String needed, String spendable) {
    return 'Cet échange nécessite environ $needed ZEC, frais de réseau compris, mais seulement $spendable ZEC sont disponibles pour le moment.';
  }

  @override
  String walletSwapFaultOverCeiling(String limit) {
    return 'Cette application limite actuellement les échanges à $limit ZEC.';
  }

  @override
  String walletSwapFaultInsufficientCatchingUp(
    String needed,
    String spendable,
  ) {
    return 'Cet échange nécessite environ $needed ZEC, frais de réseau compris, mais seulement $spendable ZEC sont disponibles pour le moment. Votre solde rattrape encore son retard — davantage pourrait devenir disponible bientôt.';
  }

  @override
  String get walletSwapFaultStateUnavailable =>
      'Le portefeuille n\'a pas pu enregistrer cet échange en toute sécurité, aucun fonds n\'a donc bougé. Réessayez.';

  @override
  String get walletSwapFaultRequestInvalid =>
      'Cette demande d\'échange n\'a pas pu être traitée. Obtenez un nouveau devis et réessayez.';

  @override
  String get walletSwapFaultCouldNotQuote =>
      'Impossible d\'obtenir un devis d\'échange. Vérifiez les détails et réessayez.';

  @override
  String get walletSwapFaultWalletUnavailable =>
      'Votre portefeuille n\'est pas prêt pour le moment. Revenez en arrière et réessayez.';

  @override
  String get walletSwapDirectionBuy => 'Acheter du ZEC';

  @override
  String get walletSwapDirectionSell => 'Vendre du ZEC';

  @override
  String get walletSwapRefundLabel => 'Votre adresse de remboursement';

  @override
  String get walletSwapRefundHint =>
      'Là où vos pièces retournent si l\'échange échoue';

  @override
  String get walletSwapRefundHelper =>
      'Sur la chaîne depuis laquelle vous envoyez — pas une adresse Zcash.';

  @override
  String walletSwapRefundLabelChain(String chain) {
    return 'Votre adresse de remboursement $chain';
  }

  @override
  String walletSwapRefundHelperChain(String chain) {
    return 'Une adresse $chain — là où vos pièces retournent si l\'échange échoue. Pas une adresse Zcash.';
  }

  @override
  String get walletSwapRefundInfoTitle =>
      'À propos de votre adresse de remboursement';

  @override
  String get walletSwapRefundInfoBody =>
      'Si l\'échange ne peut pas aboutir, le prestataire renvoie vos pièces à cette adresse sur la chaîne depuis laquelle vous avez payé. Saisissez une adresse que vous contrôlez — le portefeuille ne peut pas vérifier une adresse étrangère à votre place, alors vérifiez-la attentivement.';

  @override
  String get walletSwapRefundScanTooltip =>
      'Scanner un code QR d\'adresse de remboursement';

  @override
  String get walletSwapScanTitle => 'Scanner une adresse';

  @override
  String get walletSwapScanInstruction =>
      'Pointez votre caméra vers le code QR de l\'adresse.';

  @override
  String get walletSwapScanManualEntry => 'Saisir manuellement';

  @override
  String get walletSwapScanCancel => 'Annuler';

  @override
  String get walletSwapScanCameraUnavailable =>
      'Caméra indisponible. Saisissez l\'adresse manuellement ci-dessous.';

  @override
  String get walletSwapSourceAssetLabel => 'Actif à échanger';

  @override
  String get walletSwapSourceAssetHint => 'Sélectionnez un actif';

  @override
  String walletSwapForeignAmountLabel(String symbol) {
    return 'Montant à envoyer ($symbol)';
  }

  @override
  String get walletSwapForeignAmountLabelGeneric => 'Montant à envoyer';

  @override
  String walletSwapForeignValue(String amount, String asset) {
    return '$amount $asset';
  }

  @override
  String walletSwapTokenLabel(String symbol, String chain) {
    return '$symbol sur $chain';
  }

  @override
  String get walletSwapPickerTitle => 'Choisissez un actif de départ';

  @override
  String get walletSwapPickerTitleReceive => 'Choisissez un actif à recevoir';

  @override
  String get walletSwapPickerStale =>
      'Impossible d\'actualiser la liste des actifs — affichage de la dernière liste connue.';

  @override
  String get walletSwapPickerEmpty =>
      'Aucun actif n\'est disponible pour l\'échange pour le moment. Réessayez plus tard.';

  @override
  String get walletSwapPickerSearchHint => 'Rechercher par nom ou par chaîne';

  @override
  String walletSwapPickerNoMatch(String query) {
    return 'Aucun actif ne correspond à « $query ».';
  }

  @override
  String get walletSwapPickerError =>
      'Impossible de charger la liste des actifs. Vérifiez votre connexion et réessayez.';

  @override
  String get walletSwapPickerRetry => 'Réessayer';

  @override
  String get walletSwapSlippageLabel => 'Tolérance de glissement';

  @override
  String walletSwapSlippagePercent(String value) {
    return '$value %';
  }

  @override
  String get walletSwapSlippageCustom => 'Personnalisé';

  @override
  String get walletSwapSlippageCustomLabel => 'Glissement personnalisé';

  @override
  String get walletSwapSlippageMayFail =>
      'Très faible — l\'échange peut échouer si le prix évolue.';

  @override
  String get walletSwapSlippageNormal => 'Une tolérance sûre.';

  @override
  String get walletSwapSlippageRisky =>
      'Élevée — vous pourriez recevoir sensiblement moins que le montant coté.';

  @override
  String get walletSwapSlippageTooHigh =>
      'Trop élevée — l\'échange sera rejeté. Réduisez-la à 10 % ou moins.';

  @override
  String walletSwapIntoZecFloorNote(String zec, String slippage) {
    return 'Vous recevrez au moins $zec ZEC — votre plancher de glissement de $slippage %. Le montant final ne descendra pas en dessous de ce seuil.';
  }

  @override
  String get walletSwapIntoZecShieldTitle =>
      'Vous recevez du ZEC sur votre propre adresse';

  @override
  String get walletSwapIntoZecEndsShielded =>
      'Tant que vous ne l\'avez pas protégé — un simple geste, suggéré à l\'arrivée — le montant reçu est brièvement public et visible sur la chaîne. Un petit montant peut rester public jusqu\'à son accumulation.';

  @override
  String get walletSwapRefundVerifyTitle =>
      'Vérifiez votre adresse de remboursement';

  @override
  String get walletSwapRefundVerifyBody =>
      'Vérifiez-la caractère par caractère — c\'est là que vos pièces retournent si l\'échange échoue. Le portefeuille ne peut pas vérifier une adresse étrangère à votre place.';

  @override
  String get walletSwapRefundVerifyAck =>
      'J\'ai vérifié que mon adresse de remboursement est correcte.';

  @override
  String get walletSwapPayoutVerifyTitle =>
      'Vérifiez votre adresse de réception';

  @override
  String walletSwapPayoutVerifyBody(String asset) {
    return 'Vérifiez-la caractère par caractère — c\'est là que vous recevrez $asset. Le portefeuille ne peut pas vérifier une adresse étrangère à votre place.';
  }

  @override
  String get walletSwapPayoutVerifyAck =>
      'J\'ai vérifié que mon adresse de réception est correcte.';

  @override
  String get walletSwapTrackingUnavailableBodyIntoZec =>
      'L\'échange est désactivé ici. Tout ZEC déjà en route apparaîtra dans votre portefeuille après votre prochaine synchronisation.';

  @override
  String get walletSwapFaultForeignAmountRequired =>
      'Saisissez le montant que vous souhaitez échanger.';

  @override
  String get walletSwapFaultRefundAddressRequired =>
      'Saisissez votre adresse de remboursement sur la chaîne source.';

  @override
  String get walletSwapDepositTitle => 'Envoyez votre paiement';

  @override
  String walletSwapDepositInstruction(
    String amount,
    String asset,
    String chain,
  ) {
    return 'Envoyez exactement $amount $asset sur $chain à l\'adresse ci-dessous.';
  }

  @override
  String get walletSwapDepositExactNote =>
      'Envoyez le montant exact. Envoyer moins, ou envoyer après la fermeture de la fenêtre, entraîne un remboursement du prestataire vers votre adresse de remboursement.';

  @override
  String walletSwapDepositExpiresIn(String time) {
    return 'Fenêtre de dépôt : $time restant';
  }

  @override
  String get walletSwapDepositExpired =>
      'Cette fenêtre de dépôt est fermée. N\'envoyez pas de fonds maintenant — démarrez un nouvel échange. Si vous avez déjà envoyé les fonds, le prestataire devrait les rembourser à votre adresse de remboursement.';

  @override
  String get walletSwapDepositQrLabel => 'Code QR de l\'adresse de dépôt';

  @override
  String get walletSwapDepositAddressLabel => 'Adresse de dépôt';

  @override
  String get walletSwapDepositCopy => 'Copier l\'adresse de dépôt';

  @override
  String get walletSwapDepositCopied => 'Adresse de dépôt copiée';

  @override
  String get walletSwapDepositMemoRequired => 'Ce dépôt nécessite un mémo/tag';

  @override
  String get walletSwapDepositMemoWarning =>
      'Vous DEVEZ inclure ce mémo exact avec votre dépôt. L\'envoyer sans lui — ou avec un mémo incorrect — peut entraîner la perte définitive de vos fonds.';

  @override
  String get walletSwapDepositMemoLabel => 'Mémo/tag de dépôt';

  @override
  String get walletSwapDepositMemoCopy => 'Copier le mémo';

  @override
  String get walletSwapDepositMemoCopied => 'Mémo copié';

  @override
  String get walletSwapDepositSent => 'J\'ai envoyé les fonds';

  @override
  String get walletSwapDepositBackTitle => 'Quitter cet écran ?';

  @override
  String get walletSwapDepositBackBody =>
      'Cela n\'annulera pas votre échange — il continue en arrière-plan. Mais vous aurez besoin de l\'adresse de dépôt pour payer, alors copiez-la d\'abord si ce n\'est pas déjà fait.';

  @override
  String get walletSwapDepositBackBodyExpired =>
      'Cela n\'annulera pas votre échange — il continue en arrière-plan. La fenêtre de dépôt est fermée. N\'envoyez pas de fonds à l\'adresse de dépôt maintenant. Si vous avez déjà envoyé les fonds, le prestataire devrait les rembourser à votre adresse de remboursement.';

  @override
  String get walletSwapDepositBackStay => 'Rester';

  @override
  String get walletSwapDepositBackLeave => 'Quitter';

  @override
  String get walletReceive => 'Recevoir';

  @override
  String get walletReceiveSubtitle =>
      'Partagez cette adresse pour recevoir du ZEC. Elle peut être partagée publiquement en toute sécurité.';

  @override
  String get walletReceiveCopy => 'Copier l\'adresse';

  @override
  String get walletReceiveCopied => 'Adresse copiée';

  @override
  String get walletReceiveUnavailable =>
      'Votre portefeuille n\'est pas encore prêt.';

  @override
  String get walletReceiveError =>
      'Impossible de charger votre adresse. Veuillez réessayer.';

  @override
  String get walletReceivePreparing => 'Préparation de votre adresse…';

  @override
  String get walletReceivePreparingHint =>
      'Votre portefeuille prépare cette adresse sur votre appareil — cela peut prendre un moment si le portefeuille est occupé par une autre tâche.';

  @override
  String get walletReceiveRetry => 'Réessayer';

  @override
  String get walletReceiveQrLabel => 'Code QR de votre adresse de réception';

  @override
  String get walletReceiveTypeShielded => 'Protégée';

  @override
  String get walletReceiveTypeTransparent => 'Publique';

  @override
  String get walletReceiveSubtitleTransparent =>
      'Partagez cette adresse publique pour recevoir du ZEC d\'un expéditeur qui ne peut pas payer une adresse protégée.';

  @override
  String get walletReceiveTransparentWarning =>
      'Il s\'agit d\'une adresse publique : elle est visible sur la chaîne et relie vos paiements entre eux en cas de réutilisation. Préférez votre adresse protégée ; protégez ces fonds après réception.';

  @override
  String get walletReceiveQrLabelTransparent =>
      'Code QR de votre adresse de réception publique';

  @override
  String get walletReceiveFreshAddress => 'Utiliser une nouvelle adresse';

  @override
  String get walletReceiveFreshCaption =>
      'Nouvelle adresse — elle ne peut pas être liée à vos autres adresses. Les paiements vers celle-ci arrivent quand même dans ce portefeuille, et vos adresses précédentes continuent de fonctionner. Elle ne sera plus affichée ici — copiez-la maintenant.';

  @override
  String get walletReceiveFreshError =>
      'Impossible de créer une nouvelle adresse. Réessayez.';

  @override
  String get walletReceiveFreshBusy =>
      'Le portefeuille est occupé pour le moment. Réessayez avec la nouvelle adresse dans un instant.';

  @override
  String get walletReceiveShare => 'Partager';

  @override
  String get walletReceiveRequestAmount => 'Demander un montant';

  @override
  String get walletReceiveRequestAmountLabel => 'Montant (facultatif)';

  @override
  String get walletReceiveFreshCopyNow =>
      'Elle ne sera plus affichée ici — copiez-la maintenant.';

  @override
  String get walletSecurityMenuItem => 'Sécurité…';

  @override
  String get securityTitle => 'Sécurité';

  @override
  String get securityUnavailableBody =>
      'Les paramètres de sécurité du portefeuille sont gérés par cette application, et non par le portefeuille lui-même.';

  @override
  String get securityCustodySectionTitle => 'Conservation des clés';

  @override
  String get securityCustodyTierSecureEnclave => 'Secure Enclave (matériel)';

  @override
  String get securityCustodyTierStrongBox => 'StrongBox (matériel)';

  @override
  String get securityCustodyTierTee => 'Coffre-fort matériel (TEE)';

  @override
  String get securityCustodyTierSoftware => 'Coffre-fort logiciel';

  @override
  String get securityCustodyTierKeychain => 'Trousseau (chiffré par logiciel)';

  @override
  String get securityCustodyTierNone => 'Aucun coffre-fort matériel';

  @override
  String get securityCustodyTierUnknown => 'Inconnu';

  @override
  String get securityCustodyHardwareKey =>
      'La clé qui verrouille ce portefeuille est conservée dans le matériel sécurisé de cet appareil et est supprimée avec le portefeuille.';

  @override
  String get securityCustodyBestEffort =>
      'La suppression retire vos clés au mieux ; une brève fenêtre de récupération forensique peut subsister jusqu\'à ce que l\'appareil récupère l\'espace de stockage. Pour une garantie totale, utilisez également la fonction « Effacer tout le contenu » de votre appareil.';

  @override
  String get securityCustodyProbeError =>
      'Impossible de lire le statut de conservation. Revenez en arrière et réessayez.';

  @override
  String get securityDeleteWalletButton => 'Supprimer le portefeuille';

  @override
  String get securityDeleteWalletSubtitle =>
      'Supprime ce portefeuille et sa clé de cet appareil. Vos fonds restent sur la chaîne et peuvent être restaurés à partir de votre phrase de récupération.';

  @override
  String get securityDeleteWalletSubtitleWatchOnly =>
      'Supprime ce portefeuille et sa clé de cet appareil. Il ne détient aucune clé de dépense, il n\'y a donc rien à sauvegarder — vous pouvez le rajouter à tout moment avec sa clé de consultation.';

  @override
  String get securityDeleteDialogTitle => 'Supprimer ce portefeuille ?';

  @override
  String get securityDeleteDialogBody =>
      'Cette opération supprime le portefeuille et sa clé de cet appareil. Assurez-vous d\'avoir sauvegardé votre phrase de récupération — c\'est le SEUL moyen de restaurer vos fonds.';

  @override
  String get securityDeleteDialogBodyWatchOnly =>
      'Cette opération supprime le portefeuille et sa clé de cet appareil. Il ne détient aucune clé de dépense, il n\'y a donc rien à sauvegarder — vous pourrez le rajouter plus tard avec sa clé de consultation.';

  @override
  String get securityDeleteDialogConfirm => 'Supprimer';

  @override
  String get securityDeleteDialogCancel => 'Annuler';

  @override
  String get securityDeleteFailedSnack =>
      'Impossible de supprimer le portefeuille — votre portefeuille est inchangé. Réessayez.';

  @override
  String securityDeleteRefusedBusySnack(int seconds) {
    return 'Terminez d\'abord le changement de serveur — il aboutit ou s\'arrête en $seconds secondes au plus. Réessayez ensuite de supprimer le portefeuille.';
  }

  @override
  String get walletParkedTitle => 'Enregistré et en attente';

  @override
  String get walletParkedSubtitle =>
      'Ces paiements n\'ont pas encore été envoyés. Leurs montants font toujours partie de votre solde.';

  @override
  String get walletParkedSubtitlePreparing =>
      'Ces paiements n\'ont pas encore été envoyés. Leurs montants font toujours partie de votre solde — sauf ceux que votre portefeuille est en train d\'envoyer, dont le montant est peut-être déjà réservé.';

  @override
  String get walletParkedCancel => 'Annuler';

  @override
  String get walletParkedPausedHint =>
      'En pause — ce paiement ne sera pas envoyé tout seul. Vos fonds sont en sécurité. Envoyez-le maintenant, ou annulez-le.';

  @override
  String get walletParkedRetryStale =>
      'Ce paiement n\'est plus en attente. Consultez vos paiements en attente et votre activité.';

  @override
  String get walletParkedAlreadyInProgress =>
      'Ce paiement n\'est plus en attente — votre portefeuille est peut-être déjà en train de l\'envoyer. Consultez « Enregistré et en attente » et votre activité.';

  @override
  String get walletReclaimExplainer =>
      'Les envois par adresse à usage unique sont bloqués. Vous pouvez les rouvrir — cela déplace un petit montant entre vos propres adresses, puis vous le renvoie.';

  @override
  String get walletReclaimButton => 'Rouvrir l\'envoi';

  @override
  String get walletReclaimInProgress => 'Réouverture…';

  @override
  String get walletReclaimConfirmTitle =>
      'Rouvrir l\'envoi par adresse à usage unique ?';

  @override
  String get walletReclaimConfirmBody =>
      'Cela déplace un petit montant entre vos propres adresses pour libérer l\'envoi par adresse à usage unique, puis vous le renvoie. Cela coûte quelques frais de réseau. Une fois confirmé, récupérez le montant déplacé avec « Récupérer maintenant ».';

  @override
  String get walletReclaimConfirmCancel => 'Pas maintenant';

  @override
  String get walletReclaimConfirmAction => 'Rouvrir';

  @override
  String get walletReclaimStarted =>
      'Réouverture lancée. Une fois confirmée, envoyez le paiement en pause, puis récupérez le montant déplacé avec « Récupérer maintenant ».';

  @override
  String get walletReclaimNothing => 'Rien à rouvrir pour le moment.';

  @override
  String get walletReclaimNotBroadcast =>
      'Impossible de confirmer que la réouverture a atteint le réseau. Elle a peut-être quand même abouti. Réessayez dans un instant.';

  @override
  String get walletReclaimNeedsFunds =>
      'Il vous faut du ZEC protégé pour rouvrir l\'envoi.';

  @override
  String get walletReclaimFailed =>
      'Impossible de rouvrir l\'envoi pour le moment. Vos fonds sont inchangés. Réessayez.';

  @override
  String get walletReclaimUnknown =>
      'Réouverture terminée. Vérifiez vos envois par adresse à usage unique, et récupérez un éventuel montant déplacé avec « Récupérer maintenant ».';

  @override
  String get walletParkedError =>
      'Impossible de charger vos paiements en attente pour le moment.';

  @override
  String get walletParkedErrorRetry => 'Réessayer';

  @override
  String get walletParkedErrorRetryInProgress => 'Tentative en cours…';

  @override
  String get walletParkedCancelConfirmTitle =>
      'Annuler ce paiement en attente ?';

  @override
  String get walletParkedCancelConfirmBody =>
      'Cela supprime le paiement enregistré. Il n\'a pas été envoyé, aucun fonds ne quitte donc votre portefeuille — mais cette action est irréversible.';

  @override
  String get walletParkedCancelConfirmKeep => 'Conserver';

  @override
  String get walletParkedCancelConfirmDiscard => 'Supprimer le paiement';

  @override
  String get walletParkedCancelDone => 'Paiement en attente annulé.';

  @override
  String get walletParkedCancelAlreadySending =>
      'Ce paiement est peut-être déjà en route — consultez votre activité.';

  @override
  String get walletParkedCancelFailed =>
      'Impossible d\'annuler pour le moment. Votre paiement est inchangé. Réessayez.';

  @override
  String get walletRecoverNow => 'Récupérer maintenant';

  @override
  String get walletRecoverConfirmTitle =>
      'Récupérer vers votre solde protégé ?';

  @override
  String get walletRecoverConfirmBody =>
      'Cette opération vérifie vos adresses à usage unique et transfère tout ce qui est trouvé vers votre solde privé protégé. Elle peut être relancée à tout moment en toute sécurité.';

  @override
  String get walletRecoverConfirmCancel => 'Pas maintenant';

  @override
  String get walletRecoverConfirmAction => 'Récupérer';

  @override
  String get walletRecoverInProgress => 'Récupération en cours…';

  @override
  String walletRecoverDone(String amount) {
    return '$amount est en cours de récupération vers votre solde protégé.';
  }

  @override
  String walletRecoverDonePartial(String amount) {
    return '$amount est en cours de récupération — certains fonds nécessitent encore une nouvelle tentative.';
  }

  @override
  String get walletRecoverRetry =>
      'Certains fonds nécessitent une nouvelle tentative — relancez la récupération.';

  @override
  String get walletRecoverTruncated =>
      'Toutes les adresses à usage unique n\'ont pas encore été vérifiées — relancez l\'opération pour vérifier le reste.';

  @override
  String get walletRecoverNothing => 'Rien à récupérer pour le moment.';

  @override
  String get walletRecoverFailed =>
      'Impossible de récupérer pour le moment. Vos fonds sont inchangés. Réessayez.';

  @override
  String walletParkedRowTimed(String amount, String time) {
    return '$amount enregistré et en attente · $time';
  }

  @override
  String walletParkedCancelSemanticTimed(String amount, String time) {
    return 'Annuler le paiement de $amount enregistré $time';
  }

  @override
  String walletParkedRowPausedTimed(String amount, String time) {
    return '$amount en pause · $time';
  }

  @override
  String walletParkedRowPreparingTimed(String amount, String time) {
    return '$amount en préparation d\'envoi · $time';
  }

  @override
  String get walletParkedPreparingHint =>
      'Votre portefeuille prépare ce paiement — son montant est peut-être déjà réservé. Vos fonds sont en sécurité. S\'il n\'aboutit pas, il retourne dans la liste tout seul.';

  @override
  String get walletParkedPreparingHintSyncPaused =>
      'Votre portefeuille prépare ce paiement — son montant est peut-être déjà réservé. Vos fonds sont en sécurité, mais il ne pourra aboutir qu\'une fois que votre portefeuille se synchronisera à nouveau.';

  @override
  String get walletParkedSendNow => 'Envoyer maintenant';

  @override
  String walletParkedSendNowInProgressSemanticTimed(
    String amount,
    String time,
  ) {
    return 'Envoi en cours du paiement de $amount enregistré $time';
  }

  @override
  String walletParkedSendNowSemanticTimed(String amount, String time) {
    return 'Envoyer maintenant le paiement de $amount enregistré $time';
  }

  @override
  String get walletParkedSendNowInProgress => 'Envoi en cours…';

  @override
  String get walletParkedAuthorizeSent => 'Envoi de votre paiement en cours.';

  @override
  String get walletParkedAuthorizeSentSyncPaused =>
      'Envoi de votre paiement en cours. S\'il n\'aboutit pas, votre portefeuille ne pourra le finaliser qu\'une fois qu\'il se synchronisera à nouveau.';

  @override
  String get walletParkedAuthorizeStillWaiting =>
      'Pas encore prêt à être envoyé. Votre paiement est enregistré et inchangé.';

  @override
  String get walletParkedAuthorizeRearmed =>
      'Pas encore prêt à être envoyé. Votre paiement est enregistré et n\'est plus en pause — réessayez plus tard avec « Envoyer maintenant », ou annulez-le.';

  @override
  String get walletParkedAuthorizeFailed =>
      'Impossible de l\'envoyer pour le moment. Le paiement est inchangé. Réessayez.';

  @override
  String get walletTransparentFundsMenuItem => 'Fonds publics…';

  @override
  String get walletTransparentFundsTitle => 'Fonds publics';

  @override
  String get walletTransparentFundsIntro =>
      'Les fonds publics sont visibles publiquement sur la blockchain — le montant, les adresses et l\'historique des pièces.';

  @override
  String get walletExpertToggleLabel => 'Avancé : fonds publics';

  @override
  String get walletExpertToggleDescription =>
      'Afficher les réglages avancés pour conserver des fonds publics et désactiver la protection automatique.';

  @override
  String get walletExpertToggleDescriptionNoAutoShield =>
      'Afficher les réglages avancés pour conserver des fonds publics.';

  @override
  String get walletAutoShieldToggleLabel => 'Protéger automatiquement';

  @override
  String walletAutoShieldToggleDescription(String minZec) {
    return 'Lorsque votre solde public atteint $minZec ZEC, il est automatiquement déplacé vers votre solde protégé. Si cette option est désactivée, les fonds publics restent visibles publiquement jusqu\'à ce que vous les protégiez vous-même.';
  }

  @override
  String get walletSettingsSaveFailed =>
      'Impossible d\'enregistrer le paramètre. Réessayez.';

  @override
  String get walletAutoShieldIncomplete =>
      'La protection automatique ne s\'est pas terminée — ces fonds sont toujours visibles publiquement. Vous pouvez les protéger maintenant.';

  @override
  String get walletSendPrivacyShielded =>
      'Paiement protégé — le montant et le destinataire restent privés sur la chaîne.';

  @override
  String get walletSendPrivacyTransparent =>
      'Paiement public — le montant et les adresses sont visibles sur la blockchain.';

  @override
  String get walletActivityPublicBadge =>
      'Visible publiquement sur la blockchain';

  @override
  String get walletShieldWalletEnded =>
      'La session du portefeuille s\'est terminée. Fermez puis rouvrez pour réessayer.';

  @override
  String walletTransparentFundsAutoOn(String minZec) {
    return 'Les nouveaux fonds publics sont protégés automatiquement vers votre solde privé dès qu\'ils atteignent $minZec ZEC.';
  }

  @override
  String get walletTransparentFundsAutoOff =>
      'La protection automatique est désactivée — les fonds publics restent visibles publiquement jusqu\'à ce que vous les protégiez.';

  @override
  String get walletMoveAutoShieldNote =>
      'La protection automatique est activée : une fois ces fonds arrivés, ils seront à nouveau protégés automatiquement (moyennant des frais supplémentaires). Pour les conserver publics, désactivez d\'abord la protection automatique dans Fonds publics.';

  @override
  String walletMoveBelowFloorNote(String amount, String floor) {
    return 'Après ce déplacement, votre solde public sera de $amount ZEC — en dessous des $floor ZEC nécessaires pour le protéger à nouveau. Il reste public jusqu\'à l\'arrivée de fonds supplémentaires.';
  }

  @override
  String get walletMoveOwnAddressNoteStaysPublic =>
      'Vous déplacez des fonds vers votre propre adresse publique. Ce déplacement reste inscrit publiquement de façon permanente.';

  @override
  String get walletTxDetailVisibility => 'Visibilité';

  @override
  String get walletTransparentFundsAutoDenied =>
      'La protection automatique est en pause pour cette session — elle n\'a pas été approuvée. Vous pouvez toujours protéger vos fonds manuellement.';

  @override
  String get walletDeepScanMenuItem =>
      'Vérifier les anciennes adresses d\'échange…';

  @override
  String get walletMenuSyncNotRunningHint =>
      'La synchronisation n\'est pas en cours pour le moment.';

  @override
  String get walletDeepScanTitle =>
      'Vérifier les anciennes adresses d\'échange';

  @override
  String get walletDeepScanBody =>
      'Si vous avez restauré ce portefeuille et qu\'il a beaucoup utilisé les échanges par le passé, les fonds de ses échanges les plus anciens peuvent nécessiter une étape supplémentaire pour être retrouvés. Cette vérification les recherche — tout ce qui est trouvé apparaît dans votre solde à mesure que votre portefeuille se synchronise.';

  @override
  String get walletDeepScanCoverage =>
      'Vos anciennes adresses d\'échange sont vérifiées jusqu\'ici. Si des fonds provenant d\'un ancien échange semblent toujours manquants, vérifiez des adresses encore plus anciennes.';

  @override
  String get walletDeepScanCoveragePending =>
      'Vérification de la plage actuelle toujours en cours — tout ce qui est trouvé apparaît dans votre solde. Cela peut prendre un peu de temps.';

  @override
  String get walletDeepScanCoverageUnknown =>
      'Recherche des fonds provenant des échanges les plus anciens de votre portefeuille.';

  @override
  String get walletDeepScanCheckButton =>
      'Vérifier les adresses plus anciennes';

  @override
  String get walletDeepScanCheckDeeperButton =>
      'Vérifier des adresses encore plus anciennes';

  @override
  String get walletDeepScanChecking => 'Vérification…';

  @override
  String get walletDeepScanClose => 'Fermer';

  @override
  String get walletDeepScanTorHint =>
      'Vous n\'êtes pas connecté via Tor pour le moment. Pour plus de confidentialité, envisagez d\'attendre que Tor soit actif avant de lancer la vérification.';

  @override
  String get walletDeepScanRescanBusy =>
      'Vous pourrez vérifier les anciennes adresses d\'échange une fois la réanalyse terminée.';

  @override
  String get walletDeepScanRan =>
      'Vérification des anciennes adresses d\'échange — tout ce qui est trouvé apparaîtra dans votre solde.';

  @override
  String get walletDeepScanFailed =>
      'Impossible de démarrer la vérification. Rien n\'a changé — réessayez.';

  @override
  String get walletDeepScanSlow =>
      'Cela prend plus de temps que d\'habitude. Si vos anciennes adresses d\'échange ont été vérifiées, tout ce qui est trouvé apparaîtra dans votre solde — vérifiez à nouveau sous peu.';

  @override
  String get walletDeepScanRefusedDisabled =>
      'L\'échange est désactivé pour le moment, cette action est donc impossible. Réessayez lorsque l\'échange sera disponible.';

  @override
  String get walletDeepScanRefusedOutstanding =>
      'La vérification de la dernière plage est encore en cours — cela peut prendre jusqu\'à deux jours, mais généralement beaucoup moins. Elle se termine d\'elle-même ; vérifiez à nouveau plus tard.';

  @override
  String get walletDeepScanTorUnknownHint =>
      'Nous ne pouvons pas encore confirmer la confidentialité de votre connexion. Pour plus de confidentialité, vérifiez une fois que Tor est actif.';

  @override
  String get walletDeepScanBannerChecking =>
      'Vérification des anciennes adresses d\'échange toujours en cours — tout ce qui est trouvé apparaîtra dans votre solde.';

  @override
  String get walletRescanSwapPointer =>
      'Vous cherchez des fonds provenant d\'un ancien échange ? Une réanalyse ne les trouvera pas — utilisez plutôt « Vérifier les anciennes adresses d\'échange ».';

  @override
  String get walletDeepScanRestoreNoteTitle =>
      'Avez-vous restauré un portefeuille qui utilisait des échanges ?';

  @override
  String get walletDeepScanRestoreNoteBody =>
      'Si ce portefeuille avait un très long historique d\'échanges, les fonds de ses échanges les plus anciens peuvent nécessiter une étape supplémentaire pour être retrouvés. La plupart des portefeuilles n\'ont besoin de rien.';

  @override
  String get walletDeepScanRestoreNoteCheck => 'Vérifier maintenant';

  @override
  String get walletDeepScanRestoreNoteDismiss => 'Ignorer';

  @override
  String walletTorHostPath(String transport) {
    return 'Via le chemin privé de votre application ($transport)';
  }

  @override
  String walletTorHostPathLinkable(String transport) {
    return 'Via le chemin privé de votre application ($transport) ; le proxy peut relier les connexions';
  }

  @override
  String get walletTorHostOtherTransport => 'un chemin privé';

  @override
  String get walletTorHostDirect =>
      'Non privé (connexion directe de votre application)';

  @override
  String walletSyncServerFallbackRefusedByTransport(String host) {
    return 'Votre serveur enregistré utilise une adresse non chiffrée, que le chemin privé de votre application ne peut pas acheminer. $host est utilisé.';
  }

  @override
  String walletInfoButtonLabel(String label) {
    return 'En savoir plus sur $label';
  }

  @override
  String get walletSendPaste => 'Coller';

  @override
  String get walletSendScanQr => 'Scanner un code QR';

  @override
  String get walletSendRecipientGetsLabel => 'Le destinataire reçoit';

  @override
  String get walletSwapDepositCopyAmount => 'Copier le montant';

  @override
  String get walletSwapDepositAmountCopied => 'Montant copié';

  @override
  String get walletScanOpenSettings => 'Ouvrir les réglages';

  @override
  String get walletScanOpenSettingsFailed =>
      'Impossible d\'ouvrir les réglages.';

  @override
  String get walletSendLeaveTitle => 'Envoi en cours';

  @override
  String get walletSendLeaveBody =>
      'Votre paiement continue si vous partez. Vous verrez son résultat dans votre activité.';

  @override
  String get walletSendLeaveStay => 'Rester';

  @override
  String get walletSendLeaveConfirm => 'Partir';

  @override
  String get walletSheetLeaveBody =>
      'Cela continue si vous partez. Vous verrez son résultat dans votre activité.';

  @override
  String get walletLoadingLabel => 'Chargement';

  @override
  String get walletSendUnknownTitle => 'Check before sending again';

  @override
  String get walletSendUnknownBody =>
      'We couldn\'t confirm this payment. Check Activity before sending it again.';

  @override
  String get walletSendUnknownQueuedBody =>
      'We couldn\'t confirm this payment was saved. Check your pending payments before sending it again.';

  @override
  String get walletShieldUnknownTitle => 'Vérifiez avant de protéger à nouveau';

  @override
  String get walletShieldUnknownBody =>
      'Nous n\'avons pas pu confirmer cette protection. Consultez Activité avant de réessayer.';

  @override
  String get walletMoveUnknownTitle => 'Vérifiez avant de déplacer à nouveau';

  @override
  String get walletMoveUnknownBody =>
      'Nous n\'avons pas pu confirmer ce déplacement. Consultez Activité avant de réessayer.';

  @override
  String get walletTxExplainRetryingExpired =>
      'This attempt expired before the network confirmed it. Your wallet will send the payment again by itself — don\'t send it again yourself.';
}
