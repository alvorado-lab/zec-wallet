import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/lifecycle/app_lifecycle_provider.dart';
import '../../core/router/wallet_navigation.dart';
import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../wallet/onboarding/onboarding_providers.dart';
import '../wallet/onboarding/ufvk_export.dart';
import '../wallet/wallet_providers.dart'
    show isWatchOnlyProvider, walletSessionProvider;
import '../wallet/reveal_authorization.dart';
import '../wallet/reveal_grant.dart';
import '../../shared/address_text.dart';
import '../../shared/qr_tile.dart';
import '../../shared/wallet_notice.dart';
import 'managed_by_host.dart';

/// Settings → Security → **Export viewing key** (#397 §3.7 D5 / ADR-0538): the
/// ONE sanctioned UFVK egress into the UI. Clones the backup screen's gating —
/// the host RE-AUTH gate ([WalletRevealAuthorizer]), screenshot/recents
/// protection, the shoulder-surf reveal latch with auto-hide on background, and
/// the autoDispose read — because the CONSEQUENCE being confirmed (total
/// history surveillance) deserves the same deliberate bar as the seed, even
/// though a UFVK cannot spend.
///
/// DIVERGENCE from the backup screen: a UFVK is PUBLIC once shared, so the
/// artifact IS copyable + QR-able (a seed never is). The D9 warning copy states,
/// in plain words, exactly what sharing this key does — everything received AND
/// sent, past and future, becomes visible; it cannot spend or reach the seed;
/// share only with someone trusted; the only un-share is moving funds to a new
/// wallet.
///
/// SOURCE (G1, #361 companion): the provisioner when the package owns
/// provisioning, ELSE the `WalletSession` port — so a session-only host reaches
/// a WORKING export here instead of the pre-G1 dead end. Only a mount with
/// NEITHER seam renders the honest managed-by-host terminal state. The D9
/// envelope below (re-auth gate, un-share warning, screen security, autoDispose)
/// is identical on both paths — but the package's DEFAULT reveal authorizer is
/// pass-through, so a host mounting this route owes an override of
/// `walletRevealAuthorizerProvider` for the gate to mean anything.
class ExportViewingKeyScreen extends ConsumerStatefulWidget {
  const ExportViewingKeyScreen({super.key});

  @override
  ConsumerState<ExportViewingKeyScreen> createState() =>
      _ExportViewingKeyScreenState();
}

class _ExportViewingKeyScreenState
    extends ConsumerState<ExportViewingKeyScreen> {
  /// Shoulder-surf mitigation: the key stays hidden until a deliberate reveal
  /// tap that PASSES re-auth — and then shows only while the [RevealGrant] that
  /// re-auth minted is still current (the same wallet session instance, the
  /// same operation generation; S2 §3.4, R06). Dropped on background (the
  /// auto-hide) and on any identity change under the mounted screen.
  RevealGrant? _grant;

  /// Re-auth is in flight — the reveal button shows a spinner and is
  /// single-flighted so a double-tap can't stack two host prompts.
  bool _authorizing = false;

  /// A non-cancel re-auth fault (the authorizer threw something other than a
  /// user cancel) — an honest retry line above the reveal button.
  bool _reauthFault = false;

  /// Captured in [initState] so [dispose] releases protection without touching
  /// `ref` (unsafe once deactivated).
  late final ScreenSecurity _security;

  /// True only after the native side ACKNOWLEDGED the screenshot block
  /// (pessimistic until the ack).
  bool _screenshotsBlocked = false;

  @override
  void initState() {
    super.initState();
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
    // SYNCHRONOUS single-flight (the backup screen's rationale): two taps in one
    // frame both see the old `onPressed` and would stack two host prompts.
    if (_authorizing) return;
    setState(() {
      _authorizing = true;
      _reauthFault = false;
    });
    try {
      final grant = await authorizeRevealGrant(ref);
      if (!mounted) return;
      // The grant's first check: a wallet identity change while the prompt
      // was up voids the pass — nothing is read, and the screen stays on the
      // (new) wallet's un-revealed state.
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
      // User cancel — silent (the host prompt already communicated it).
      if (!mounted) return;
      setState(() => _authorizing = false);
    } catch (_) {
      // Any other re-auth error is a real fault; no payload read (§5.4).
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

    // AUTO-HIDE ON BACKGROUND: collapse to pre-reveal on the real background
    // moment (`hidden`/`paused`) so the recents snapshot never captures the
    // key, and the autoDispose export provider drops the cached string. NOT on
    // `inactive` (transient focus noise). Identical to the backup screen. The
    // grant goes with it, so a re-reveal after resume re-authorizes.
    ref.listen<AppLifecycleState>(appLifecycleProvider, (_, next) {
      final backgrounding =
          next == AppLifecycleState.hidden || next == AppLifecycleState.paused;
      if (backgrounding && _grant != null) {
        setState(() => _grant = null);
      }
    });

    return Scaffold(
      appBar: AppBar(title: Text(l10n.walletExportViewingKeyTitle)),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: _body(context, l10n),
        ),
      ),
    );
  }

  /// The D9 un-share warning copy, kind-aware (#397 §3.7 D5, UX-M3): the base
  /// copy tells the sharer the only way to un-share is to MOVE THEIR FUNDS to a
  /// new wallet — unfollowable on a WATCH-ONLY wallet (it can't spend), so the
  /// watch-only variant ends with the honest "it cannot be un-shared" instead.
  String _warningText(WalletLocalizations l10n) =>
      ref.watch(isWatchOnlyProvider)
      ? l10n.walletExportViewingKeyWarningWatchOnly
      : l10n.walletExportViewingKeyWarning;

  Widget _body(BuildContext context, WalletLocalizations l10n) {
    // G1 (#361 companion): the export follows `ufvkExportProvider`'s source order
    // — the provisioner, ELSE the session port. A session-only host now reaches a
    // WORKING export (the key is the wallet's, the D9 gate below is the package's)
    // instead of the pre-G1 dead end, which was a package-rendered watch-only
    // chrome pointing at a capability the package refused to serve. Only a mount
    // with NEITHER seam is genuinely managed-elsewhere.
    final session = ref.watch(walletSessionProvider);
    if (ref.watch(walletProvisionerProvider) == null && session == null) {
      return const WalletManagedByHost();
    }

    final textTheme = Theme.of(context).textTheme;

    // The grant's check before every render: WATCHING the session above is
    // what rebuilds this screen when the wallet identity changes under it; a
    // grant minted for another session instance or generation is dropped here,
    // so the export provider it keyed is unwatched (and disposed) instead of
    // re-read against the next wallet.
    final grant = _grant;
    if (grant != null &&
        !revealGrantIsCurrent(ref, grant, currentSession: session)) {
      _grant = null;
    }
    final live = _grant;
    if (live != null) {
      return _revealedBody(context, l10n, textTheme, live);
    }
    return _sealedBody(l10n);
  }

  /// Pre-reveal: the warning, the secure note and the reveal button.
  Widget _sealedBody(WalletLocalizations l10n) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // The D9 warning copy — the full "what sharing this key does" contract,
        // shown BEFORE the reveal so the user reads it before acting.
        _WarningCard(text: _warningText(l10n)),
        const SizedBox(height: 16),
        _SecureNote(blocksScreenshots: _screenshotsBlocked, l10n: l10n),
        const SizedBox(height: 24),
        if (_reauthFault) ...[
          WalletNotice(
            tone: WalletNoticeTone.warning,
            glyph: WalletGlyph.error,
            message: l10n.walletBackupReauthFailed,
            liveRegion: true,
          ),
          const SizedBox(height: 16),
        ],
        FilledButton.icon(
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
                ? l10n.walletExportViewingKeyRetry
                : l10n.walletExportViewingKeyReveal,
          ),
        ),
      ],
    );
  }

  /// Post-reveal: watch the autoDispose export future keyed by [grant]. Reached
  /// only after re-auth passed and only while [grant] is current.
  Widget _revealedBody(
    BuildContext context,
    WalletLocalizations l10n,
    TextTheme textTheme,
    RevealGrant grant,
  ) {
    final ufvk = ref.watch(ufvkExportProvider(grant));
    return ufvk.when(
      loading: () => Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Column(
          children: [
            const CircularProgressIndicator.adaptive(),
            const SizedBox(height: 16),
            Text(
              l10n.walletExportViewingKeyRevealing,
              style: textTheme.bodyMedium,
            ),
          ],
        ),
      ),
      error: (error, _) {
        // The grant went void between the render check and the read: nothing
        // was exported. Back to the un-revealed state; a reveal re-authorizes.
        if (error is RevealGrantVoid) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (mounted && identical(_grant, grant)) {
              setState(() => _grant = null);
            }
          });
          return _sealedBody(l10n);
        }
        // A watch-only store mid-provision (the crash window) surfaces
        // ProvisioningIncomplete; anything else is a transient store read fault.
        // Both are honest one-tap retries that re-run the export.
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _WarningCard(text: _warningText(l10n)),
            const SizedBox(height: 24),
            WalletNotice(
              tone: WalletNoticeTone.warning,
              glyph: WalletGlyph.error,
              message: l10n.walletExportViewingKeyFailed,
              liveRegion: true,
            ),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: () => ref.invalidate(ufvkExportProvider(grant)),
              icon: const WalletIcon(WalletGlyph.retry),
              label: Text(l10n.walletExportViewingKeyRetry),
            ),
          ],
        );
      },
      data: (redacted) {
        // The one place the key leaves its redacting box: shown, copied and
        // encoded as a QR code below, never logged.
        final viewingKey = redacted.key;
        // Defensive: a healthy wallet always returns a non-empty artifact.
        if (viewingKey.isEmpty) return const WalletManagedByHost();
        // ONE binding for the copy: the button's `onPressed` and the semantics
        // action below both read it, so a screen reader's double-tap and a
        // finger run the same code.
        Future<void> copy() async {
          await Clipboard.setData(ClipboardData(text: viewingKey));
          unawaited(HapticFeedback.lightImpact());
          if (!context.mounted) return;
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(l10n.walletExportViewingKeyCopied)),
          );
        }

        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _WarningCard(text: _warningText(l10n)),
            const SizedBox(height: 24),
            // The user exports a UFVK to share it, so QR + copy are correct
            // here (the divergence from the seed grid). The QR encodes the key VERBATIM
            // (lowercase), IDENTICAL to the displayed + copied text.
            //
            // (tried uppercasing the payload for BIP173 alphanumeric-mode
            // density — REVERTED the pinned `qr_flutter`/`qr` encoder
            // always uses 8-bit BYTE mode via QrImageView's `data:` path (it
            // never auto-selects alphanumeric — that needs an explicit
            // addAlphaNumeric this widget doesn't call), so uppercasing yields an
            // IDENTICAL-size byte-mode code — zero density gain — while shipping a
            // non-canonical uppercase bech32 a strict lowercase-only third-party
            // scanner could reject. All cost, no benefit. Real densification, if
            // ever wanted, is a separate QrTile alphanumeric mode. Our import
            // still folds case at the F1 chokepoint, so a foreign uppercase QR
            // imports regardless.)
            Center(
              child: QrTile(
                payload: viewingKey,
                label: l10n.walletExportViewingKeyQrLabel,
                tileKey: const Key('export-ufvk-qr'),
              ),
            ),
            const SizedBox(height: 16),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: AddressText(viewingKey),
              ),
            ),
            const SizedBox(height: 16),
            Semantics(
              container: true,
              button: true,
              excludeSemantics: true,
              label: l10n.walletExportViewingKeyCopy,
              // `excludeSemantics` drops the child subtree and the button's own
              // tap action with it, so the action must live on THIS node — a
              // screen reader otherwise hears a button whose double-tap has
              // nothing to invoke (Relim `0c5ae1bd`, #661/#721).
              onTap: () => unawaited(copy()),
              child: FilledButton.icon(
                icon: const WalletIcon(WalletGlyph.copy),
                label: Text(l10n.walletExportViewingKeyCopy),
                onPressed: () => unawaited(copy()),
              ),
            ),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: () => context.leaveToWalletRoot(),
              child: Text(l10n.walletExportViewingKeyDone),
            ),
          ],
        );
      },
    );
  }
}

/// The D9 warning — a text-carried caution (not colour-only): the honest
/// "what this key reveals" contract, as a notice line in the warning tone (the
/// treatment the transparent-receive warning uses).
class _WarningCard extends StatelessWidget {
  const _WarningCard({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    // NO liveRegion: the warning is screen-ENTRY content (read in normal
    // traversal order, first by position), not a state change — a live region
    // here re-announces the full D9 text over the reveal latch / screen-
    // security rebuilds, drowning the change that actually happened (UX-L2).
    return WalletNotice(
      key: const Key('export-ufvk-warning'),
      tone: WalletNoticeTone.warning,
      glyph: WalletGlyph.reveal,
      message: text,
    );
  }
}

/// The ACK-driven screenshot-protection note (mirrors RecoveryPhraseSecureNote):
/// says whether screenshots are actually blocked, never over-claiming.
class _SecureNote extends StatelessWidget {
  const _SecureNote({required this.blocksScreenshots, required this.l10n});

  final bool blocksScreenshots;
  final WalletLocalizations l10n;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        WalletIcon(
          blocksScreenshots ? WalletGlyph.locked : WalletGlyph.info,
          size: 18,
          color: colors.textMuted,
        ),
        const SizedBox(width: 8),
        Expanded(
          child: Text(
            blocksScreenshots
                ? l10n.walletExportViewingKeySecureNoteAndroid
                : l10n.walletExportViewingKeySecureNoteOther,
            style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
          ),
        ),
      ],
    );
  }
}
