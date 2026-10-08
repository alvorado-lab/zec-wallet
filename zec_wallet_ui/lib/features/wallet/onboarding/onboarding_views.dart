import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart' show SemanticsService;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/lifecycle/app_lifecycle_provider.dart';
import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import '../../../core/theme/shapes.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/wallet_cta.dart';
import '../../../shared/wallet_dialog.dart';
import '../../../shared/wallet_info_button.dart';
import '../recovery_phrase_widgets.dart';
import '../swap/swap_address_scanner.dart';
import '../wallet_config.dart';
import 'bip39_wordlist.dart';
import 'mnemonic_pill_field.dart';
import 'mnemonic_reveal.dart';
import 'onboarding_controller.dart';
import 'onboarding_providers.dart';
import 'onboarding_state.dart';

/// The wallet onboarding views (spec §3.2g iii-B-2-a): the visible half of the
/// create → back-up → confirm → active flow, over the already-shipped
/// [OnboardingController] money-safety gate. Rendering layer ONLY — every
/// transition runs through the controller; no state is stored here (design
/// invariant 1). The controller's state machine is the single source of truth,
/// so each view is a thin renderer keyed to one phase. The wallet surface only
/// becomes deposit-ready in [OnboardingActive], which is reachable solely
/// through the confirmed-and-persisted backup ([WalletBackupView]).

/// Every onboarding page's padding: the 16 side gutter (S13 Build B).
@visibleForTesting
const EdgeInsets onboardingPagePadding = EdgeInsets.fromLTRB(16, 24, 16, 24);

/// A step's title with its explanation behind an (i) after it (S13 Build B;
/// DESIGN §6.20: no paragraphs — the explanation moves WHOLE under its
/// existing string). The title stays on screen; it is not the AppBar's.
@visibleForTesting
class OnboardingTitle extends StatelessWidget {
  const OnboardingTitle({
    super.key,
    required this.title,
    this.explanation,
    this.center = false,
  });

  final String title;

  /// The (i)'s body; null draws the title alone — for a step whose body is a
  /// money-safety warning that must stay on screen (Restore).
  final String? explanation;

  /// Centred (the Welcome and Failed layouts) rather than start-aligned.
  final bool center;

  @override
  Widget build(BuildContext context) {
    final text = Text(
      title,
      style: Theme.of(context).textTheme.headlineSmall,
      textAlign: center ? TextAlign.center : TextAlign.start,
    );
    final explanation = this.explanation;
    return Row(
      mainAxisAlignment: center
          ? MainAxisAlignment.center
          : MainAxisAlignment.start,
      children: [
        if (center) Flexible(child: text) else Expanded(child: text),
        if (explanation != null)
          WalletInfoButton(label: title, body: explanation),
      ],
    );
  }
}

// ---------------------------------------------------------------------------
// Welcome
// ---------------------------------------------------------------------------

/// [OnboardingWelcome]: no wallet on disk → offer to create one OR restore from
/// an existing recovery phrase. Both are live, equal-weight provisioning entries
/// (create is primary/filled, restore secondary/outlined).
class WalletWelcomeView extends ConsumerWidget {
  const WalletWelcomeView({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    return Center(
      child: SingleChildScrollView(
        padding: onboardingPagePadding,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            WalletIcon(WalletGlyph.wallet, size: 48, color: colors.accent),
            const SizedBox(height: 16),
            // The body sits behind the title's (i) (S13 Build B; DESIGN
            // §6.20): an explanation, not a state.
            OnboardingTitle(
              title: l10n.walletOnboardingWelcomeTitle,
              explanation: l10n.walletOnboardingWelcomeBody,
              center: true,
            ),
            const SizedBox(height: 24),
            WalletCta(
              child: FilledButton.icon(
                // startCreate handles its own failures (→ OnboardingFailed); no
                // rethrow, the state IS the surface, so fire-and-forget is right.
                onPressed: () => ref
                    .read(onboardingControllerProvider.notifier)
                    .startCreate(),
                icon: const WalletIcon(WalletGlyph.add),
                label: Text(l10n.walletCreateButton),
              ),
            ),
            const SizedBox(height: 12),
            WalletCta(
              child: OutlinedButton.icon(
                // beginRestore is a synchronous state transition (Welcome →
                // RestoreInput); it drives `state`, so fire-and-forget is right.
                onPressed: () => ref
                    .read(onboardingControllerProvider.notifier)
                    .beginRestore(),
                icon: const WalletIcon(WalletGlyph.behind),
                label: Text(l10n.walletRestoreButton),
              ),
            ),
            const SizedBox(height: 12),
            WalletCta(
              child: OutlinedButton.icon(
                key: const ValueKey('wallet-welcome-watch-only'),
                // #397 §3.7 D5: enter the watch-only import flow (Welcome →
                // WatchOnlyInput). Synchronous state transition — fire-and-forget.
                onPressed: () => ref
                    .read(onboardingControllerProvider.notifier)
                    .beginWatchOnly(),
                icon: const WalletIcon(WalletGlyph.watchOnly),
                label: Text(l10n.walletWatchOnlyButton),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Busy (generating / confirming)
// ---------------------------------------------------------------------------

/// A neutral busy view for the transient phases ([OnboardingGenerating],
/// [OnboardingConfirming], and the boot probe). One coherent screen-reader node.
class WalletOnboardingBusyView extends StatelessWidget {
  const WalletOnboardingBusyView({required this.label, super.key});

  final String label;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Center(
      child: Semantics(
        container: true,
        excludeSemantics: true,
        label: label,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const CircularProgressIndicator.adaptive(),
            const SizedBox(height: 16),
            Text(
              label,
              style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Failed
// ---------------------------------------------------------------------------

/// [OnboardingFailed]: an honest, actionable message keyed on the failure
/// CATEGORY (never a raw code — design invariant 6). EVERY kind has a forward-path
/// action so a failure is never a dead-end (#251): `needsRecovery` → "Restore from
/// recovery phrase" (a deliberate, confirmed force-clear + restore — the funds are
/// recovered from the phrase); every other kind (including the rare, possibly-
/// transient `noVault`) → re-run the boot probe ("Try again").
///
/// The ONE exception is `configuration` (#317): a host-app config error fails
/// identically on every retry, so the deliberate choice is NO Retry button —
/// the copy says to report it to the app's developer, and offering "Try again"
/// would be the exact infinite lie-loop the dedicated kind exists to prevent
/// (security review N1). The #251 rule targets USER dead-ends the user can
/// escape; here the fix is the DEVELOPER's (a bad config), the failure is
/// near-unreachable for a package consumer (the walletOnboardingOverrides
/// ArgumentError belt fires first — see classifyOnboardingFailure), and the
/// stable RW-CFG code rides logs the developer reads. A passive "copy
/// diagnostic" affordance would be a strict improvement (turning "report this"
/// into something reportable) and is the recorded refinement (#318); it is NOT
/// a Retry, so it does not reopen the lie-loop this arm closes.
class WalletOnboardingFailedView extends ConsumerWidget {
  const WalletOnboardingFailedView({required this.kind, super.key});

  final OnboardingFailureKind kind;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    return Center(
      child: SingleChildScrollView(
        padding: onboardingPagePadding,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            WalletIcon(WalletGlyph.error, size: 48, color: colors.orange),
            const SizedBox(height: 16),
            Text(
              l10n.walletOnboardingFailedTitle,
              style: textTheme.headlineSmall,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 12),
            Text(
              _body(l10n, kind),
              style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
              textAlign: TextAlign.center,
            ),
            // EVERY failure kind offers a forward path — NEVER a dead-end (#251).
            // Retryable kinds (and the rare `noVault`, which may be a transient
            // keychain) re-run the boot probe; `needsRecovery` routes to recovery-
            // by-restore (the on-device key is gone but the funds are recoverable
            // from the phrase). A money-app must never trap the user with no way out.
            // `configuration` is the deliberate exception (see the class doc):
            // no button, because every available action would lie.
            if (kind != OnboardingFailureKind.configuration) ...[
              const SizedBox(height: 24),
              WalletCta(child: _action(context, ref, l10n, kind)),
            ],
          ],
        ),
      ),
    );
  }

  /// The forward-path action for [kind]. `needsRecovery` ⇒ a deliberate
  /// "Restore from recovery phrase" (gated by a funds-are-safe confirm before the
  /// force-clear); every other kind ⇒ re-run the boot probe. Guarantees a button
  /// is ALWAYS shown — no failure state is a dead-end (#251).
  static Widget _action(
    BuildContext context,
    WidgetRef ref,
    WalletLocalizations l10n,
    OnboardingFailureKind kind,
  ) {
    final notifier = ref.read(onboardingControllerProvider.notifier);
    if (kind == OnboardingFailureKind.needsRecovery) {
      return FilledButton.icon(
        onPressed: () async {
          // Deliberate (the SDK forbids auto-forcing a wipe): confirm — restating
          // that the funds are safe — BEFORE the force-clear + restore route.
          final confirmed = await _confirmRecover(context, l10n);
          if (confirmed) await notifier.recoverByRestore();
        },
        icon: const WalletIcon(WalletGlyph.recover),
        label: Text(l10n.walletOnboardingFailedRestoreAction),
      );
    }
    // Retryable kinds + noVault: re-run the boot probe (guarded to the failed
    // state); it drives `state`, so fire-and-forget is right.
    return FilledButton.icon(
      onPressed: () => notifier.retry(),
      icon: const WalletIcon(WalletGlyph.retry),
      label: Text(l10n.walletOnboardingRetry),
    );
  }

  /// Confirm the recovery — a deliberate, reassuring gate before the destructive
  /// force-clear. Restates the money-safety truth (funds live on-chain, controlled
  /// by the recovery phrase) so the user is never scared into thinking "delete"
  /// loses their coins.
  static Future<bool> _confirmRecover(
    BuildContext context,
    WalletLocalizations l10n,
  ) {
    // Destructive (stage S11 C3): it removes the unreadable wallet data. The
    // helper's dialog scrolls, never clips, at large text scales (the
    // reclaim-dialog rule).
    return showWalletConfirm(
      context,
      title: l10n.walletOnboardingRecoverConfirmTitle,
      body: l10n.walletOnboardingRecoverConfirmBody,
      cancelLabel: l10n.walletOnboardingRecoverConfirmCancel,
      confirmLabel: l10n.walletOnboardingFailedRestoreAction,
      kind: WalletConfirmKind.destructive,
    );
  }

  /// Category → honest body copy. Total over the enum (no default blind spot).
  static String _body(WalletLocalizations l10n, OnboardingFailureKind kind) {
    return switch (kind) {
      OnboardingFailureKind.deviceLocked =>
        l10n.walletOnboardingFailedDeviceLocked,
      OnboardingFailureKind.alreadyOpen =>
        l10n.walletOnboardingFailedAlreadyOpen,
      OnboardingFailureKind.needsRecovery =>
        l10n.walletOnboardingFailedNeedsRecovery,
      OnboardingFailureKind.storageFull =>
        l10n.walletOnboardingFailedStorageFull,
      OnboardingFailureKind.noVault => l10n.walletOnboardingFailedNoVault,
      OnboardingFailureKind.network => l10n.walletOnboardingFailedNetwork,
      OnboardingFailureKind.interruptedSetup =>
        l10n.walletOnboardingFailedInterruptedSetup,
      OnboardingFailureKind.configuration =>
        l10n.walletOnboardingFailedConfiguration,
      OnboardingFailureKind.unknown => l10n.walletOnboardingFailedUnknown,
    };
  }
}

// ---------------------------------------------------------------------------
// Backup (the money-safety screen — reveal recovery words, then confirm)
// ---------------------------------------------------------------------------

/// [OnboardingAwaitingBackup] / [OnboardingConfirming]: show the recovery words
/// (behind a deliberate Reveal tap), require an explicit confirmation, then
/// persist-and-activate via the controller. Stateful so it can (a) request
/// screenshot/recents protection for its whole lifetime via [ScreenSecurity],
/// and (b) hold the ephemeral reveal/confirm/saving UI flags.
///
/// Rendered for BOTH AwaitingBackup and the transient Confirming so the view
/// stays mounted across the confirm round-trip: on a persist failure the
/// controller reverts to AwaitingBackup and this same instance shows the honest
/// "couldn't save" cue without a dead-context crash; on success the controller
/// reaches Active and the wallet surface replaces this view.
///
/// KEY RESIDUE (spec §10): the words are read via the `autoDispose`
/// [mnemonicRevealProvider], so they are dropped the moment this view unmounts.
/// They are NEVER copied into this State, are shown as plain (non-selectable,
/// non-copyable) text — clipboard is shared/synced, a seed must never reach it —
/// and are never logged. Dart memory cannot be zeroized; the smallest lifetime
/// is the mitigation.
class WalletBackupView extends ConsumerStatefulWidget {
  const WalletBackupView({super.key});

  @override
  ConsumerState<WalletBackupView> createState() => _WalletBackupViewState();
}

class _WalletBackupViewState extends ConsumerState<WalletBackupView> {
  /// Shoulder-surfing mitigation: words are hidden until a deliberate tap.
  bool _revealed = false;

  /// The explicit money-safety confirmation (the checkbox).
  bool _confirmed = false;

  /// UI busy flag during the confirm persist — disables the inputs and shows a
  /// spinner. The controller's own [OnboardingConfirming] transient is the
  /// gate's SSOT and the re-entrancy guard; this local flag is just the
  /// view-layer double-tap/affordance, independent of which phase renders us.
  bool _saving = false;

  /// UI busy flag during a Start-over delete (#356-F2) — locks every action on
  /// the screen so a shred can't race the confirm persist (the controller's
  /// `_deletionInFlight` latch is the real single-flight; this is the
  /// view-layer affordance, as [_saving] is for confirm).
  bool _deleting = false;

  /// Captured in [initState] so [dispose] can release protection WITHOUT
  /// touching `ref` — `ref` is unsafe once the widget is deactivated (Riverpod
  /// guidance: save the provider state in a field for use in dispose).
  late final ScreenSecurity _security;

  /// True only after the native side ACKNOWLEDGED the block (review B1):
  /// the security note starts pessimistic and upgrades on the ack, so a host
  /// that never wired the native handler reads "screenshots possible" —
  /// never a false "screenshots are off" over a visible seed.
  bool _screenshotsBlocked = false;

  @override
  void initState() {
    super.initState();
    // Request screenshot/recents protection for the whole time words can show.
    // Best-effort defence-in-depth (NOT the gate): a failure still shows the
    // screen — it just keeps the honest unprotected copy. ref.read in
    // initState is allowed (no watch).
    _security = ref.read(screenSecurityProvider);
    unawaited(
      _security.enable().then((engaged) {
        if (engaged && mounted) setState(() => _screenshotsBlocked = true);
      }),
    );
  }

  @override
  void dispose() {
    // Release protection as the screen leaves the tree, via the captured handle.
    unawaited(_security.disable());
    super.dispose();
  }

  Future<void> _confirm() async {
    // Capture before the await (the view unmounts on success — Active replaces
    // it); the messenger/l10n must not be read off a deactivated context after.
    final messenger = ScaffoldMessenger.of(context);
    final l10n = WalletLocalizations.of(context);
    setState(() => _saving = true);
    try {
      // Persists the confirmation, THEN opens the gate. On success the
      // controller reaches Active and this view is replaced by the wallet
      // surface, so nothing below runs in that path.
      await ref.read(onboardingControllerProvider.notifier).confirmBackup();
      // Symmetric with the catch guard: on success this view is unmounted
      // (Active replaced it), so never touch state after the await.
      if (!mounted) return;
      // Still mounted ⇒ the confirm did NOT advance (a silent controller
      // no-op — e.g. its delete/rescan latch). Release the view lock, or
      // every action on this screen wedges disabled (security NIT).
      setState(() => _saving = false);
    } catch (_) {
      // confirmBackup reverted to AwaitingBackup (gate stayed CLOSED) and
      // rethrew. This same view is still mounted; surface the honest cue and
      // re-enable the action. Caught by type is unnecessary — every failure
      // maps to the same recoverable message (no raw code; invariant 6).
      if (!mounted) return;
      setState(() => _saving = false);
      messenger.showSnackBar(
        SnackBar(content: Text(l10n.walletBackupSaveFailed)),
      );
    }
  }

  /// The #356-F2 escape out of forced backup: a user who mis-tapped Create (or
  /// changed their mind) must never be cornered into ticking "I backed it up"
  /// FALSELY just to leave — the money-UX inversion the device e2e
  /// flagged (a coerced false confirmation defeats the gate it satisfies).
  /// Deleting here forfeits no funds: the wallet has never been deposit-ready
  /// (the gate opens only past this screen, so the app has never shown it a
  /// receive address), and the rare restored wallet stranded here by a
  /// confirm-persist failure lives on-chain — its phrase restores it again.
  /// `deleteWallet` is already legal from [OnboardingAwaitingBackup]; on
  /// success the controller lands on [OnboardingWelcome] and this view is
  /// replaced.
  Future<void> _startOver() async {
    final l10n = WalletLocalizations.of(context);
    final messenger = ScaffoldMessenger.of(context);
    // Deliberate, confirm-guarded (a destructive action off a money screen) —
    // the cancel action is the SAFE default and says what it keeps.
    // Must SCROLL, not clip, at large text scales (the reclaim-dialog rule;
    // UX HIGH): at 2.0×/320dp the three-sentence fund-loss warning
    // exceeds the viewport — the one dialog whose body MUST be readable. The
    // helper's Material dialog always scrolls.
    final confirmed = await showWalletConfirm(
      context,
      title: l10n.walletBackupStartOverConfirmTitle,
      body: l10n.walletBackupStartOverConfirmBody,
      cancelLabel: l10n.walletBackupStartOverKeep,
      confirmLabel: l10n.walletBackupStartOverConfirm,
      kind: WalletConfirmKind.destructive,
    );
    if (!confirmed || !mounted) return;
    setState(() => _deleting = true);
    final outcome = await ref
        .read(onboardingControllerProvider.notifier)
        .deleteWallet();
    // shredded → Welcome replaced this view; failedClosed → OnboardingFailed
    // replaced it (its own retry path) — both unmount us, so nothing below
    // runs. notDeletable is a raced double-tap no-op.
    if (!mounted) return;
    setState(() => _deleting = false);
    if (outcome == WalletDeletionOutcome.failedRecovered) {
      // The shred faulted and the keychain-first wipe deleted NOTHING — still
      // on this screen with the wallet intact; tell the truth, never a false
      // "deleted" (the Security screen's twin outcome copy).
      messenger.showSnackBar(
        SnackBar(content: Text(l10n.securityDeleteFailedSnack)),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);

    // The #356-F2 escape is offered ONLY on a wallet with no observed prior
    // life (security HIGH — see [OnboardingAwaitingBackup.hasPriorActivity]):
    // a lost confirmed-flag can land a FUNDED, balance-less-rendered wallet on
    // this screen, where a fresh-create-flavored delete would be a fund-loss
    // trap. Suppressed ⇒ the screen degrades to the pre-F2 posture (re-confirm
    // the backup) — annoying, never dangerous. Any non-backup state (a
    // transition frame) also hides it.
    final showStartOver = switch (ref.watch(onboardingControllerProvider)) {
      OnboardingAwaitingBackup(:final hasPriorActivity) => !hasPriorActivity,
      OnboardingConfirming(:final hasPriorActivity) => !hasPriorActivity,
      _ => false,
    };

    // AUTO-HIDE ON BACKGROUND (the cross-platform shoulder-surf / recents-
    // snapshot mitigation). FLAG_SECURE is Android-ONLY, so on iOS and desktop
    // the OS app-switcher/window snapshot taken when the app is BACKGROUNDED
    // would otherwise capture the on-screen seed — permanently, no matter what
    // iii-B-2-b wires. We collapse back to the pre-reveal state on the actual
    // BACKGROUND moment — `hidden` (the OS takes the app-switcher snapshot;
    // also desktop minimize) and `paused` (full Android background) — which
    // unwatches the autoDispose mnemonicRevealProvider and drops the cached
    // copy (NO explicit invalidate — invalidating while still watched would
    // re-run revealMnemonic and re-fetch the seed, the opposite of the intent).
    //
    // NOT on `inactive`: that is transient FOREGROUND focus-noise (notification
    // shade, call banner, Control Center, desktop alt-tab) that takes no
    // snapshot. Hiding there would clear the words on every interruption — the
    // perverse incentive to screenshot — and make desktop multitasking
    // unusable; it also contradicts the flutter-patterns lifecycle rule the
    // sync stream already follows (`inactive`/`hidden` are focus noise, act on
    // the real background). `&& !_saving` so a background mid-persist never
    // tears down an in-flight confirm (the brief shared-prefs write completes
    // first; FLAG_SECURE covers the Android snapshot regardless). The instant
    // iOS cover-on-resign-active + the desktop-alt-tab polish ride the native
    // ScreenSecurity adapter (iii-B-2-b). `ref.listen` in build is the
    // sanctioned spot (mirrors syncStatusProvider's lifecycle listen).
    ref.listen<AppLifecycleState>(appLifecycleProvider, (_, next) {
      final backgrounding =
          next == AppLifecycleState.hidden || next == AppLifecycleState.paused;
      if (backgrounding && _revealed && !_saving) {
        setState(() {
          _revealed = false;
          _confirmed = false;
        });
      }
    });

    return SingleChildScrollView(
      padding: onboardingPagePadding,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l10n.walletBackupTitle, style: textTheme.headlineSmall),
          const SizedBox(height: 12),
          // Stays ON SCREEN, unlike the other steps' bodies: it is the
          // money-safety warning the confirmation below attests to (whoever
          // holds the words holds the funds), a cost, not an explanation.
          Text(l10n.walletBackupBody, style: textTheme.bodyMedium),
          const SizedBox(height: 16),
          RecoveryPhraseSecureNote(blocksScreenshots: _screenshotsBlocked),
          const SizedBox(height: 24),
          if (!_revealed)
            WalletCta(
              child: FilledButton.icon(
                onPressed: _deleting
                    ? null
                    : () => setState(() => _revealed = true),
                icon: const WalletIcon(WalletGlyph.reveal),
                label: Text(l10n.walletBackupReveal),
              ),
            )
          else
            _buildRevealed(context, l10n, textTheme, colors),
          if (showStartOver) ...[
            const SizedBox(height: 24),
            // The forced-backup escape (#356-F2) — understated (muted, below
            // the primary flow): the primary path is still "back it up"; this
            // exists so the only exit is never a FALSE "I backed it up".
            Center(
              child: TextButton.icon(
                // Colour only: the theme owns the size and shape.
                style: TextButton.styleFrom(foregroundColor: colors.textMuted),
                onPressed: (_saving || _deleting) ? null : _startOver,
                icon: _deleting
                    ? const SizedBox(
                        height: 16,
                        width: 16,
                        child: CircularProgressIndicator.adaptive(
                          strokeWidth: 2,
                        ),
                      )
                    : const WalletIcon(WalletGlyph.recover, size: 18),
                label: Text(l10n.walletBackupStartOver),
              ),
            ),
          ],
        ],
      ),
    );
  }

  /// The post-reveal body — watches the autoDispose reveal future. Inlined (not
  /// a child widget) so it can read/setState the reveal/confirm/saving flags.
  Widget _buildRevealed(
    BuildContext context,
    WalletLocalizations l10n,
    TextTheme textTheme,
    WalletColors colors,
  ) {
    final words = ref.watch(mnemonicRevealProvider);
    return words.when(
      loading: () => Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: WalletOnboardingBusyView(label: l10n.walletBackupRevealing),
      ),
      // No error PAYLOAD is read (it could carry sensitive context) — a single
      // honest, plain-language message + a retry that re-runs the reveal.
      error: (_, _) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            l10n.walletBackupRevealFailed,
            style: textTheme.bodyMedium?.copyWith(color: colors.orange),
          ),
          const SizedBox(height: 16),
          OutlinedButton.icon(
            // Locked with the rest of the screen (reliability MED): a
            // retry here re-runs revealMnemonic — a seed re-unseal that must
            // never race an in-flight Start-over shred (or the confirm
            // persist). The [_deleting] doc's "locks every action" is now true.
            onPressed: (_saving || _deleting)
                ? null
                : () => ref.invalidate(mnemonicRevealProvider),
            icon: const WalletIcon(WalletGlyph.retry),
            label: Text(l10n.walletBackupRetryReveal),
          ),
        ],
      ),
      data: (revealed) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          RecoveryWordGrid(words: revealed.words),
          const SizedBox(height: 24),
          // The deliberate money-safety confirmation. ≥44px row by default.
          CheckboxListTile(
            value: _confirmed,
            // Locked while saving (the gate decision can't change mid-persist)
            // and while a Start-over delete runs.
            onChanged: (_saving || _deleting)
                ? null
                : (v) => setState(() => _confirmed = v ?? false),
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            title: Text(
              l10n.walletBackupConfirmCheckbox,
              style: textTheme.bodyMedium,
            ),
          ),
          const SizedBox(height: 16),
          WalletCta(
            child: FilledButton(
              // Enabled ONLY once the box is checked and not mid-save/mid-
              // delete. The controller also guards (a confirm off
              // AwaitingBackup only), so a stray double-tap is a no-op at
              // both layers.
              onPressed: (_confirmed && !_saving && !_deleting)
                  ? _confirm
                  : null,
              child: _saving
                  ? const SizedBox(
                      height: 20,
                      width: 20,
                      child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                    )
                  : Text(l10n.walletBackupContinue),
            ),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Restore (the inbound recovery-phrase entry — the second sanctioned crossing)
// ---------------------------------------------------------------------------

/// The BIP39 word counts a valid recovery phrase can have. The Restore action
/// is gated on one of these (a checksum/wordlist error past the length is the
/// SDK's job and surfaces as an inline fault) — pinned by a widget test (gate 7).
const Set<int> kValidMnemonicLengths = {12, 15, 18, 21, 24};

/// [OnboardingRestoreInput] / [OnboardingRestoring]: enter an existing recovery
/// phrase to restore a wallet. Rendered for BOTH states (like [WalletBackupView]
/// across confirm) so the typed phrase in [_phraseController] survives the
/// restore round-trip — a fixable fault (a mistyped word) returns here with the
/// phrase intact, never a blank field.
///
/// KEY RESIDUE (spec §10, the inbound twin of the backup screen): the phrase the
/// user types lives in [_phraseController] (Dart memory, which cannot be
/// zeroized) only while this screen is mounted; it is dropped when the screen
/// unmounts (restore succeeds → Active, or Back → Welcome). It is the documented,
/// minimised inbound exposure. The field is the user's own input, so — unlike the
/// backup screen — it is deliberately editable/paste-friendly; the
/// auto-hide-on-background that the display screen uses would destroy in-progress
/// input, so the protection here is FLAG_SECURE (Android) + the honest secure
/// note + the manager-carried native cover-on-resign-active (iOS/desktop).
class WalletRestoreView extends ConsumerStatefulWidget {
  const WalletRestoreView({super.key});

  @override
  ConsumerState<WalletRestoreView> createState() => _WalletRestoreViewState();
}

class _WalletRestoreViewState extends ConsumerState<WalletRestoreView> {
  /// The committed recovery words, mirrored from [MnemonicPillField] via its
  /// `onChanged`. The pill field owns the authoritative entry state (so the words
  /// survive the RestoreInput→Restoring→RestoreInput round-trip with the view);
  /// this is the copy the count / submit read.
  List<String> _words = const [];

  /// The wallet-creation date that floors the restore scan (faster than a full
  /// history scan). DEFAULTS to ~6 months ago (maintainer): a fast first sync for
  /// the common case, instead of years of history. The user can pick an earlier
  /// date (an older wallet) or switch to a full-history scan (`null`) below; the
  /// money-safety of the default lives in the section copy ("older funds won't
  /// appear — pick an earlier date"). `null` = "scan everything" → the SDK
  /// floors to Sapling activation (slower, but never skips older funds).
  DateTime? _creationDate;

  /// Captured in [initState] so [dispose] can release protection WITHOUT
  /// touching `ref` (Riverpod guidance — `ref` is unsafe once deactivated).
  late final ScreenSecurity _security;

  /// True only after the native side ACKNOWLEDGED the block (review B1) —
  /// see the backup view's twin field; same pessimistic-until-ack rule.
  bool _screenshotsBlocked = false;

  @override
  void initState() {
    super.initState();
    // The fast-by-default restore birthday (see [_creationDate]). Computed once
    // here (not in build) so it is stable for the view's life; the day-of-month
    // is irrelevant to a month-granularity estimate, and Dart normalizes a month
    // underflow (e.g. month 0 → the prior December).
    final now = DateTime.now();
    _creationDate = DateTime(now.year, now.month - 6, now.day);
    // Request screenshot/recents protection for the whole time a phrase can be
    // on screen (FLAG_SECURE on Android; an honest no-op + advice elsewhere).
    // ref.read in initState is allowed (no watch).
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

  Future<void> _pickCreationDate(BuildContext context) async {
    final now = DateTime.now();
    final picked = await showDatePicker(
      context: context,
      initialDate: _creationDate ?? DateTime(now.year - 1, now.month),
      // Sapling activation is the earliest USEFUL birthday — the SDK floors
      // anything earlier to it, so allowing a Sprout-era pick would only silently
      // advance the user's choice by ~2 years. Bounding the picker here makes the
      // floor visible instead of surprising. Shared with the rescan picker so the
      // two can never drift (one source of truth — `kZcashSaplingActivationDate`).
      firstDate: kZcashSaplingActivationDate,
      lastDate: now,
      helpText: WalletLocalizations.of(context).walletRestoreBirthdayPick,
    );
    // showDatePicker awaits a dialog — re-check mounted before touching state
    // (the project pattern; suppresses use_build_context_synchronously).
    if (!mounted || picked == null) return;
    setState(() => _creationDate = picked);
  }

  void _submit() {
    // The pill field already lowercased/trimmed each word; the controller
    // re-applies the same idempotent normalizer as the chokepoint guarantee.
    // Fire-and-forget — startRestore drives `state` (→ Restoring → Active /
    // RestoreInput(fault)).
    unawaited(
      ref
          .read(onboardingControllerProvider.notifier)
          .startRestore(_words, approximateCreationTime: _creationDate),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);

    // Submitting iff the controller is mid-restore. The restore screen renders
    // for both RestoreInput and Restoring; in Restoring the inputs lock and the
    // button spins. The fault (if any) rides the RestoreInput state.
    final state = ref.watch(onboardingControllerProvider);
    final submitting = state is OnboardingRestoring;
    final fault = state is OnboardingRestoreInput ? state.fault : null;
    final faultWordIndex = state is OnboardingRestoreInput
        ? state.invalidWordIndex
        : null;

    // The bundled BIP39 list (null while loading) drives live validity +
    // autocomplete in the pill field and the known-bad-word gate here.
    final wordlist = ref.watch(bip39WordlistProvider).asData?.value;
    final wordCount = _words.length;
    final invalidCount = wordlist == null
        ? 0
        : _words.where((w) => !wordlist.isValid(w)).length;
    final lengthOk = kValidMnemonicLengths.contains(wordCount);
    // Gate submit on a valid length AND (once the list loads) no KNOWN-bad word —
    // typos are fixed before submit; the SDK is still the checksum gate (an
    // all-valid-words phrase can still fail checksum → the inline fault).
    final canSubmit = lengthOk && invalidCount == 0;

    return SingleChildScrollView(
      padding: onboardingPagePadding,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          OnboardingTitle(title: l10n.walletRestoreTitle),
          const SizedBox(height: 12),
          // ON screen, not behind an (i): it is the money-safety warning — a
          // 25th-word (passphrase) wallet cannot be restored here and would
          // show as an EMPTY wallet, not an error. Unlike the Welcome and
          // Watch bodies, a user must read it before typing a phrase.
          Text(
            l10n.walletRestoreBody,
            key: const Key('onboarding-restore-body'),
            style: textTheme.bodyMedium,
          ),
          const SizedBox(height: 16),
          RecoveryPhraseSecureNote(blocksScreenshots: _screenshotsBlocked),
          const SizedBox(height: 20),
          // The per-word pill entry: each word becomes a numbered pill, a real
          // BIP39 word reads normal, an unknown word flags in the error colour
          // live, and the autocomplete row suggests as you type. Paste-friendly.
          MnemonicPillField(
            wordlist: wordlist,
            enabled: !submitting,
            onChanged: (words) => setState(() => _words = words),
          ),
          const SizedBox(height: 8),
          // Honest live status: the SDK fault (if any) > the known-bad-word cue >
          // the valid-length hint > the plain count.
          Semantics(
            // liveRegion armed ONLY on an SDK fault: the fault APPEARS after a
            // submit round-trip and must be announced (the watch-only twin's
            // UX-M5 convention), while the always-present word-count/hint text
            // must NOT announce every keystroke (the send screen's
            // conditional-arming precedent).
            liveRegion: fault != null,
            child: Text(
              _statusText(
                l10n,
                fault,
                faultWordIndex,
                wordCount,
                lengthOk,
                invalidCount,
              ),
              style: textTheme.bodySmall?.copyWith(
                color: (fault != null || invalidCount > 0)
                    ? colors.orange
                    : (canSubmit ? colors.textMuted : colors.textDim),
              ),
            ),
          ),
          const SizedBox(height: 24),
          _RestoreBirthdaySection(
            creationDate: _creationDate,
            enabled: !submitting,
            onPick: () => _pickCreationDate(context),
            onClear: () => setState(() => _creationDate = null),
          ),
          const SizedBox(height: 24),
          WalletCta(
            child: FilledButton(
              // Enabled only with a valid-length, all-known-words phrase and
              // not mid-restore; the controller also guards (startRestore only
              // from RestoreInput), so a stray double-tap is a no-op at both
              // layers.
              onPressed: (canSubmit && !submitting) ? _submit : null,
              child: submitting
                  ? const SizedBox(
                      height: 20,
                      width: 20,
                      child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                    )
                  : Text(l10n.walletRestoreSubmit),
            ),
          ),
          const SizedBox(height: 8),
          TextButton(
            // Back to Welcome. Disabled mid-restore so a backgrounded persist is
            // never abandoned half-way.
            onPressed: submitting
                ? null
                : () => ref
                      .read(onboardingControllerProvider.notifier)
                      .cancelRestore(),
            child: Text(l10n.walletRestoreBack),
          ),
        ],
      ),
    );
  }

  /// The single honest status line under the field, by priority: a SDK fault
  /// (e.g. a checksum failure the live wordlist can't see) → a known-bad-word cue
  /// → the valid-length hint → the plain word count.
  static String _statusText(
    WalletLocalizations l10n,
    RestoreInputFault? fault,
    int? faultWordIndex,
    int wordCount,
    bool lengthOk,
    int invalidCount,
  ) {
    if (fault != null) return _faultMessage(l10n, fault, faultWordIndex);
    if (invalidCount > 0) {
      return l10n.walletRestoreSomeWordsInvalid(invalidCount);
    }
    if (!lengthOk && wordCount > 0) {
      return '${l10n.walletRestoreWordCount(wordCount)} · ${l10n.walletRestoreLengthHint}';
    }
    return l10n.walletRestoreWordCount(wordCount);
  }

  /// Fault → honest, actionable message. Total over [RestoreInputFault]; the word
  /// INDEX (never the word — §5.4) is shown only when the SDK pinned one.
  static String _faultMessage(
    WalletLocalizations l10n,
    RestoreInputFault fault,
    int? wordIndex,
  ) {
    return switch (fault) {
      RestoreInputFault.invalidWord =>
        wordIndex != null
            ? l10n.walletRestoreFaultInvalidWord(wordIndex)
            : l10n.walletRestoreFaultInvalidPhrase,
      RestoreInputFault.seedMismatch => l10n.walletRestoreFaultSeedMismatch,
      RestoreInputFault.alreadyExists => l10n.walletRestoreFaultAlreadyExists,
      RestoreInputFault.birthdayTooRecent =>
        l10n.walletRestoreFaultBirthdayTooRecent,
    };
  }
}

/// The optional "speed up the first sync" birthday control. A creation date
/// floors the scan; left unset, the SDK scans the whole chain (slower, but
/// money-safe) — stated honestly so the user makes an informed choice.
class _RestoreBirthdaySection extends StatelessWidget {
  const _RestoreBirthdaySection({
    required this.creationDate,
    required this.enabled,
    required this.onPick,
    required this.onClear,
  });

  final DateTime? creationDate;
  final bool enabled;
  final VoidCallback onPick;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    final date = creationDate;
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: colors.bgCard,
        borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
        border: Border.all(color: colors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l10n.walletRestoreBirthdayTitle,
            style: textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 6),
          // A chosen date EXCLUDES older funds — the (default) risky option, so
          // it carries an honest warning treatment (orange + info icon). The
          // full-scan state (null) is money-safe, so it stays neutral/muted.
          if (date == null)
            Text(
              l10n.walletRestoreBirthdayNone,
              style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
            )
          else
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                WalletIcon(WalletGlyph.info, size: 16, color: colors.orange),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    l10n.walletRestoreBirthdayChosen(
                      MaterialLocalizations.of(context).formatMonthYear(date),
                    ),
                    style: textTheme.bodySmall?.copyWith(color: colors.orange),
                  ),
                ),
              ],
            ),
          const SizedBox(height: 12),
          // A Wrap, not a Row: when the two buttons don't fit side by side the
          // second moves to its own line with its label whole. The fix
          // (a 6.4px overflow on a narrow phone) shared the row with
          // ellipsized labels instead, which read "Chan…" and "Scan all hi…"
          // on a 415dp phone at 1.2x text (walk, TANK MINI).
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              OutlinedButton.icon(
                onPressed: enabled ? onPick : null,
                icon: const WalletIcon(WalletGlyph.pickDate, size: 18),
                label: Text(
                  date == null
                      ? l10n.walletRestoreBirthdayPick
                      : l10n.walletRestoreBirthdayChange,
                ),
              ),
              if (date != null)
                TextButton(
                  onPressed: enabled ? onClear : null,
                  child: Text(l10n.walletRestoreBirthdayClear),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

/// [OnboardingWatchOnlyInput] / [OnboardingCreatingWatchOnly] (#397 §3.7 D5):
/// paste — or, on a camera platform, SCAN (#397 P4, the shared reader brick
/// wearing [ScanPurpose.viewingKey]; completes the in-app export→scan→import
/// loop against the export screen's QR tile) — a unified full viewing key +
/// pick the wallet's creation date to import a WATCH-ONLY wallet. Paste stays
/// the canonical path and the ONLY path on desktop/web (the swap scanner's
/// platform gate, reused). Rendered for BOTH states (like the restore view)
/// so the pasted key + picked date survive the create round-trip — a fixable
/// fault (a wrong-network key) returns here with the field intact.
///
/// A UFVK cannot spend or reach the seed, but it shows every past and future
/// payment: the field takes no key-residue ceremony beyond the smallest-scope
/// [TextEditingController] lifetime, and DOES take the screenshot block. The birthday is
/// REQUIRED here (a watch-only import has no lazy seed-path — the account is
/// imported eagerly), so it defaults to ~6 months ago and is always set, with
/// no "scan everything" arm.
class WalletWatchOnlyView extends ConsumerStatefulWidget {
  const WalletWatchOnlyView({super.key});

  @override
  ConsumerState<WalletWatchOnlyView> createState() =>
      _WalletWatchOnlyViewState();
}

class _WalletWatchOnlyViewState extends ConsumerState<WalletWatchOnlyView> {
  final TextEditingController _keyController = TextEditingController();

  /// REQUIRED creation date (a watch-only import needs an eager birthday floor);
  /// defaults to ~6 months ago and is always non-null — the user can pick an
  /// earlier date but never clear it.
  late DateTime _creationDate;

  /// A viewing key reveals the whole history: the same screenshot / recents
  /// protection the restore view takes while it can be on screen. Captured
  /// here so [dispose] releases it without touching `ref`.
  late final ScreenSecurity _security;

  @override
  void initState() {
    super.initState();
    final now = DateTime.now();
    _creationDate = DateTime(now.year, now.month - 6, now.day);
    // Re-render the submit gate live as the key field changes.
    _keyController.addListener(_onChanged);
    _security = ref.read(screenSecurityProvider);
    unawaited(_security.enable());
  }

  @override
  void dispose() {
    unawaited(_security.disable());
    _keyController.removeListener(_onChanged);
    _keyController.dispose();
    super.dispose();
  }

  void _onChanged() => setState(() {});

  Future<void> _pickCreationDate(BuildContext context) async {
    final now = DateTime.now();
    final picked = await showDatePicker(
      context: context,
      initialDate: _creationDate,
      firstDate: kZcashSaplingActivationDate,
      lastDate: now,
      helpText: WalletLocalizations.of(context).walletWatchOnlyBirthdayPick,
    );
    if (!mounted || picked == null) return;
    setState(() => _creationDate = picked);
  }

  void _submit() {
    // Fire-and-forget — startWatchOnly drives `state` (→ CreatingWatchOnly →
    // Active / WatchOnlyInput(fault)).
    unawaited(
      ref
          .read(onboardingControllerProvider.notifier)
          .startWatchOnly(_keyController.text, creationDate: _creationDate),
    );
  }

  /// Scan a viewing-key QR into the field via the shared reader brick (#397
  /// P4). ADDITIVE — the field stays fully paste/type-able; this only fills
  /// it. The payload is §5.4-sensitive (NEVER logged); the SDK validates it
  /// Rust-side (a wrong QR reads as the inline invalid-key fault).
  Future<void> _scanKey() async {
    final scan = ref.read(viewingKeyScannerProvider);
    final raw = await scan(context);
    if (!mounted || raw == null) return;
    // Fill the field with the raw scan; normalization (whitespace strip +
    // all-caps case-fold) happens at the startWatchOnly chokepoint (F1) so a
    // PASTE gets the identical treatment — the fold used to live here, which
    // rescued an uppercase scan but not an uppercase paste of the same key.
    setState(() => _keyController.text = raw.trim());
    // A11y (UX-M5 announce-on-change): a sighted user watches the field
    // populate, but a screen-reader user gets NO cue the scan landed (the field
    // is not a live region). Announce that a key was captured — NEVER the key
    // itself (§5.4). DEFERRED to the next frame (UX MINOR): firing
    // synchronously here collides with the scanner route POPPING and focus
    // returning to this screen, and a one-shot announcement made mid-transition
    // is dropped on some platforms (iOS VoiceOver); post-frame it lands after
    // focus settles. The view/direction/string are captured NOW (all valid —
    // `mounted` checked above, no await since), so the callback touches no
    // possibly-stale context.
    final view = View.of(context);
    final direction = Directionality.of(context);
    final message = WalletLocalizations.of(context).walletWatchOnlyScanFilled;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      SemanticsService.sendAnnouncement(view, message, direction);
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);

    final state = ref.watch(onboardingControllerProvider);
    final submitting = state is OnboardingCreatingWatchOnly;
    final fault = state is OnboardingWatchOnlyInput ? state.fault : null;
    // A well-formed unified viewing key is a long `uview…` string; gate submit
    // on a non-empty field (the SDK is the real validator — a malformed key
    // returns the inline fault). Trim so a stray paste-whitespace doesn't pass.
    final canSubmit = _keyController.text.trim().isNotEmpty;

    return SingleChildScrollView(
      padding: onboardingPagePadding,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          OnboardingTitle(
            title: l10n.walletWatchOnlyTitle,
            explanation: l10n.walletWatchOnlyBody,
          ),
          const SizedBox(height: 16),
          TextField(
            key: const ValueKey('watch-only-key-field'),
            controller: _keyController,
            enabled: !submitting,
            minLines: 2,
            maxLines: 4,
            autocorrect: false,
            enableSuggestions: false,
            // A viewing key reveals the whole history: keep it out of the
            // keyboard's learned words.
            enableIMEPersonalizedLearning: false,
            // `null`, not the default empty list: any list opts the field into
            // the platform autofill service, which may offer to save the key.
            autofillHints: null,
            smartDashesType: SmartDashesType.disabled,
            smartQuotesType: SmartQuotesType.disabled,
            decoration: InputDecoration(
              labelText: l10n.walletWatchOnlyKeyLabel,
              hintText: l10n.walletWatchOnlyKeyHint,
              // The live-scan affordance — camera platforms only (the swap
              // scanner's gate, reused); paste stays the canonical path.
              suffixIcon: ref.watch(addressScannerSupportedProvider)
                  ? IconButton(
                      key: const ValueKey('watch-only-scan'),
                      icon: const WalletIcon(WalletGlyph.scanQr),
                      tooltip: l10n.walletWatchOnlyScanTooltip,
                      onPressed: submitting ? null : _scanKey,
                    )
                  : null,
            ),
          ),
          // Discoverability (UX): the body copy is paste-first, and the scan
          // affordance is only a suffix icon (its meaning hidden behind a
          // tooltip). On a camera platform, name it in a muted hint so the
          // in-app export→scan→import loop is findable without hunting the icon.
          // Gated on the SAME provider as the suffix icon — never shown on
          // desktop/web where paste is the only path.
          if (ref.watch(addressScannerSupportedProvider)) ...[
            const SizedBox(height: 6),
            Text(
              l10n.walletWatchOnlyScanHint,
              style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
            ),
          ],
          const SizedBox(height: 8),
          if (fault != null)
            // liveRegion: the fault APPEARS after a submit round-trip (a state
            // change, not screen-entry content), so a screen reader must hear
            // it without re-traversing the form — the announce-on-change
            // convention the wallet surface uses for state cues (UX-M5).
            Semantics(
              liveRegion: true,
              child: Text(
                _faultMessage(l10n, fault),
                key: const ValueKey('watch-only-fault'),
                style: textTheme.bodySmall?.copyWith(color: colors.orange),
              ),
            ),
          const SizedBox(height: 24),
          // The REQUIRED creation-date control — a watch-only import always has
          // a birthday floor (no full-scan arm).
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: colors.bgCard,
              borderRadius: BorderRadius.circular(
                WalletShapes.of(context).group,
              ),
              border: Border.all(color: colors.border),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  l10n.walletWatchOnlyBirthdayTitle,
                  style: textTheme.bodyMedium?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 6),
                // A watch-only import ALWAYS scans from a chosen floor (no
                // full-scan arm), so it always carries the honest orange warning
                // that funds received before the date won't appear — the exact
                // treatment the restore screen gives its chosen-date branch
                // (UX-H1: the auditor importing an OLDER wallet must not silently
                // see an understated balance).
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    WalletIcon(
                      WalletGlyph.info,
                      size: 16,
                      color: colors.orange,
                    ),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        l10n.walletWatchOnlyBirthdayChosen(
                          MaterialLocalizations.of(
                            context,
                          ).formatMonthYear(_creationDate),
                        ),
                        style: textTheme.bodySmall?.copyWith(
                          color: colors.orange,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  onPressed: submitting
                      ? null
                      : () => _pickCreationDate(context),
                  icon: const WalletIcon(WalletGlyph.pickDate, size: 18),
                  label: Text(l10n.walletWatchOnlyBirthdayChange),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          WalletCta(
            child: FilledButton(
              key: const ValueKey('watch-only-submit'),
              onPressed: (canSubmit && !submitting) ? _submit : null,
              child: submitting
                  ? const SizedBox(
                      height: 20,
                      width: 20,
                      child: CircularProgressIndicator.adaptive(strokeWidth: 2),
                    )
                  : Text(l10n.walletWatchOnlySubmit),
            ),
          ),
          const SizedBox(height: 8),
          TextButton(
            onPressed: submitting
                ? null
                : () => ref
                      .read(onboardingControllerProvider.notifier)
                      .cancelWatchOnly(),
            child: Text(l10n.walletWatchOnlyBack),
          ),
        ],
      ),
    );
  }

  /// Fault → honest, actionable message. Total over [WatchOnlyInputFault].
  static String _faultMessage(
    WalletLocalizations l10n,
    WatchOnlyInputFault fault,
  ) {
    return switch (fault) {
      WatchOnlyInputFault.invalidViewingKey =>
        l10n.walletWatchOnlyFaultInvalidKey,
      WatchOnlyInputFault.networkMismatch =>
        l10n.walletWatchOnlyFaultNetworkMismatch,
      WatchOnlyInputFault.alreadyExists =>
        l10n.walletWatchOnlyFaultAlreadyExists,
      WatchOnlyInputFault.birthdayTooRecent =>
        l10n.walletWatchOnlyFaultBirthdayTooRecent,
    };
  }
}
