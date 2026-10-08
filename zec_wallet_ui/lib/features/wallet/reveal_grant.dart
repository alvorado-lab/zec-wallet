import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'onboarding/onboarding_controller.dart';
import 'reveal_authorization.dart';
import 'wallet_providers.dart' show walletSessionProvider;
import 'wallet_session.dart';

/// What a passed reveal re-auth is good for (S2 §3.4, the review's R06): ONE
/// wallet session instance under ONE operation generation. Package-internal —
/// deliberately NOT barrel-exported, so [WalletRevealAuthorizer]'s shape is
/// unchanged for a host: the host still answers "may this person see it?",
/// and the package decides WHAT that answer is spent on.
///
/// Minted by [authorizeRevealGrant] with the session and generation read
/// BEFORE the host prompt, so an identity change while the prompt is up voids
/// the answer. The reveal step checks it twice: before the secret is read
/// (the grant-keyed reveal providers throw [RevealGrantVoid] rather than read)
/// and again before every render (the screen drops a grant that is no longer
/// [isCurrent] and shows the un-revealed state). A backgrounded screen drops
/// its grant too, so a grant never outlives the foreground session it was
/// granted in.
///
/// The binding is to the session INSTANCE — `walletSessionProvider`'s value
/// identity, which is what a screen can observe — never to value equality:
/// the same wallet re-opened under a fresh session voids the grant and costs
/// one re-prompt (the safe side).
final class RevealGrant {
  const RevealGrant._(this.session, this._generation);

  /// The session instance the grant was minted under — `null` for a mount
  /// with no session in the gate (a provisioner-only mount outside
  /// `OnboardingActive`). The session-only export reads through it.
  final WalletSession? session;

  final int _generation;

  /// True iff [currentSession] is the very instance this grant was minted
  /// under and [currentGeneration] is the generation it was minted in.
  bool isCurrent({
    required WalletSession? currentSession,
    required int currentGeneration,
  }) => identical(session, currentSession) && _generation == currentGeneration;

  @override
  String toString() => 'RevealGrant(generation $_generation)';
}

/// Thrown by a grant-keyed reveal provider when the grant it was asked to
/// read under is no longer current — the read did not happen.
final class RevealGrantVoid implements Exception {
  const RevealGrantVoid();
}

/// The controller's operation generation — the second half of a grant.
int _generationOf(OnboardingController controller) =>
    controller.operationGeneration;

/// Ask the host's [WalletRevealAuthorizer] (unchanged contract: resolve on a
/// pass, throw [WalletRevealReauthDenied] on a cancel, anything else is a
/// fault) and bind the pass to the session and generation that were current
/// when the prompt was raised. The caller still checks [RevealGrant.isCurrent]
/// against the present before it acts on the grant.
Future<RevealGrant> authorizeRevealGrant(WidgetRef ref) async {
  final session = ref.read(walletSessionProvider);
  final generation = _generationOf(
    ref.read(onboardingControllerProvider.notifier),
  );
  await ref.read(walletRevealAuthorizerProvider).authorizeReveal();
  return RevealGrant._(session, generation);
}

/// [grant] checked against the present, from a widget. [currentSession] is
/// passed in so a build can WATCH the session (and rebuild on its change)
/// while an event handler only reads it.
bool revealGrantIsCurrent(
  WidgetRef ref,
  RevealGrant grant, {
  required WalletSession? currentSession,
}) => grant.isCurrent(
  currentSession: currentSession,
  currentGeneration: _generationOf(
    ref.read(onboardingControllerProvider.notifier),
  ),
);

/// [grant] checked against the present, from inside a provider — the check a
/// grant-keyed reveal provider runs immediately before it reads the secret.
/// READS only: a reveal provider must never re-run on its own when the
/// session changes (that re-run is exactly the read R06 forbids).
bool revealGrantIsCurrentIn(Ref ref, RevealGrant grant) => grant.isCurrent(
  currentSession: ref.read(walletSessionProvider),
  currentGeneration: _generationOf(
    ref.read(onboardingControllerProvider.notifier),
  ),
);
