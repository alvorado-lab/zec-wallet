import 'dart:async';
import 'dart:math' show max;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/wallet_sheet.dart';
import '../move_to_transparent/move_to_transparent_sheet.dart';
import '../wallet_providers.dart';
import '../zat_format.dart';
import 'auto_shield_controller.dart';
import 'transparent_funds_providers.dart';

/// Open the Transparent-funds surface (§3.2i-3 (b)) as a modal bottom sheet —
/// the expert-layer home for the transparent-funds POLICY: a plain-factual
/// explanation, the expert gate ("Advanced: transparent funds", default OFF),
/// and — with the gate ON — the auto-shield switch plus the promoted
/// Move-to-transparent entry. Deliberately NOT a money form: nothing here
/// signs or proposes; the money flows it links to (shield / move) carry their
/// own confirms and disclosures.
Future<void> showTransparentFundsSheet(BuildContext context) {
  return showWalletSheet<void>(
    context,
    builder: (_) => const TransparentFundsSheet(),
  );
}

/// The sheet body (public for widget tests — pump it directly in a
/// `ProviderScope` with a fake [walletSettingsStoreProvider]).
class TransparentFundsSheet extends ConsumerStatefulWidget {
  const TransparentFundsSheet({super.key});

  @override
  ConsumerState<TransparentFundsSheet> createState() =>
      _TransparentFundsSheetState();
}

class _TransparentFundsSheetState extends ConsumerState<TransparentFundsSheet> {
  /// The last toggle write failed — rendered INLINE (a SnackBar draws in the
  /// Scaffold UNDER this modal route and is fully occluded; UX review
  /// M4, measured). Cleared on the next successful write.
  bool _saveFailed = false;

  @override
  void initState() {
    super.initState();
    // Retry-on-open (reliability review MINOR-3, reframed): riverpod
    // 3 already auto-retries failing providers with backoff, so a failed
    // persisted-flag READ is not permanently stuck — but the backoff cap
    // makes recovery lag; opening the policy sheet is the natural IMMEDIATE
    // retry moment. Invalidate ONLY the errored ones (never churn healthy
    // state).
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      if (ref.read(walletExpertTransparentFundsProvider).hasError) {
        ref.invalidate(walletExpertTransparentFundsProvider);
      }
      if (ref.read(walletAutoShieldEnabledProvider).hasError) {
        ref.invalidate(walletAutoShieldEnabledProvider);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    // Fail-safe render directions while the persisted flags load: the gate
    // reads CLOSED and the auto-shield switch shows the shipped default (ON);
    // BOTH switches are disabled while their flag is genuinely LOADING (a tap
    // must never write over a not-yet-read persisted choice). On a READ
    // ERROR the switches stay ENABLED: a write is the recovery path
    // (persist-then-publish repopulates the state truthfully).
    final expertAsync = ref.watch(walletExpertTransparentFundsProvider);
    final expert = expertAsync.value ?? false;
    final autoShieldAsync = ref.watch(walletAutoShieldEnabledProvider);
    final autoShieldOn = autoShieldAsync.value ?? true;
    // #383 R2: a host whose custody can never run an automatic spend hides
    // the whole auto-shield story — no toggle that reads ON but never runs,
    // no automation claim (an honest absence; the loop side is gated too).
    final autoShieldSupported = ref.watch(walletAutoShieldSupportedProvider);
    // The loop's live status colours the automation claim: a denial-latched
    // session must not read "shielded automatically".
    final autoShieldStatus = ref.watch(walletAutoShieldControllerProvider);
    // Copy clamps to the engine floor: a host-lowered threshold must not make
    // the sheet promise a minimum the engine won't honor (m9).
    final minZat = max(
      ref.watch(walletAutoShieldThresholdZatProvider),
      walletAutoShieldFloorZat,
    );

    return SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            WalletSheetHeader(
              title: l10n.walletTransparentFundsTitle,
              closeKey: const Key('transparent-funds-close'),
            ),
            const SizedBox(height: 8),
            // Plain-factual (maintainer call): the visibility FACTS, then a
            // CONDITIONALLY-factual second sentence — the auto-shield claim
            // tracks the actual switch (review M1: the unconditional
            // sentence was false above a visible OFF switch, and false with
            // the gate closed over a persisted OFF). While the flag is
            // loading, the shipped-default sentence is the honest best claim.
            Text(
              l10n.walletTransparentFundsIntro,
              style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
            ),
            // NO automation claim on a READ ERROR: the
            // persisted value is unknown, so both "automatic" and "off"
            // would be guesses — silence over a possibly-false claim (the
            // retry-on-open above usually heals the state).
            if (autoShieldSupported && !autoShieldAsync.hasError) ...[
              const SizedBox(height: 4),
              Text(
                autoShieldOn
                    ? (autoShieldStatus == AutoShieldStatus.denied
                          // The loop is latched off for this session (the host
                          // authorizer declined) — claiming "automatic" here
                          // would be false for the rest of the session (
                          // wrap UX review: this sheet is where a cue-alarmed
                          // user investigates).
                          ? l10n.walletTransparentFundsAutoDenied
                          : l10n.walletTransparentFundsAutoOn(
                              formatZec(minZat),
                            ))
                    : l10n.walletTransparentFundsAutoOff,
                key: const ValueKey('wallet-transparent-funds-auto-claim'),
                style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
              ),
            ],
            // The save-failure line lives ABOVE the switches (wrap UX
            // MAJOR-1, measured): appended at the end of the scroll column it
            // sat off-viewport at 2.0× text scale — feedback the user can't
            // see is the same failure the inline fold fixed for the occluded
            // SnackBar.
            if (_saveFailed) ...[
              const SizedBox(height: 8),
              Text(
                l10n.walletSettingsSaveFailed,
                key: const ValueKey('wallet-settings-save-failed'),
                style: textTheme.bodySmall?.copyWith(color: colors.orange),
              ),
            ],
            const SizedBox(height: 12),
            SwitchListTile(
              key: const ValueKey('wallet-expert-toggle'),
              contentPadding: EdgeInsets.zero,
              title: Text(l10n.walletExpertToggleLabel),
              // The description must not advertise "turning automatic
              // shielding off" when the host declared auto-shield unsupported
              // (the R2 honest-absence rule): flipping the gate then
              // reveals only the Move entry, and the promised switch never
              // appears.
              subtitle: Text(
                autoShieldSupported
                    ? l10n.walletExpertToggleDescription
                    : l10n.walletExpertToggleDescriptionNoAutoShield,
              ),
              value: expert,
              onChanged: expertAsync.isLoading
                  ? null
                  : (enabled) => unawaited(
                      _setFlag(
                        () => ref
                            .read(walletExpertTransparentFundsProvider.notifier)
                            .set(enabled: enabled),
                      ),
                    ),
            ),
            if (expert) ...[
              if (autoShieldSupported)
                SwitchListTile(
                  key: const ValueKey('wallet-auto-shield-toggle'),
                  contentPadding: EdgeInsets.zero,
                  title: Text(l10n.walletAutoShieldToggleLabel),
                  subtitle: Text(
                    l10n.walletAutoShieldToggleDescription(formatZec(minZat)),
                  ),
                  value: autoShieldOn,
                  onChanged: autoShieldAsync.isLoading
                      ? null
                      : (enabled) => unawaited(
                          _setFlag(
                            () => ref
                                .read(walletAutoShieldEnabledProvider.notifier)
                                .set(enabled: enabled),
                          ),
                        ),
                ),
              const SizedBox(height: 8),
              // The promoted unshield entry (§3.2i-3 (b)) — the same
              // deliberate, warned §3.2i-1 flow the overflow menu keeps; this
              // sheet just makes it discoverable where the policy lives. The
              // move sheet stacks above and drives its own outcome.
              Align(
                // Directional: in RTL the entry must sit at the reading START
                // (fold — the sheet-dismiss precedent).
                alignment: AlignmentDirectional.centerStart,
                child: OutlinedButton.icon(
                  key: const ValueKey('wallet-move-entry'),
                  onPressed: () =>
                      unawaited(showMoveToTransparentSheet(context)),
                  icon: const WalletIcon(WalletGlyph.transparent, size: 18),
                  label: Text(l10n.walletMoveMenuItem),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  /// Persist a toggle; on a failed write the switch stays at its persisted
  /// value (persist-then-publish) and an INLINE line says the save failed —
  /// never a switch that lies, never a message hidden under the sheet.
  Future<void> _setFlag(Future<void> Function() write) async {
    try {
      await write();
      if (mounted && _saveFailed) setState(() => _saveFailed = false);
    } catch (_) {
      if (!mounted) return;
      setState(() => _saveFailed = true);
    }
  }
}
