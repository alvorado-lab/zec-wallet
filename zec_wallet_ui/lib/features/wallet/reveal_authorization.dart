import 'package:flutter_riverpod/flutter_riverpod.dart';

/// The host RE-AUTH seam for revealing the recovery phrase (#333).
///
/// Showing the recovery words in Settings is a different threat model from
/// showing them once during onboarding: onboarding runs seconds after the user
/// chose to create a wallet, but a Settings reveal can happen at any time — on a
/// device someone else picked up already unlocked. Custody models that hold a
/// re-authentication capability (an app passphrase, a biometric/device
/// credential) want to gate the reveal behind it, exactly as they gate a spend
/// behind [WalletSendAuthorizer].
///
/// A reveal is NOT a spend, so it deliberately does NOT reuse
/// `walletSendAuthorizerProvider` (whose [WalletSpendKind] semantics and
/// deferred-signing caveats are money-path only). It is also a simpler contract:
/// there is no signing credential to stage and re-lock, so this is a plain
/// GATE ([authorizeReveal] resolves iff the user re-authenticated, throws
/// [WalletRevealReauthDenied] if they cancelled) rather than the action-wrapper
/// the send seam needs. The words themselves never pass through this seam — they
/// are read from the `autoDispose` reveal future the backup screen owns, so the
/// §10 key-residue minimisation is unchanged whether or not a host wires re-auth.
///
/// SCOPE (do not over-read the send-seam parity): this is a re-auth LIVENESS
/// check — "is the person in front of the device allowed to see the phrase?" —
/// NOT a decrypt-credential carrier. Unlike [WalletSendAuthorizer] it hands your
/// [authorizeReveal] nothing to scope and gives you no post-reveal re-lock hook,
/// because the SDK's own `revealMnemonic` reads the already-open handle and needs
/// no per-reveal credential. A host whose custody DERIVES the phrase from a
/// per-reveal secret must carry that secret INSIDE its own `revealMnemonic`
/// override; this gate only fronts it with the user re-auth.
///
/// DEFAULT ([WalletPassthroughRevealAuthorizer]): no host re-auth step. The
/// backup screen's own deliberate reveal action + the on-screen warning + the
/// platform screenshot/recents protection are the reference-app posture; a
/// production host with a real credential overrides
/// [walletRevealAuthorizerProvider] to add its prompt. This mirrors the
/// send-authorizer default (the SDK acts whenever asked for sealed-keychain
/// custody), so a host that needs neither wires nothing.
abstract interface class WalletRevealAuthorizer {
  /// Re-authenticate the user before the recovery phrase is revealed. A host
  /// implementation shows its own credential prompt (it owns a navigator; this
  /// seam passes no BuildContext, matching [WalletSendAuthorizer]) and:
  ///
  ///  * resolves the future when the user PASSES re-auth — the backup screen
  ///    then reads the words from the reveal future, or
  ///  * throws [WalletRevealReauthDenied] when the user CANCELS — the backup
  ///    screen silently stays on its pre-reveal state (no words are read; the
  ///    host's own prompt already told the user why), or
  ///  * lets any OTHER error propagate — the backup screen treats it as a
  ///    reveal failure and offers a retry (never a false "revealed").
  ///
  /// MUST always complete (resolve or throw): the reveal button holds its busy
  /// state until it does, so the prompt must be cancelable. The package applies
  /// NO timeout — re-auth legitimately takes as long as the user takes.
  ///
  /// WHAT A PASS IS GOOD FOR (S2): a grant is valid for one wallet session
  /// instance, one generation and one foreground session. The package binds
  /// the pass to the wallet session in the gate when the prompt was raised and
  /// to the wallet's lifecycle generation; if the wallet changes under the
  /// screen — a delete, a server switch, a rescan, the same wallet re-opened,
  /// a host account switch — the pass is void: nothing of the new wallet is
  /// read or shown, and the next reveal calls this again. A background hides
  /// the secret and a re-reveal calls this again too.
  Future<void> authorizeReveal();
}

/// The default: no host re-auth step. Correct for the reference app and any
/// sealed-keychain host that gates reveal by the on-screen deliberate action
/// alone. Resolves immediately.
final class WalletPassthroughRevealAuthorizer
    implements WalletRevealAuthorizer {
  const WalletPassthroughRevealAuthorizer();

  @override
  Future<void> authorizeReveal() async {}
}

/// Thrown by a [WalletRevealAuthorizer] when the user cancels re-auth. The
/// backup screen catches it BEFORE reading the words and silently returns to
/// its pre-reveal state — the host's own prompt is the user-facing channel for
/// the cancel, so the package shows no additional fault (mirrors
/// [WalletSpendAuthorizationDenied]).
final class WalletRevealReauthDenied implements Exception {
  const WalletRevealReauthDenied();
}

/// The host re-auth seam source. PRODUCTION default is the passthrough (no
/// re-auth step); a host with a credential overrides this at the ProviderScope
/// root, exactly as it overrides `walletSendAuthorizerProvider`. Tests inject a
/// fake to assert the reveal is gated (passes → words shown; denied → no words).
final walletRevealAuthorizerProvider = Provider<WalletRevealAuthorizer>(
  (ref) => const WalletPassthroughRevealAuthorizer(),
);
