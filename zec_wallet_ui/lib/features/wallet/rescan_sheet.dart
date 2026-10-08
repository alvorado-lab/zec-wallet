import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/theme/colors.dart';
import '../../core/theme/icons.dart';
import '../../core/theme/shapes.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../shared/wallet_sheet.dart';
import 'onboarding/onboarding_providers.dart';
import 'onboarding/wallet_provisioner.dart'
    show
        RescanAllHistory,
        RescanFromTime,
        RescanFromWalletBirthday,
        RescanTarget;
import 'sync_status_presentation.dart' show compactBlockCount;
import 'wallet_config.dart';
import 'wallet_providers.dart';
import 'wallet_rescan_controller.dart';

/// Open the rescan-recovery confirm sheet (FR-1b / ADR-0534). A modal bottom
/// sheet so the action is deliberate (a recovery, not a one-tap); it exits ONLY
/// through its buttons (no scrim-tap/drag/back dismiss — UX review: a
/// mid-rescan dismissal left the up-to-a-minute `Running` window with no surface
/// cue and a re-openable sheet whose Start silently no-oped; the session swap
/// must stay visibly owned by this sheet until it resolves). Returns when the
/// sheet closes; the OUTCOME is presented on the wallet surface (the rebuilding
/// cue / failed notice via [walletRescanControllerProvider]), not here, so the
/// surface stays the single source of truth across the session swap.
Future<void> showWalletRescanSheet(BuildContext context) {
  // The shared sheet frame (scroll-controlled for the date control + large
  // text scales, the desktop width cap). Locked: no scrim tap, no drag — and
  // so no drag handle or grabber either.
  return showWalletSheet<void>(
    context,
    isDismissible: false,
    enableDrag: false,
    builder: (_) => const _RescanSheet(),
  );
}

class _RescanSheet extends ConsumerStatefulWidget {
  const _RescanSheet();

  @override
  ConsumerState<_RescanSheet> createState() => _RescanSheetState();
}

/// Below this span the "~N blocks to scan" cue is suppressed (no-magic-numbers,
/// gate 7): a sub-2-block estimate is a rounding artifact of a floor at the
/// tip, not a magnitude worth showing.
const int walletRescanEstimateFloor = 2;

/// The sheet's range selection: still resolving the floor read, the wallet's
/// own start, an explicit user-picked date, or the whole chain.
enum _RescanRange { resolving, walletStart, date, all }

class _RescanSheetState extends ConsumerState<_RescanSheet> {
  /// The range selection. DECIDED once the floor read resolves (see
  /// [_initDefaultRange]): the DEFAULT is the wallet's OWN birthday floor
  /// (#317). This is the ONE always-money-safe default. Rescan is a
  /// DESTRUCTIVE, LOWER-ONLY rebuild (core `rescan_from`): it wipes the data
  /// DB and re-scans from `from` to the tip, so a `from` ABOVE the current
  /// floor HIDES every note between the old floor and `from` until a deeper
  /// rescan. The floor is therefore the highest height that never hides a
  /// funded block — and, being the wallet's birthday, it also never wastes a
  /// scan on empty pre-wallet blocks. A blind "one year
  /// back" default was fund-UNSAFE for any wallet older than a year (it sat
  /// above the floor and cratered the balance on the one-tap action — the
  /// security review BLOCKER). Going BELOW the floor is the post-restore
  /// recovery this sheet exists for — the user reaches it via Pick-a-date /
  /// Scan-all (the copy nudges them there), never the default. Until the read
  /// resolves the range is `resolving` (Start disabled, no target).
  _RescanRange _range = _RescanRange.resolving;

  /// The picked earliest date — set iff [_range] is [_RescanRange.date].
  DateTime? _pickedDate;

  /// The wallet's birthday height, read at open. `null` while the read is in
  /// flight — and after, if it reported no account or the read faulted, in
  /// which case the default degrades to Scan-all (the only money-safe default
  /// when the floor is unknown — see [_initDefaultRange]).
  int? _floor;

  /// True from the moment Start is tapped until the rescan resolves and the sheet
  /// pops. Drives the spinner + disables every control so the rebuild is never
  /// double-triggered from the sheet (the controller also single-flights).
  bool _submitting = false;

  @override
  void initState() {
    super.initState();
    _readFloor();
  }

  Future<void> _readFloor() async {
    final session = ref.read(walletSessionProvider);
    try {
      // Local FFI read; bounded so a wedged bridge degrades to a safe default
      // instead of a Start button that never enables (same posture as the
      // receive-address timeout).
      final floor = session == null
          ? null
          : await session.birthdayHeight().timeout(walletFfiWedgeTimeout);
      if (!mounted) return;
      setState(() => _initDefaultRange(floor));
    } catch (_) {
      // The floor read faulted (a wedged bridge): the floor is UNKNOWN, so the
      // only money-safe default is Scan-all — a blind dated default could sit
      // above the true floor and hide funds (the LOWER-ONLY contract).
      if (!mounted) return;
      setState(() => _initDefaultRange(null));
    }
  }

  /// Choose the default range once the floor read resolves. Applies ONLY while
  /// still `resolving` — a Pick / Scan-all the user tapped during the read
  /// (both stay live) WINS and must not be silently reverted (reliability
  /// HIGH). Always records `_floor` (the size cue reads it).
  void _initDefaultRange(int? floor) {
    _floor = floor;
    if (_range != _RescanRange.resolving) return;
    // A known floor → default to it (never above it: the LOWER-ONLY contract).
    // An UNKNOWN floor (no account yet, or a faulted read) → Scan-all is the
    // only default guaranteed not to hide funds — activation is at/below every
    // real birthday, whereas any dated guess could sit above it.
    _range = floor == null ? _RescanRange.all : _RescanRange.walletStart;
  }

  Future<void> _pickDate() async {
    final now = DateTime.now();
    final picked = await showDatePicker(
      context: context,
      initialDate: _pickedDate ?? DateTime(now.year - 1, now.month),
      // Shared floor with the restore picker (one source of truth) — the SDK
      // floors anything earlier, so bounding here makes the floor visible.
      firstDate: kZcashSaplingActivationDate,
      lastDate: now,
      helpText: WalletLocalizations.of(context).walletRescanDatePick,
    );
    if (!mounted || picked == null) return;
    setState(() {
      _range = _RescanRange.date;
      _pickedDate = picked;
    });
  }

  /// The chosen target — `null` only while the floor read is still resolving
  /// the default (Start stays disabled for that brief window).
  RescanTarget? get _target => switch (_range) {
    _RescanRange.resolving => null,
    _RescanRange.walletStart => RescanFromWalletBirthday(_floor!),
    _RescanRange.date => RescanFromTime(_pickedDate!),
    _RescanRange.all => const RescanAllHistory(),
  };

  Future<void> _confirm() async {
    final target = _target;
    if (target == null) return; // Start is disabled in this window (belt).
    setState(() => _submitting = true);
    // Awaits the WHOLE rescan (running → resolved). The controller never throws
    // — the resulting WalletRescanState IS the surface — so no try/catch here.
    await ref.read(walletRescanControllerProvider.notifier).rescan(target);
    // The sheet's job is done the instant the outcome is set; the wallet surface
    // renders the rebuilding cue / failed notice. Re-check mounted (an awaited
    // gap) before popping.
    if (mounted) Navigator.of(context).pop();
  }

  /// The approximate scan-start height of the CURRENT selection, for the size
  /// cue — display guidance only (the binding conversion happens in the
  /// adapter at confirm). `null` hides the cue (honest absence over a guess).
  int? _estimateFromHeight() {
    switch (_range) {
      case _RescanRange.resolving:
        return null;
      case _RescanRange.walletStart:
        return _floor;
      case _RescanRange.date:
        // The SAME conservative estimator the adapter converts through, so
        // the cue also makes the estimator's floor visible (a "July 1" pick
        // estimating ~50 days deep is a fact worth showing).
        final date = _pickedDate;
        if (date == null) return null;
        return ref
            .read(walletProvisionerProvider)
            ?.estimateBirthdayHeight(date);
      case _RescanRange.all:
        return ref
            .read(walletProvisionerProvider)
            ?.estimateBirthdayHeight(kZcashSaplingActivationDate);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    // The size cue: tip − start, compact ("About 1.6M blocks to scan"), so the
    // duration warning below has a visible magnitude. Hidden when either end
    // is unknown (no tip yet / floor unresolved / no provisioner) — an honest
    // absence beats a guess. The snapshot tip is the cold-read chain tip; a
    // slightly stale tip skews the cue by minutes of blocks, immaterial at
    // "About …" precision.
    final tip = ref.watch(walletSnapshotProvider).value?.tip;
    final fromHeight = _estimateFromHeight();
    final int? blocks = (tip != null && fromHeight != null)
        ? tip - fromHeight
        : null;
    // Suppress the cue below a trivial span (mirrors the sync countdown's
    // `>= 2` floor): "About 0 blocks to scan" on a wallet whose floor sits at
    // the tip reads as broken, not as a magnitude. The warning copy still
    // carries the human "minutes vs hours" expectation in that case.
    final String? estimate =
        (blocks != null && blocks >= walletRescanEstimateFloor)
        ? l10n.walletRescanEstimate(compactBlockCount(blocks, l10n.localeName))
        : null;
    return PopScope(
      // The sheet is scrim/drag-locked at [showWalletRescanSheet]; this closes
      // the remaining exit (the system back gesture) while the rescan runs, so
      // the ≤~60s worst-case Running window is always covered by THIS sheet's
      // spinner. Cancel (enabled while idle) remains the deliberate way out.
      canPop: !_submitting,
      child: Padding(
        // Lift above the keyboard/gesture inset; the sheet is scrollable so it
        // never overflows at large system text scales.
        padding: EdgeInsets.only(
          left: 24,
          right: 24,
          top: 8,
          bottom: 24 + MediaQuery.viewInsetsOf(context).bottom,
        ),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // No close while the rescan runs (S13 §1.7): the sheet owns
              // the exit until the rebuild resolves, exactly as Back above.
              WalletSheetHeader(
                title: l10n.walletRescanTitle,
                closeKey: const Key('rescan-sheet-close'),
                showClose: !_submitting,
              ),
              const SizedBox(height: 12),
              Text(l10n.walletRescanBody, style: textTheme.bodyMedium),
              const SizedBox(height: 12),
              // #390 cross-pointer: a rescan re-scans compact blocks, which carry
              // no transparent outputs, and its re-seed keeps the restore ceiling —
              // so it CANNOT surface funds from an old swap. The rescan copy's
              // "missing older funds?" framing otherwise misdirects the exact
              // affected user; point them at the deep scan instead.
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  WalletIcon(
                    WalletGlyph.swap,
                    size: 16,
                    color: colors.textMuted,
                  ),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      l10n.walletRescanSwapPointer,
                      style: textTheme.bodySmall?.copyWith(
                        color: colors.textMuted,
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 20),
              _RescanRangeSection(
                range: _range,
                pickedDate: _pickedDate,
                estimate: estimate,
                enabled: !_submitting,
                onPick: _pickDate,
                onScanAll: () => setState(() => _range = _RescanRange.all),
              ),
              const SizedBox(height: 16),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  WalletIcon(
                    WalletGlyph.scheduled,
                    size: 16,
                    color: colors.textMuted,
                  ),
                  const SizedBox(width: 6),
                  Expanded(
                    child: Text(
                      l10n.walletRescanWarning,
                      style: textTheme.bodySmall?.copyWith(
                        color: colors.textMuted,
                      ),
                    ),
                  ),
                ],
              ),
              // Settling-send advisory (#364 M3): with money in motion the
              // engine's W-swap-4-a-3 fence will refuse the rescan — but only
              // AFTER the full quiesce + session teardown the confirm costs.
              // Say so BEFORE Start, from the same in-flight surface the wallet
              // screen renders. Advisory only — Start stays enabled and the
              // ENGINE stays authoritative (the in-flight read is a cautionary
              // view; a send can settle between this frame and the confirm,
              // and a stale advisory must never block a rescan the engine
              // would accept).
              if (ref.watch(walletInFlightSendsProvider).value?.isNotEmpty ??
                  false) ...[
                const SizedBox(height: 12),
                // liveRegion: the in-flight read can resolve a beat after the
                // sheet opens, inserting this advisory silently — a screen
                // reader must hear the refusal warning land before Start.
                Semantics(
                  container: true,
                  liveRegion: true,
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      ExcludeSemantics(
                        child: WalletIcon(
                          WalletGlyph.waiting,
                          size: 16,
                          color: colors.orange,
                        ),
                      ),
                      const SizedBox(width: 6),
                      Expanded(
                        child: Text(
                          l10n.walletRescanSettlingAdvisory,
                          style: textTheme.bodySmall?.copyWith(
                            color: colors.text,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
              const SizedBox(height: 24),
              FilledButton(
                // Also disabled for the brief window while the walletStart
                // default is still resolving its floor (a local read; the
                // fallback arm re-enables it if that read fails).
                onPressed: _submitting || _target == null ? null : _confirm,
                child: _submitting
                    ? Row(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          const SizedBox(
                            height: 18,
                            width: 18,
                            child: CircularProgressIndicator.adaptive(
                              strokeWidth: 2,
                            ),
                          ),
                          const SizedBox(width: 12),
                          // Flexible: a Row child gets unbounded width, so a
                          // long in-progress label at 3x scale on a 320dp phone
                          // overflows instead of wrapping (#407 R10a — the
                          // third and fourth siblings of the #401 R8c shape;
                          // the move sheet got this, these two never
                          // did, and the shield twin was found overflowing 41px
                          // for exactly this reason).
                          Flexible(child: Text(l10n.walletRescanRunning)),
                        ],
                      )
                    : Text(l10n.walletRescanConfirm),
              ),
              const SizedBox(height: 8),
              TextButton(
                // Dismiss without rescanning. Disabled mid-rebuild so a started
                // rescan is never abandoned half-way from the sheet.
                onPressed: _submitting
                    ? null
                    : () => Navigator.of(context).pop(),
                child: Text(l10n.walletRescanCancel),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// The "how far back to scan" control, mirroring the restore birthday section but
/// with RECOVERY semantics: scanning EARLIER recovers MORE (the safe direction).
/// Three states (#317): the wallet's OWN start (the default — complete for this
/// wallet, never a wasted block), a chosen date (the honest "might still miss
/// older funds" cue), and "Scan all history" (most thorough, slowest). The size
/// cue under the description gives the chosen range a visible magnitude.
class _RescanRangeSection extends StatelessWidget {
  const _RescanRangeSection({
    required this.range,
    required this.pickedDate,
    required this.estimate,
    required this.enabled,
    required this.onPick,
    required this.onScanAll,
  });

  final _RescanRange range;

  /// Set iff [range] is [_RescanRange.date].
  final DateTime? pickedDate;

  /// The pre-formatted "About N blocks to scan." cue, or `null` while either
  /// end of the range is unknown (hidden — an honest absence beats a guess).
  final String? estimate;

  final bool enabled;
  final VoidCallback onPick;
  final VoidCallback onScanAll;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    final colors = WalletColors.of(context);
    final description = switch (range) {
      // The floor read is still in flight (sub-ms healthy; up to the FFI
      // timeout on a wedged bridge). Pick/Scan-all stay live so the user is
      // never stuck; only Start waits for a target.
      _RescanRange.resolving => l10n.walletRescanRangeResolving,
      _RescanRange.walletStart => l10n.walletRescanRangeDefault,
      _RescanRange.all => l10n.walletRescanRangeAll,
      _RescanRange.date => l10n.walletRescanRangeChosen(
        MaterialLocalizations.of(context).formatMonthYear(pickedDate!),
      ),
    };
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
            l10n.walletRescanRangeTitle,
            style: textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 6),
          Text(
            description,
            style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
          ),
          if (estimate != null) ...[
            const SizedBox(height: 4),
            Text(
              estimate!,
              style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
            ),
          ],
          const SizedBox(height: 12),
          // A Wrap, not a Row: "Scan all history" moves to its own line, label
          // whole, when it can't share the row. The earlier Flexible + ellipsis
          // (2e-2b-iv, a 0.7px overflow at the 560 cap) kept the row one line
          // by cutting the label; the sheet scrolls, so a second line is safe.
          // Same rule as the restore screen's twin row.
          Wrap(
            spacing: 8,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              OutlinedButton.icon(
                onPressed: enabled ? onPick : null,
                icon: const WalletIcon(WalletGlyph.pickDate, size: 18),
                label: Text(
                  range == _RescanRange.date
                      ? l10n.walletRescanChange
                      : l10n.walletRescanPick,
                ),
              ),
              if (range != _RescanRange.all)
                TextButton(
                  onPressed: enabled ? onScanAll : null,
                  child: Text(l10n.walletRescanScanAll),
                ),
            ],
          ),
        ],
      ),
    );
  }
}
