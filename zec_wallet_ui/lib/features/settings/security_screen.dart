import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../core/router/wallet_routes.dart';
import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../wallet/onboarding/onboarding_controller.dart';
import '../wallet/onboarding/onboarding_providers.dart';
import '../wallet/onboarding/onboarding_state.dart';
import '../wallet/wallet_providers.dart';
import '../../shared/settings_section_header.dart';
import '../../shared/wallet_dialog.dart';
import '../../shared/wallet_group.dart';
import '../../shared/wallet_info_button.dart';

/// Settings → Security: the wallet's key-custody honesty (FR-14 H1) + the
/// destructive "Delete wallet". The custody card is the PRODUCTION consumer of
/// `WalletHandle.custodyDisclosure` — it tells the user, BEFORE they delete,
/// whether deletion removes a key held by secure hardware or is best-effort, so
/// the confirmation is honest (operating principle 6). No tier is permanent
/// erasure (ADR-0571).
///
/// Reachable only from a provisioned wallet (the wallet overflow menu). On a
/// successful shred the onboarding controller resets to Welcome and we return to
/// `/wallet`, which re-renders the onboarding surface — no stale session survives.
class SecurityScreen extends ConsumerStatefulWidget {
  const SecurityScreen({super.key});

  @override
  ConsumerState<SecurityScreen> createState() => _SecurityScreenState();
}

class _SecurityScreenState extends ConsumerState<SecurityScreen> {
  /// True while a delete is in flight — disables the button (the controller is
  /// also single-flighted, but this is the honest UI feedback).
  bool _deleting = false;

  /// True while the backup screen it pushed is open — a SYNCHRONOUS re-entrancy
  /// guard against a same-frame double-tap pushing TWO identical BackupScreens.
  /// Now a pure UX guard: the screenshot-protection race it once also covered
  /// (a stacked twin popping and dropping FLAG_SECURE under the phrase still on
  /// screen) is handled durably by the refcounted `ScreenSecurity` port
  /// (#344 — `RefCountedScreenSecurity` keeps the flag up while any secure
  /// screen holds it). Kept because stacking two identical backup screens is
  /// still a pointless UX wart.
  bool _openingBackup = false;

  /// Single-flight guard for the export-viewing-key tile (mirrors
  /// [_openingBackup]) — a double-tap must not stack two export screens.
  bool _openingExport = false;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);

    // Re-gate on live state (the wallet_router contract: the gate lives with
    // the money surface, never the navigation). Both of this screen's actions
    // — the custody probe and the delete-wallet crypto-shred — run through the
    // package provisioner; a session-only host (own provisioning, overridden
    // `walletSessionProvider`) leaves that seam null and owns custody itself.
    // The overflow-menu entry already hides there; this covers a host-mounted
    // deep link honestly instead of showing a delete button that no-ops.
    if (ref.watch(walletProvisionerProvider) == null) {
      return Scaffold(
        appBar: AppBar(title: Text(l10n.securityTitle)),
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: Text(
              l10n.securityUnavailableBody,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: WalletColors.of(context).textMuted,
              ),
            ),
          ),
        ),
      );
    }

    final disclosure = ref.watch(walletCustodyDisclosureProvider);
    // #397 §3.7 D5: a watch-only wallet holds no seed, so the delete copy must
    // NOT promise recovery "from your recovery phrase" (there is none) — it
    // re-imports from its viewing key instead. `isWatchOnly` (not the custody
    // disclosure) is the authoritative kind source; the custody tier still
    // decides the erase line appended in the dialog.
    final watchOnly = ref.watch(isWatchOnlyProvider);

    return PopScope(
      // Back-lock while the shred is in flight (the rescan sheet's
      // posture, applied to the destructive twin): leaving mid-delete would
      // strand the outcome handling — the wallet-surface reset on success and
      // the honest "couldn't delete" on a recovered fault both assume THIS
      // screen is still mounted to route them. Covers the system back gesture
      // AND the AppBar back button (both run through maybePop). The delete
      // is bounded (close bound + wipe), so the lock always releases.
      canPop: !_deleting,
      child: Scaffold(
        appBar: AppBar(title: Text(l10n.securityTitle)),
        // Groups (S13 Build B; DESIGN §2.5): each section's rows on one
        // surface with inset hairlines, the explanations behind an (i) after
        // their label (DESIGN §6.20), and the destructive row alone in the
        // LAST group, in the error role.
        body: ListView(
          padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
          children: [
            SettingsSectionHeader(l10n.securityCustodySectionTitle),
            WalletGroup(
              key: securityCustodyGroupKey,
              children: [_CustodyRow(disclosure: disclosure)],
            ),
            // #397 §3.7 D5: a WATCH-ONLY wallet has no seed to back up — swap
            // the backup row for the honest no-keys explanation, behind the
            // section label's (i). Both kinds get the "Export viewing key"
            // row (a spending wallet can share a view-only copy too; a
            // watch-only wallet re-exports its own key).
            if (watchOnly)
              Row(
                children: [
                  Expanded(
                    child: SettingsSectionHeader(
                      l10n.walletWatchOnlySectionTitle,
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.only(top: 12),
                    child: WalletInfoButton(
                      label: l10n.walletWatchOnlySectionTitle,
                      body: l10n.walletWatchOnlyAboutBody,
                    ),
                  ),
                ],
              )
            else
              SettingsSectionHeader(l10n.walletBackupSectionTitle),
            WalletGroup(
              key: securityKeysGroupKey,
              dividerIndent: WalletGroup.glyphRowInset,
              children: [
                if (!watchOnly)
                  ListTile(
                    leading: const WalletIcon(WalletGlyph.recoveryPhrase),
                    title: Text(l10n.walletBackupTileTitle),
                    subtitle: Text(l10n.walletBackupTileSubtitle),
                    trailing: const WalletIcon(WalletGlyph.chevronForward),
                    // Pushed (not go) so the AppBar back + the screen's Done
                    // both return here. The backup screen re-gates on the
                    // provisioner, so a session-only host shows its honest
                    // managed-by-host state. The `_openingBackup` guard
                    // prevents a double-tap from stacking two.
                    onTap: () {
                      if (_openingBackup) return;
                      _openingBackup = true;
                      context.push(WalletRoutes.backup).whenComplete(() {
                        if (mounted) _openingBackup = false;
                      });
                    },
                  ),
                ListTile(
                  leading: const WalletIcon(WalletGlyph.reveal),
                  title: Text(l10n.walletExportViewingKeyTileTitle),
                  subtitle: Text(l10n.walletExportViewingKeyTileSubtitle),
                  trailing: const WalletIcon(WalletGlyph.chevronForward),
                  // Same push + single-flight discipline as the backup row;
                  // the export screen re-gates on the provisioner and on
                  // re-auth.
                  onTap: () {
                    if (_openingExport) return;
                    _openingExport = true;
                    context.push(WalletRoutes.exportViewingKey).whenComplete(
                      () {
                        if (mounted) _openingExport = false;
                      },
                    );
                  },
                ),
              ],
            ),
            const SizedBox(height: 32),
            // The destructive row, last and alone: its label is the section's
            // whole title, so it carries no header above it.
            WalletGroup(
              key: securityDeleteGroupKey,
              children: [
                _DeleteRow(
                  deleting: _deleting,
                  explanation: watchOnly
                      ? l10n.securityDeleteWalletSubtitleWatchOnly
                      : l10n.securityDeleteWalletSubtitle,
                  onTap: _confirmDelete,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _confirmDelete() async {
    final l10n = WalletLocalizations.of(context);
    // Restate the per-tier custody at the MOMENT of decision (not only on the
    // card above) — operating principle 6. A hardware tier says its key is held
    // by secure hardware and deleted with the wallet, never that it is
    // unrecoverable (ADR-0571); a best-effort tier reads the residual caution.
    // Falls back to the base warning if the tier probe hasn't resolved.
    final disclosure = ref.read(walletCustodyDisclosureProvider).asData?.value;
    final eraseLine = disclosure == null
        ? null
        : _eraseLine(l10n, disclosure.eraseAssurance);
    // A watch-only wallet has no recovery phrase to warn about — the dialog says
    // it re-imports from its viewing key instead (the tier erase line still
    // appends for BOTH kinds; the DB wrap key is deleted either way).
    final baseBody = ref.read(isWatchOnlyProvider)
        ? l10n.securityDeleteDialogBodyWatchOnly
        : l10n.securityDeleteDialogBody;
    final body = eraseLine == null ? baseBody : '$baseBody\n\n$eraseLine';
    final confirmed = await showWalletConfirm(
      context,
      title: l10n.securityDeleteDialogTitle,
      body: body,
      cancelLabel: l10n.securityDeleteDialogCancel,
      confirmLabel: l10n.securityDeleteDialogConfirm,
      kind: WalletConfirmKind.destructive,
    );
    if (!confirmed || !mounted) return;

    setState(() => _deleting = true);
    final outcome = await ref
        .read(onboardingControllerProvider.notifier)
        .deleteWallet();
    if (!mounted) return;
    setState(() => _deleting = false);

    switch (outcome) {
      case WalletDeletionOutcome.shredded:
      case WalletDeletionOutcome.failedClosed:
        // shredded → the controller is now Welcome; failedClosed → OnboardingFailed.
        // Both render on the wallet surface, so leave the Security screen for it.
        context.go(WalletRoutes.wallet);
      case WalletDeletionOutcome.failedRecovered:
        // The wipe faulted but the wallet was recovered (no data lost) — stay put
        // and tell the truth, never a false "deleted".
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(l10n.securityDeleteFailedSnack)));
      case WalletDeletionOutcome.notDeletable:
        // Over a wallet-holding state a refusal means another lifecycle
        // mutation is still in flight — a server switch (M01: a delete is
        // refused, never raced). Nothing was touched; name the wait, which the
        // switch timeout bounds. Any other state is the stray-call no-op.
        final settled = ref.read(onboardingControllerProvider);
        if (settled is! OnboardingActive &&
            settled is! OnboardingAwaitingBackup) {
          break;
        }
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              l10n.securityDeleteRefusedBusySnack(
                OnboardingController.switchServerTimeout.inSeconds,
              ),
            ),
          ),
        );
    }
  }
}

/// The key-custody group's one row, and the destructive group's: keys the
/// S13 tests find the groups by (their ORDER is the contract — delete last).
@visibleForTesting
const securityCustodyGroupKey = ValueKey('security-group-custody');

/// The backup / watch-only + export group.
@visibleForTesting
const securityKeysGroupKey = ValueKey('security-group-keys');

/// The destructive group — always the screen's last.
@visibleForTesting
const securityDeleteGroupKey = ValueKey('security-group-delete');

/// The delete-wallet row: the error role on its glyph and label, the
/// explanation behind its (i) (S13 Build B), and a spinner in place of the
/// glyph while the shred runs (the row is disabled then).
class _DeleteRow extends StatelessWidget {
  const _DeleteRow({
    required this.deleting,
    required this.explanation,
    required this.onTap,
  });

  final bool deleting;
  final String explanation;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final error = Theme.of(context).colorScheme.error;
    return ListTile(
      key: const ValueKey('security-delete-row'),
      enabled: !deleting,
      onTap: onTap,
      // The error role while live; the disabled tile greys it with the rest.
      textColor: error,
      iconColor: error,
      leading: deleting
          ? const SizedBox(
              width: 18,
              height: 18,
              child: CircularProgressIndicator.adaptive(strokeWidth: 2),
            )
          : const WalletIcon(WalletGlyph.deleteWallet),
      title: Text(l10n.securityDeleteWalletButton),
      trailing: WalletInfoButton(
        label: l10n.securityDeleteWalletButton,
        body: explanation,
      ),
    );
  }
}

/// The erase line for [assurance]: ONE mapping for the custody row and the delete
/// confirmation, so the two can never say different things.
String _eraseLine(WalletLocalizations l10n, EraseAssurance assurance) =>
    switch (assurance) {
      EraseAssurance.hardwareKeyDeleted => l10n.securityCustodyHardwareKey,
      // A value from a newer core reads as best-effort: never more custody than
      // this package can name (ADR-0571).
      EraseAssurance.bestEffort ||
      EraseAssurance.unknown => l10n.securityCustodyBestEffort,
    };

/// The custody-tier row — the FR-14 H1 honesty surface. Renders the measured
/// tier name (the STATE, on screen) + an HONEST erase line keyed off
/// `eraseAssurance` (NOT `degraded`, which is false for both `none` and the raw
/// keychain), behind the tier's (i) (S13 Build B). The same line is restated in
/// the delete confirmation at the moment of decision.
class _CustodyRow extends StatelessWidget {
  const _CustodyRow({required this.disclosure});

  final AsyncValue<CustodyDisclosure> disclosure;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    return Padding(
      padding: const EdgeInsetsDirectional.fromSTEB(16, 12, 4, 12),
      child: disclosure.when(
        loading: () => Row(
          children: [
            const SizedBox(
              width: 18,
              height: 18,
              child: CircularProgressIndicator.adaptive(strokeWidth: 2),
            ),
            const SizedBox(width: 12),
            // Expanded, like the error arm: a bare Text in a Row
            // overflowed 82 px at 2.0x on a 320 dp phone.
            Expanded(
              child: Text(
                l10n.securityCustodySectionTitle,
                style: textTheme.bodyMedium,
              ),
            ),
          ],
        ),
        error: (_, _) => Row(
          children: [
            WalletIcon(WalletGlyph.unknown, color: colors.textMuted),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                l10n.securityCustodyProbeError,
                style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
              ),
            ),
          ],
        ),
        data: (d) {
          final hardware =
              d.eraseAssurance == EraseAssurance.hardwareKeyDeleted;
          // A hardware-held key reads positive; everything else is an honest
          // best-effort caution — NEVER gated on !degraded.
          final accent = hardware ? colors.green : colors.orange;
          final tier = _tierName(l10n, d.tier);
          return Row(
            children: [
              WalletIcon(
                hardware ? WalletGlyph.verified : WalletGlyph.shielded,
                color: accent,
              ),
              const SizedBox(width: 12),
              Expanded(child: Text(tier, style: textTheme.titleMedium)),
              WalletInfoButton(
                label: tier,
                body: _eraseLine(l10n, d.eraseAssurance),
              ),
            ],
          );
        },
      ),
    );
  }

  /// Map the SDK tier label to a human display name. The label is diagnostic;
  /// the boolean predicates drive the honesty, so an unrecognised (newer-core)
  /// tier falls back to "Unknown" rather than guessing.
  static String _tierName(WalletLocalizations l10n, String tier) =>
      switch (tier) {
        'apple_secure_enclave' => l10n.securityCustodyTierSecureEnclave,
        'strongbox' => l10n.securityCustodyTierStrongBox,
        'tee' => l10n.securityCustodyTierTee,
        'software_keystore' => l10n.securityCustodyTierSoftware,
        'apple_keychain' => l10n.securityCustodyTierKeychain,
        'none' => l10n.securityCustodyTierNone,
        _ => l10n.securityCustodyTierUnknown,
      };
}
