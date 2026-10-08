import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/lifecycle/app_lifecycle_provider.dart';
import '../../core/router/wallet_navigation.dart';
import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../shared/wallet_cta.dart';
import '../../shared/wallet_notice.dart';
import '../wallet/onboarding/mnemonic_reveal.dart';
import '../wallet/onboarding/onboarding_providers.dart';
import '../wallet/recovery_phrase_widgets.dart';
import '../wallet/reveal_authorization.dart';
import '../wallet/reveal_grant.dart';
import '../wallet/wallet_providers.dart' show walletSessionProvider;
import 'managed_by_host.dart';

/// Settings → Security → **Back up recovery phrase** (#333): the POST-onboarding
/// twin of the onboarding backup step. Onboarding shows the words once, behind a
/// money-safety gate, seconds after the user chose to create the wallet; this
/// screen lets them see the words again at any later time — a different threat
/// model, so it adds a host RE-AUTH gate ([WalletRevealAuthorizer]) that a
/// custody model with a credential can plug its prompt into. It is VIEW-ONLY:
/// there is no confirm checkbox and nothing is persisted (the onboarding gate
/// already recorded the backup); the user reads the words and leaves.
///
/// KEY RESIDUE (spec §10): the words are read via the `autoDispose`
/// [grantedMnemonicRevealProvider] — watched ONLY after re-auth passes, only
/// while its grant is current, and dropped the instant this screen unmounts,
/// the app backgrounds (the auto-hide below) or the wallet changes under it. As
/// on the onboarding view they are shown as plain (non-selectable, non-copyable)
/// text and are never copied into this State, persisted, or logged.
///
/// SEED-OWNERSHIP CASES (the maintainer's seed-backup Q&A):
///  * Case 1 (the SDK generated / restored the seed — [walletProvisionerProvider]
///    is wired and holds a mnemonic): reveal the words here.
///  * Case 2 (a raw-seed / host-managed wallet — session-only mode leaves the
///    provisioner null, or a host provisioner whose `revealMnemonic` throws
///    [WalletErrorKind.noMnemonic]): there is NO wallet-local phrase; the honest
///    state tells the user their recovery lives in the app that set the wallet
///    up, never inventing words that don't exist.
class BackupScreen extends ConsumerStatefulWidget {
  const BackupScreen({super.key});

  @override
  ConsumerState<BackupScreen> createState() => _BackupScreenState();
}

class _BackupScreenState extends ConsumerState<BackupScreen> {
  /// Shoulder-surfing mitigation: words stay hidden until a deliberate reveal
  /// tap that PASSES re-auth — and then show only while the [RevealGrant] that
  /// re-auth minted is still current (the same wallet session instance, the
  /// same operation generation; S2 §3.4 — the viewing-key export's grant: one
  /// type, two consumers). Dropped on background (the auto-hide) and on any
  /// identity change under the mounted screen.
  RevealGrant? _grant;

  /// Re-auth is in flight (the host prompt is up) — the reveal button shows a
  /// spinner and is single-flighted so a double-tap can't stack two prompts.
  bool _authorizing = false;

  /// A non-cancel re-auth fault (the host authorizer threw something other than
  /// [WalletRevealReauthDenied]) — shows an honest retry above the reveal
  /// button. A user CANCEL is silent (no fault), matching the send flow.
  bool _reauthFault = false;

  /// Captured in [initState] so [dispose] can release protection WITHOUT
  /// touching `ref` (Riverpod guidance: `ref` is unsafe once deactivated).
  late final ScreenSecurity _security;

  /// True only after the native side ACKNOWLEDGED the screenshot block (
  /// review B1) — pessimistic until the ack, so a host that never wired the
  /// FLAG_SECURE handler reads "screenshots possible", never a false claim.
  bool _screenshotsBlocked = false;

  @override
  void initState() {
    super.initState();
    // Request screenshot/recents protection for this screen's whole life (words
    // can appear on it). Best-effort defence-in-depth, NOT the gate: a failure
    // still shows the screen with the honest unprotected copy.
    _security = ref.read(screenSecurityProvider);
    unawaited(
      _security.enable().then((engaged) {
        if (engaged && mounted) setState(() => _screenshotsBlocked = true);
      }),
    );
  }

  @override
  void dispose() {
    unawaited(_security.disable());
    super.dispose();
  }

  Future<void> _reveal() async {
    // SYNCHRONOUS re-entrancy guard, not just the button's `onPressed` disable:
    // `setState` schedules the rebuild for the NEXT frame, but Flutter drains
    // pointer events as they arrive, so two taps in one frame would both see the
    // old `onPressed: _reveal` and stack TWO host credential prompts. This
    // short-circuits the second synchronously (the button-disable is the
    // visible affordance; this is the actual single-flight).
    if (_authorizing) return;
    setState(() {
      _authorizing = true;
      _reauthFault = false;
    });
    try {
      // The host RE-AUTH gate. Default passthrough (no step); a credential
      // host shows its prompt and passes/denies. The pass comes back bound to
      // the session and generation current when the prompt was raised.
      final grant = await authorizeRevealGrant(ref);
      if (!mounted) return;
      // The grant's first check: a wallet identity change while the prompt
      // was up voids the pass — nothing is unsealed. Passed and current: keep
      // the grant — the build then starts watching the autoDispose reveal
      // future keyed by it (the unseal happens only now, post-auth).
      final current = revealGrantIsCurrent(
        ref,
        grant,
        currentSession: ref.read(walletSessionProvider),
      );
      setState(() {
        _authorizing = false;
        if (current) _grant = grant;
      });
    } on WalletRevealReauthDenied {
      // User cancelled re-auth. Silent — the host's own prompt already
      // communicated it; just return to the pre-reveal state.
      if (!mounted) return;
      setState(() => _authorizing = false);
    } catch (_) {
      // Any OTHER re-auth error is a real fault. No payload is read (§5.4 — it
      // could carry sensitive context); one honest retry line, no words shown.
      if (!mounted) return;
      setState(() {
        _authorizing = false;
        _reauthFault = true;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);

    // AUTO-HIDE ON BACKGROUND (the cross-platform shoulder-surf / recents-
    // snapshot mitigation, identical to the onboarding backup view): FLAG_SECURE
    // is Android-only, so on iOS/desktop the app-switcher/window snapshot taken
    // when the app is BACKGROUNDED would otherwise capture the on-screen seed.
    // Collapse to pre-reveal on the real background moment — `hidden` (the OS
    // takes the switcher snapshot; also desktop minimize) and `paused` (full
    // Android background) — which unwatches the autoDispose reveal provider and
    // drops the cached words (NO explicit invalidate — invalidating while still
    // watched would RE-RUN the reveal and re-fetch the seed). NOT on `inactive`
    // (transient foreground focus-noise — notification shade, alt-tab — takes no
    // snapshot; hiding there would clear on every interruption and make desktop
    // multitasking unusable). `ref.listen` in build is the sanctioned spot
    // (mirrors the onboarding view + syncStatus lifecycle listen). Gating on
    // the grant alone is enough: `_authorizing` only overlaps the pre-reveal
    // state (no grant yet), so a background during an in-flight re-auth is
    // already a no-op here — the re-auth just resolves on the way back.
    // Dropping the grant is what makes a re-reveal after resume re-authorize.
    ref.listen<AppLifecycleState>(appLifecycleProvider, (_, next) {
      final backgrounding =
          next == AppLifecycleState.hidden || next == AppLifecycleState.paused;
      if (backgrounding && _grant != null) {
        setState(() => _grant = null);
      }
    });

    return Scaffold(
      appBar: AppBar(title: Text(l10n.walletBackupScreenTitle)),
      body: SafeArea(
        child: SingleChildScrollView(
          // The 16 side gutter (S13 Build B).
          padding: const EdgeInsets.fromLTRB(16, 24, 16, 24),
          child: _body(context, l10n),
        ),
      ),
    );
  }

  Widget _body(BuildContext context, WalletLocalizations l10n) {
    // Case 2 (managed-by-host), detected up front WITHOUT any reveal attempt:
    // session-only mode leaves the provisioner null, so there is no wallet-local
    // phrase to show. Honest terminal state — no reveal button, no re-auth
    // dead-end. (A host provisioner that instead THROWS noMnemonic is caught in
    // the reveal error arm below and lands on the same state.)
    if (ref.watch(walletProvisionerProvider) == null) {
      return const WalletManagedByHost();
    }

    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);

    // The grant's check before every render. WATCHING the session is what
    // rebuilds this screen when the wallet identity changes under it (a
    // delete, a switch, the same wallet re-opened); a grant minted for another
    // session instance or generation is dropped here, so the reveal provider
    // it keyed is unwatched (and disposed) and nothing is shown.
    final session = ref.watch(walletSessionProvider);
    final grant = _grant;
    if (grant != null &&
        !revealGrantIsCurrent(ref, grant, currentSession: session)) {
      _grant = null;
    }
    final live = _grant;
    if (live != null) {
      return _revealedBody(context, l10n, textTheme, colors, live);
    }
    return _sealedBody(l10n, textTheme);
  }

  /// Pre-reveal: the body copy, the secure note and the reveal button.
  Widget _sealedBody(WalletLocalizations l10n, TextTheme textTheme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(l10n.walletBackupBody, style: textTheme.bodyMedium),
        const SizedBox(height: 16),
        RecoveryPhraseSecureNote(blocksScreenshots: _screenshotsBlocked),
        const SizedBox(height: 24),
        if (_reauthFault) ...[
          // A RE-AUTH failure (the host credential step threw) — NOT a device
          // read failure, so it doesn't tell the user to unlock their device.
          WalletNotice(
            tone: WalletNoticeTone.warning,
            glyph: WalletGlyph.error,
            message: l10n.walletBackupReauthFailed,
            // S11 C7: a fault that APPEARS after the async re-auth must be
            // announced; this copy had lost the live region its twin on the
            // viewing-key screen carries.
            liveRegion: true,
          ),
          const SizedBox(height: 16),
        ],
        WalletCta(
          child: FilledButton.icon(
            onPressed: _authorizing ? null : _reveal,
            icon: _authorizing
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                  )
                : const WalletIcon(WalletGlyph.reveal),
            label: Text(
              _reauthFault
                  ? l10n.walletBackupRetryReveal
                  : l10n.walletBackupReveal,
            ),
          ),
        ),
      ],
    );
  }

  /// Post-reveal: watch the autoDispose reveal future keyed by [grant]. Reached
  /// only after re-auth passed and only while [grant] is current.
  Widget _revealedBody(
    BuildContext context,
    WalletLocalizations l10n,
    TextTheme textTheme,
    WalletColors colors,
    RevealGrant grant,
  ) {
    final words = ref.watch(grantedMnemonicRevealProvider(grant));
    return words.when(
      loading: () => Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Column(
          children: [
            const CircularProgressIndicator.adaptive(),
            const SizedBox(height: 16),
            Text(l10n.walletBackupRevealing, style: textTheme.bodyMedium),
          ],
        ),
      ),
      error: (error, _) {
        // The grant went void between the render check and the unseal:
        // nothing was read. Back to the un-revealed state; a reveal
        // re-authorizes.
        if (error is RevealGrantVoid) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (mounted && identical(_grant, grant)) {
              setState(() => _grant = null);
            }
          });
          return _sealedBody(l10n, textTheme);
        }
        // A raw-seed / host-managed wallet has NO local phrase — the SDK throws
        // noMnemonic. Classify by KIND only (a structural discriminant, not a
        // sensitive payload — §5.4) and show the honest managed-by-host state
        // rather than the transient "try again". Any other error IS transient
        // (device locked, a keystore hiccup): one honest retry that re-runs the
        // reveal.
        final isManaged =
            error is WalletApiError &&
            error.kind == const WalletErrorKind.noMnemonic();
        if (isManaged) return const WalletManagedByHost();
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            RecoveryPhraseSecureNote(blocksScreenshots: _screenshotsBlocked),
            const SizedBox(height: 24),
            WalletNotice(
              tone: WalletNoticeTone.warning,
              glyph: WalletGlyph.error,
              message: l10n.walletBackupRevealFailed,
              liveRegion: true,
            ),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: () =>
                  ref.invalidate(grantedMnemonicRevealProvider(grant)),
              icon: const WalletIcon(WalletGlyph.retry),
              label: Text(l10n.walletBackupRetryReveal),
            ),
          ],
        );
      },
      data: (revealed) {
        final wordsList = revealed.words;
        // Defensive: a well-behaved SDK Case-1 wallet always returns a full
        // phrase, and a raw-seed wallet throws noMnemonic (handled above). But a
        // misbehaving host provisioner that returns an EMPTY list instead of
        // throwing must NOT render blank "backup" chrome around no words — the
        // honest surface for "no phrase to show here" is managed-by-host.
        if (wordsList.isEmpty) return const WalletManagedByHost();
        // The body is shown ONCE (S13 Build B): the user read it on the
        // sealed screen before choosing to reveal; the words take its place.
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            RecoveryPhraseSecureNote(blocksScreenshots: _screenshotsBlocked),
            const SizedBox(height: 24),
            RecoveryWordGrid(words: wordsList),
            const SizedBox(height: 24),
            WalletCta(
              child: FilledButton(
                onPressed: () => context.leaveToWalletRoot(),
                child: Text(l10n.walletBackupDone),
              ),
            ),
          ],
        );
      },
    );
  }
}
