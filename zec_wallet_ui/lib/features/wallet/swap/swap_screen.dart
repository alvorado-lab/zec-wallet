import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../../core/router/wallet_navigation.dart';
import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import '../../../core/theme/shapes.dart';
import '../../../core/theme/typography.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/address_text.dart';
import '../../../shared/decimal_input_formatter.dart';
import '../../../shared/wallet_ack_checkbox.dart';
import '../../../shared/wallet_info_button.dart';
import '../../../shared/wallet_notice.dart';
import '../onboarding/onboarding_providers.dart';
import '../send/zec_amount.dart';
import '../sync_status_presentation.dart' show walletCompactTimeFormat;
import '../wallet_providers.dart';
import '../wallet_sync_controller.dart' show walletSyncPassesRunProvider;
import '../wallet_rescan_controller.dart'
    show WalletCatchUpNone, walletCatchUpCueProvider;
import '../zat_format.dart';
import 'swap_assets.dart';
import 'swap_chains.dart';
import 'swap_controller.dart';
import 'swap_deposit_screen.dart';
import 'swap_address_scanner.dart';
import 'swap_enabled_provider.dart';
import 'swap_scanned_address.dart';
import 'swap_slippage.dart';
import 'swap_state.dart';
import 'swap_status_provider.dart';
import 'swap_token_icon.dart';
import 'swap_token_picker.dart';

/// The swap screen (D-2b-1): form → quote → confirm (§2.6 privacy disclosure,
/// blocking acknowledgment) → execute → live tracking. OutOfZec (ZEC → a foreign
/// asset) — the wallet sends the §4.4 deposit; the user confirms and tracks.
/// Rendering layer ONLY — every step runs through [swapControllerProvider]; Rust
/// is the single source of truth (design invariant 1).
///
/// Reachable only from the active wallet surface AND only when the host has
/// enabled swap ([swapEnabledProvider]) — so a backed-up wallet plus a host that
/// turned swap ON are the structural preconditions (§3.5 layers 1/2). A deep-link
/// with swap off, or no wallet, renders the honest unavailable state.
///
/// Stateful so it can (a) hold the form's ephemeral text controllers + the chosen
/// asset + the disclosure acknowledgment across the quote round-trip, and
/// (b) request screenshot/recents protection ([ScreenSecurity]) for its lifetime —
/// the screen shows amounts + a destination address (sensitive financial data
/// that must not land in the app-switcher snapshot). FLAG_SECURE is Android-only;
/// iOS/desktop rely on the manager-carried native cover (honest degradation).
class SwapScreen extends ConsumerStatefulWidget {
  const SwapScreen({super.key});

  @override
  ConsumerState<SwapScreen> createState() => _SwapScreenState();
}

class _SwapScreenState extends ConsumerState<SwapScreen> {
  final _amountController = TextEditingController();
  final _destinationController = TextEditingController();

  /// IntoZec: the user's source-chain refund address (§3.3b D6). Separate from
  /// [_destinationController] (OutOfZec receive) so a direction flip never reuses
  /// the wrong field's text.
  final _refundController = TextEditingController();

  /// The form direction (§3.3b L8). DEFAULT = IntoZec (buy ZEC — a privacy
  /// wallet's default).
  SwapFlowDirection _direction = SwapFlowDirection.intoZec;

  /// OutOfZec: the picked TARGET asset (from the same dynamic D5 token list the
  /// IntoZec source picker uses). `null` until the user picks one — Get-quote
  /// stays disabled until then (symmetric with [_sourceAsset]; there is no static
  /// default menu any more).
  SwapAsset? _targetAsset;

  /// IntoZec: the picked source asset (from the dynamic D5 token list). `null`
  /// until the user picks one — Get-quote stays disabled until then.
  SwapAsset? _sourceAsset;

  /// The user-tuned slippage tolerance (§3.3b D4), basis points. Opens at the SDK
  /// default; the SDK re-enforces the hard ceiling regardless.
  int _slippageBps = kSlippageDefaultBps;

  /// The §2.6 blocking acknowledgment — Start swap is disabled until checked.
  /// Reset to false on each NEW QUOTE's review (a fresh quote re-requires the
  /// ack). Keyed by quote OBJECT IDENTITY, not by state transition (#364 F10):
  /// a host-prompt denial (and the #367 store-busy retry) restores the review
  /// carrying the SAME [SwapQuote] instance the user already acknowledged —
  /// wiping both acks there punished the user for the host's prompt without
  /// any new fact to re-acknowledge. NOT keyed by `quote.id` (security
  /// review HIGH): the id is provider-controlled opaque data with no
  /// uniqueness contract — a provider recycling an order id across a
  /// re-quote with EDITED inputs would otherwise arrive with both blocking
  /// acks pre-checked over facts (a new refund address, new amounts) the
  /// user never verified. Object identity is forgery-proof and exactly
  /// matches the two restore paths the retention exists for.
  bool _acknowledged = false;

  /// The DISTINCT address-verification acknowledgment — separate from the
  /// privacy ack; Start swap requires BOTH. IntoZec: the refund address (§3.3b
  /// D6 / L8). OutOfZec: the payout address (N03, 2026-10-06 follow-up review).
  /// Reset on each new quote's review (see [_acknowledged]).
  bool _addressAcknowledged = false;

  /// The quote INSTANCE the current ack pair belongs to — the reset key
  /// (#364 F10; identity, never `id` — see [_acknowledged]).
  SwapQuote? _ackedQuote;

  /// Anchors the inline form fault for scroll-to-fault (#364 S5, mirroring the
  /// send form's #329-2): on a new fault the listener below scrolls it into
  /// view at the foot of the form, beside the Get-quote button it explains.
  final _formFaultKey = GlobalKey();

  /// Set by the review countdown when the quote's display deadline passes — Start
  /// swap is then disabled (the SDK ALSO gates execute on the real monotonic+wall
  /// deadline; this is the honest UI that stops a user confirming a dead quote and
  /// tells them to re-quote). Reset on each new review.
  bool _quoteExpired = false;

  /// Captured in [initState] so [dispose] can release protection WITHOUT touching
  /// `ref` (Riverpod guidance — `ref` is unsafe once deactivated).
  late final ScreenSecurity _security;

  @override
  void initState() {
    super.initState();
    _security = ref.read(screenSecurityProvider);
    unawaited(_security.enable());
    // Entry hygiene: the controller is non-autoDispose (root-scoped), so clear
    // a stale PRE-commitment leftover (a lingering review/fault) to a clean
    // form. Deliberately NOT a wipe of a live committed swap — resetToForm
    // no-ops on SwapExecuted/SwapAwaitingDeposit since W-swap-5 (#366), so
    // re-entry RE-ATTACHES to money in motion; leaving a live swap is the
    // terminal/error card's explicit Done. Post-frame so a provider is never
    // mutated mid-build; re-check mounted inside (the project pattern).
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      ref.read(swapControllerProvider.notifier).resetToForm();
    });
  }

  @override
  void dispose() {
    unawaited(_security.disable());
    _amountController.dispose();
    _destinationController.dispose();
    _refundController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final session = ref.watch(walletSessionProvider);
    final swapEnabled = ref.watch(swapEnabledProvider);
    final swapState = ref.watch(swapControllerProvider);

    // Identity-switch hygiene (wrap review — the duress journey): the
    // typed amount/destination/refund are SCREEN-owned; clear them on a
    // wallet-session flip so the OLD identity's draft (a foreign payout
    // address is itself sensitive) never renders in the NEW identity's form.
    ref.listen(walletSessionProvider, (_, _) {
      _amountController.clear();
      _destinationController.clear();
      _refundController.clear();
      // The ack triple gets the SAME identity hygiene (security review
      // MED): with the object-identity key a stale ack can never match a new
      // identity's quote anyway, but clearing here makes the fence hold by
      // construction, not by the key's strength (the #381 posture).
      _ackedQuote = null;
      if (_acknowledged || _addressAcknowledged) {
        setState(() {
          _acknowledged = false;
          _addressAcknowledged = false;
        });
      }
    });
    // Side effects on controller transitions (listen, not watch, so these are
    // reactions, never build-time mutations):
    ref.listen<SwapFlowState>(swapControllerProvider, (prev, next) {
      // Reset BOTH acknowledgments whenever a NEW QUOTE's review arrives — a
      // fresh quote must be acknowledged again (§2.6 blocking-UX integrity),
      // including the address-verification ack (the IntoZec refund, §3.3b D6;
      // the OutOfZec payout, N03). Keyed by
      // quote OBJECT IDENTITY, NOT the state transition and NOT `quote.id`
      // (#364 F10 + security review): a host-prompt denial and the #367
      // store-busy retry both restore the review carrying the SAME SwapQuote
      // instance — the earlier `prev is! SwapReview` edge wiped the acks
      // there, forcing a re-acknowledgment with no new fact behind it — while
      // any freshly-quoted review is a new instance and always resets, even
      // if the provider recycled the (uncontracted, provider-controlled) id.
      // `_quoteExpired` rides the same key: a same-quote restore must NOT
      // clear it (the quote's clock kept running; the build-time derived
      // check stays authoritative).
      if (next is SwapReview && !identical(next.quote, _ackedQuote)) {
        _ackedQuote = next.quote;
        if (_acknowledged || _addressAcknowledged || _quoteExpired) {
          setState(() {
            _acknowledged = false;
            _addressAcknowledged = false;
            _quoteExpired = false;
          });
        }
      }
      // Scroll a NEW form fault into view (#364 S5, the send form's #329-2
      // mirror): the fault renders at the foot of the form and, at large text
      // scales, below the fold — without this the form just "doesn't work"
      // with the explanation off-screen. Identity comparison (not equality):
      // validation faults are constructed fresh per attempt (see the
      // controller's non-canonical fault sets), so a same-reason repeat after
      // the user scrolled away still re-scrolls. The paired screen-reader
      // announcement is [_FormFault]'s own liveRegion.
      final prevFault = prev is SwapFormState ? prev.fault : null;
      final nextFault = next is SwapFormState ? next.fault : null;
      if (nextFault != null && !identical(nextFault, prevFault)) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted) return;
          final faultContext = _formFaultKey.currentContext;
          if (faultContext == null) return;
          // alignment 1.0 pins the fault to the viewport's trailing edge —
          // maximizing the space ABOVE it for the offending field. Honest
          // tradeoff vs the send form (review M2): SEND pins its
          // Review bar OUTSIDE the list, so its fault lands beside an
          // always-visible button; HERE Get-quote lives INSIDE the ListView
          // and can end up below the fold after the scroll — the user fixes
          // the field, then swipes down once to re-reach the button. Chosen
          // anyway: keeping the field visible is what makes the fault
          // actionable; the button is one swipe, the field is the work.
          // No-op on a short form.
          unawaited(
            Scrollable.ensureVisible(
              faultContext,
              alignment: 1.0,
              duration: const Duration(milliseconds: 250),
              curve: Curves.easeOut,
            ),
          );
        });
      }
      // Money just moved: execute queued the §4.4 ZEC deposit, so the wallet's
      // spendable/pending balance changed. Invalidate the cold snapshot NOW, at
      // the money-movement transition, so the wallet surface reflects it however
      // the user later leaves (Done, system back, or backgrounding) — not only on
      // a Done tap. (The send screen refreshes on its result-screen Done; swap
      // moves money one step earlier at execute, so it refreshes one step earlier
      // and more robustly.)
      if (next is SwapExecuted && prev is! SwapExecuted) {
        ref.invalidate(walletSnapshotReadProvider);
        // The durable swap HOME lists the fresh record on the SAME
        // money-movement edge (W-swap-5, #366) — without this the new swap
        // would not appear "in progress" on the wallet screen until the next
        // sync edge or resume.
        ref.invalidate(walletInFlightSwapsReadProvider);
      }
      // IntoZec commits at execute too but lands on the DEPOSIT screen first
      // (SwapExecuted only after markDepositSent) — its home row exists from
      // the execute moment, so the home refreshes on this edge as well. The
      // snapshot is untouched here: an IntoZec execute moves no wallet funds.
      if (next is SwapAwaitingDeposit && prev is! SwapAwaitingDeposit) {
        ref.invalidate(walletInFlightSwapsReadProvider);
      }
    });

    final Widget body;
    if (session == null) {
      body = _SwapUnavailable(message: l10n.walletSwapUnavailableWallet);
    } else if (!swapEnabled &&
        swapState is! SwapExecuted &&
        swapState is! SwapAwaitingDeposit) {
      // §3.5 layer 2/UI isolation: swap off ⇒ no swap surface for the PRE-execute
      // phases (defensive — the entry button is also gated, so this catches a
      // deep-link). A COMMITTED swap is NOT swallowed here (#366-d): an
      // already-EXECUTED swap's tracking view renders "tracking unavailable"
      // from the kill state with the in-flight funds-safety reassurance (§3.5),
      // and an IntoZec [SwapAwaitingDeposit] keeps its deposit screen — the
      // pre-#366 gate dropped it to a generic "off", stranding the user
      // mid-deposit with the address/amount/memo gone and the IntoZec-aware
      // reassurance unreachable.
      //
      // #397 §3.7 D3: for a WATCH-ONLY wallet this deep-link
      // catch was the ONE reachable swap copy, and the generic "isn't
      // available right now" implies transience — false for a wallet that can
      // structurally never swap. Say the permanent truth instead.
      body = _SwapUnavailable(
        message: ref.watch(isWatchOnlyProvider)
            ? l10n.walletSwapUnavailableWatchOnly
            : l10n.walletSwapUnavailableOff,
      );
    } else {
      body = _body(swapState);
    }

    return Scaffold(
      appBar: AppBar(title: Text(l10n.walletSwapTitle)),
      body: SafeArea(child: body),
    );
  }

  Widget _body(SwapFlowState state) {
    final l10n = WalletLocalizations.of(context);
    return switch (state) {
      SwapFormState(:final fault) => _buildForm(fault),
      SwapQuoting() => _SwapBusy(label: l10n.walletSwapQuoting),
      SwapReview(
        :final quote,
        :final direction,
        :final counter,
        :final payoutAddress,
        :final fault,
      ) =>
        _buildReview(quote, direction, counter, payoutAddress, fault),
      // The execute pop-guard (#367): money is being committed (the durable
      // claim + provider intent + the signed deposit enqueue) — a back/pop
      // mid-execute would drop the user on the wallet screen with no cue that
      // anything is happening until the home row lands. Block the pop for the
      // bounded window ([kSwapNetworkTimeout] backstops the state itself) and
      // say so; every outcome (tracking, fault, timeout→track) is pop-able.
      SwapExecuting() => PopScope(
        canPop: false,
        onPopInvokedWithResult: (didPop, _) {
          if (didPop) return;
          ScaffoldMessenger.of(context)
            ..clearSnackBars()
            ..showSnackBar(
              SnackBar(content: Text(l10n.walletSwapExecuteStillWorking)),
            );
        },
        child: _SwapBusy(label: l10n.walletSwapExecuting),
      ),
      SwapAwaitingDeposit(:final quote, :final counter) => SwapDepositView(
        quote: quote,
        counter: counter,
        onConfirmSent: () =>
            ref.read(swapControllerProvider.notifier).markDepositSent(),
        onLeave: () => context.leaveToWalletRoot(),
        // The expired-window escape (HIGH): startNewSwap clears the
        // latched live state, so this arm re-renders as the FORM — the label
        // is literally true here (no navigation). The durable record stays
        // home-listed.
        onStartNew: () =>
            ref.read(swapControllerProvider.notifier).startNewSwap(),
      ),
      SwapExecuted(:final swapId, :final direction, :final reattached) =>
        _SwapTrackingView(
          swapId: swapId,
          direction: direction,
          reattached: reattached,
        ),
    };
  }

  /// The W-swap-5 guard-fault re-attach (#366): when the form fault is "a swap
  /// is already in progress", offer "View swap" pointing AT the tracked swap —
  /// making the §4.4 guard copy's promise true in the shipped UI. The fault
  /// maps ONLY from the ONE-OutOfZec-deposit-in-flight guard, so the OutOfZec
  /// record IS the swap the fault is about — never an IntoZec row (closer
  /// review: attaching one would open a swap that cannot be the guard's
  /// cause). Returns null (copy-only fault, the pre-#366 rendering) when no
  /// OutOfZec record is listed (dismissed/lapsed — the guard then self-clears
  /// within its deadline).
  VoidCallback? _viewSwapAction(SwapFormFault fault) {
    if (fault is! SwapCategoricalFault ||
        fault.reason != SwapFaultReason.swapInFlight) {
      return null;
    }
    final records =
        ref.watch(walletInFlightSwapsProvider).value ?? const <SwapRecord>[];
    for (final record in records) {
      if (record.direction != SwapRecordDirection.outOfZec) continue;
      return () => ref
          .read(swapControllerProvider.notifier)
          .attachTo(swapId: record.id, direction: SwapFlowDirection.outOfZec);
    }
    return null;
  }

  /// The editable form (§3.3b L8): one shared form with a direction flip, then
  /// the direction-specific fields + the shared slippage control. Inlined (not a
  /// child widget) so the Get-quote action reads the controllers + asset/token
  /// the screen owns.
  Widget _buildForm(SwapFormFault? fault) {
    final l10n = WalletLocalizations.of(context);
    return ListView(
      padding: const EdgeInsets.all(16),
      // Keep every child INFLATED (review F1): an async fault
      // (quote timeout / connection) re-lands after the _SwapBusy interlude
      // rebuilt this ListView at offset 0 — with the default lazy cache
      // extent, a below-the-fold fault card at large text scale was never
      // inflated, so BOTH the scroll-to-fault (currentContext null) and its
      // live-region announcement (no semantics node) silently no-oped in the
      // exact scenario they exist for. The form is ~a dozen children;
      // inflating them all is trivially cheap.
      cacheExtent: 1e5,
      children: [
        _DirectionToggle(direction: _direction, onChanged: _onDirectionChanged),
        const SizedBox(height: 20),
        ...switch (_direction) {
          SwapFlowDirection.intoZec => _intoZecFields(),
          SwapFlowDirection.outOfZec => _outOfZecFields(),
        },
        const SizedBox(height: 16),
        SlippageControl(
          valueBps: _slippageBps,
          onChanged: (bps) => setState(() => _slippageBps = bps),
        ),
        if (fault != null) ...[
          const SizedBox(height: 16),
          // KeyedSubtree carries the scroll anchor; the INNER ObjectKey forces
          // a REMOUNT per fault instance (review M3): a same-reason
          // repeat carries identical text, and a live region only announces
          // on semantics CHANGE — without the remount the sighted channel got
          // the re-scroll while the screen-reader channel stayed silent (the
          // S5 invariant half-applied). Remount recreates the semantics node,
          // which announces on creation. Validation faults are fresh
          // instances per attempt (the controller's non-const invariant), so
          // the ObjectKey genuinely changes.
          KeyedSubtree(
            key: _formFaultKey,
            child: _FormFault(
              key: ObjectKey(fault),
              fault: fault,
              onViewSwap: _viewSwapAction(fault),
            ),
          ),
        ],
        const SizedBox(height: 24),
        FilledButton(
          // Disabled until the direction's asset is picked (the picker field shows
          // the "select an asset" cue); empty amount/destination then surface as
          // honest inline faults from the controller.
          onPressed: _canQuote ? _quote : null,
          child: Text(l10n.walletSwapQuoteButton),
        ),
      ],
    );
  }

  /// IntoZec fields (foreign → ZEC), in reading order: the dynamic source-asset
  /// picker, then the FOREIGN amount, then the refund address LAST (§3.3b L8).
  /// Asset → amount → refund follows the dependency: a refund address belongs to
  /// the SOURCE chain, so the field that determines that chain (the asset) comes
  /// first — by the time you fill the refund field you already know which chain
  /// it must be on (maintainer UX). The refund field stays chain-aware (its label /
  /// helper / token icon resolve from the picked asset above it).
  List<Widget> _intoZecFields() {
    final l10n = WalletLocalizations.of(context);
    final source = _sourceAsset;
    final symbol = source?.symbol.toUpperCase();
    // The friendly chain name ("Ethereum", not "ETH") for the refund label/helper.
    final chain = source == null ? null : chainDisplayName(source.chain);
    return [
      _AssetPickerField(
        fieldKey: const Key('swap-source-asset-field'),
        asset: _sourceAsset,
        onTap: _pickSourceAsset,
        label: l10n.walletSwapSourceAssetLabel,
        hint: l10n.walletSwapSourceAssetHint,
      ),
      const SizedBox(height: 16),
      TextField(
        key: const Key('swap-into-amount'),
        controller: _amountController,
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        // Foreign decimal: digits + one separator at the input layer. The SDK is
        // the real gate (the foreign decimal is validated Rust-side, NOT parsed
        // to zatoshis host-side — §3.3b L8).
        inputFormatters: const [WalletDecimalInputFormatter()],
        decoration: InputDecoration(
          labelText: symbol == null
              ? l10n.walletSwapForeignAmountLabelGeneric
              : l10n.walletSwapForeignAmountLabel(symbol),
          hintText: l10n.walletSwapAmountHint,
        ),
      ),
      const SizedBox(height: 16),
      TextField(
        key: const Key('swap-into-refund'),
        controller: _refundController,
        autocorrect: false,
        enableSuggestions: false,
        // Wrap a long source-chain address so the WHOLE thing fits in the field
        // (a single line just scrolls the tail off — maintainer UX). Monospace for
        // legibility.
        minLines: 1,
        maxLines: 3,
        style: WalletTypography.of(
          context,
        ).monoOn(const TextStyle(fontSize: 14, height: 1.3)),
        decoration: InputDecoration(
          // Chain-aware once a source asset is picked — "Your NEAR refund address".
          labelText: chain == null
              ? l10n.walletSwapRefundLabel
              : l10n.walletSwapRefundLabelChain(chain),
          hintText: l10n.walletSwapRefundHint,
          helperText: chain == null
              ? l10n.walletSwapRefundHelper
              : l10n.walletSwapRefundHelperChain(chain),
          helperMaxLines: 3,
          alignLabelWithHint: true,
          // A graphic for the selected source token: its icon marks the chain the
          // refund address must be on (maintainer UX). Null until an asset is chosen.
          prefixIcon: source == null
              ? null
              : Padding(
                  padding: const EdgeInsets.all(8),
                  child: SwapTokenIcon(
                    symbol: source.symbol,
                    chain: source.chain,
                    size: 24,
                  ),
                ),
          // Scan (camera-bearing platforms only — paste/type is always the
          // canonical path) + the "what's a refund address?" explainer.
          suffixIcon: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (ref.watch(addressScannerSupportedProvider))
                IconButton(
                  icon: const WalletIcon(WalletGlyph.scanQr),
                  tooltip: l10n.walletSwapRefundScanTooltip,
                  onPressed: _scanRefundAddress,
                ),
              WalletInfoButton(
                key: const Key('swap-refund-info'),
                label: l10n.walletSwapRefundLabel,
                title: l10n.walletSwapRefundInfoTitle,
                body: l10n.walletSwapRefundInfoBody,
              ),
            ],
          ),
        ),
      ),
    ];
  }

  /// OutOfZec fields (ZEC → foreign): the spendable balance, the dynamic TARGET-
  /// asset picker (the same `/v0/tokens` list the IntoZec source picker uses), the
  /// ZEC amount, then the chain-aware receive destination BOTTOM (§3.3b L8) —
  /// symmetric with the IntoZec form. Asset → amount → destination follows the
  /// dependency: the destination belongs to the TARGET chain, so the field that
  /// determines that chain (the asset) comes first; by the time you fill the
  /// destination you already know which chain it must be on, and the field is
  /// chain-aware (label / helper / token icon resolve from the picked asset).
  List<Widget> _outOfZecFields() {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final spendable = ref
        .watch(walletSnapshotProvider)
        .value
        ?.balance
        .spendableZat;
    final target = _targetAsset;
    // The friendly chain name ("Ethereum", not "ETH") for the destination label/
    // helper/icon, once a target asset is picked.
    final chain = target == null ? null : chainDisplayName(target.chain);
    return [
      if (spendable != null)
        Padding(
          padding: const EdgeInsets.only(bottom: 16),
          child: Text(
            // While the wallet is still catching up (#380), the spendable
            // figure is the PARTIAL repopulating balance — qualify it so a
            // low/zero "Available" is never read as lost funds. The send and
            // move Available lines apply the same rule (#381 — the
            // review found this fold was NOT the last such surface).
            // The qualifier drops when no pass will run (#405 → the
            // SSOT): no active-progress claim while nothing runs; the badge
            // story carries the caveat.
            ref.watch(walletCatchUpCueProvider) is WalletCatchUpNone ||
                    !ref.watch(walletSyncPassesRunProvider)
                ? l10n.walletSwapAvailable(formatZec(spendable))
                : l10n.walletSwapAvailableCatchingUp(formatZec(spendable)),
            style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
          ),
        ),
      _AssetPickerField(
        fieldKey: const Key('swap-target-asset-field'),
        asset: _targetAsset,
        onTap: _pickTargetAsset,
        label: l10n.walletSwapAssetLabel,
        hint: l10n.walletSwapTargetAssetHint,
      ),
      const SizedBox(height: 16),
      TextField(
        key: const Key('swap-out-amount'),
        controller: _amountController,
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        // Money is integer-exact: digits and one separator (a comma reads as
        // the point) at the input layer (the parser is the real gate).
        inputFormatters: const [WalletDecimalInputFormatter()],
        decoration: InputDecoration(
          labelText: l10n.walletSwapAmountLabel,
          hintText: l10n.walletSwapAmountHint,
        ),
      ),
      const SizedBox(height: 16),
      TextField(
        key: const Key('swap-out-destination'),
        controller: _destinationController,
        autocorrect: false,
        enableSuggestions: false,
        // Wrap a long foreign-chain address so the WHOLE thing fits in the field
        // (a single line just scrolls the tail off — maintainer UX). Monospace for
        // legibility.
        minLines: 1,
        maxLines: 3,
        style: WalletTypography.of(
          context,
        ).monoOn(const TextStyle(fontSize: 14, height: 1.3)),
        decoration: InputDecoration(
          // Chain-aware once a target asset is picked — "Your Ethereum receiving
          // address" — so a funds-losing cross-chain mistake is harder to make.
          labelText: chain == null
              ? l10n.walletSwapDestinationLabel
              : l10n.walletSwapDestinationLabelChain(chain),
          hintText: l10n.walletSwapDestinationHint,
          helperText: chain == null
              ? null
              : l10n.walletSwapDestinationHelperChain(chain),
          helperMaxLines: 3,
          alignLabelWithHint: true,
          // The picked token's icon marks the chain the destination must be on.
          prefixIcon: target == null
              ? null
              : Padding(
                  padding: const EdgeInsets.all(8),
                  child: SwapTokenIcon(
                    symbol: target.symbol,
                    chain: target.chain,
                    size: 24,
                  ),
                ),
          // Scan (camera-bearing platforms only — paste/type is always the
          // canonical path) via the shared address-scanner brick (the same one
          // the refund field uses; the payload is §5.4-sensitive, NEVER logged).
          suffixIcon: ref.watch(addressScannerSupportedProvider)
              ? IconButton(
                  key: const Key('swap-destination-scan'),
                  icon: const WalletIcon(WalletGlyph.scanQr),
                  tooltip: l10n.walletSwapDestinationScanTooltip,
                  onPressed: _scanDestinationAddress,
                )
              : null,
        ),
      ),
    ];
  }

  /// Get-quote is enabled once the direction's asset is picked (the picker field
  /// shows the "select an asset" cue until then). The asset has no static default
  /// any more, so it gates the button — a quote is never attempted with no
  /// target/source asset. Empty amount/destination then surface as honest inline
  /// faults from the controller.
  bool get _canQuote => switch (_direction) {
    SwapFlowDirection.outOfZec => _targetAsset != null,
    SwapFlowDirection.intoZec => _sourceAsset != null,
  };

  void _onDirectionChanged(SwapFlowDirection direction) {
    if (direction == _direction) return;
    setState(() => _direction = direction);
    // Clear any stale fault carried over from the other direction's form.
    ref.read(swapControllerProvider.notifier).resetToForm();
  }

  /// Open the shared D5 token picker and map the picked [SwapToken] to a display
  /// [SwapAsset] (icon + "SYMBOL on CHAIN" label), or null if dismissed / the
  /// screen left. Shared by both directions (DRY); the caller assigns it to its
  /// own asset field.
  Future<SwapAsset?> _pickAsset(String pickerTitle) async {
    final token = await showSwapTokenPicker(context, title: pickerTitle);
    if (token == null || !mounted) return null;
    final l10n = WalletLocalizations.of(context);
    return SwapAsset(
      chain: token.chain,
      symbol: token.symbol,
      label: swapTokenLabel(l10n, token.symbol, token.chain),
    );
  }

  /// IntoZec: pick the SOURCE asset (the foreign coin the user pays in).
  Future<void> _pickSourceAsset() async {
    final asset = await _pickAsset(
      WalletLocalizations.of(context).walletSwapPickerTitle,
    );
    if (asset == null || !mounted) return;
    setState(() => _sourceAsset = asset);
  }

  /// OutOfZec: pick the TARGET asset (the foreign coin the user receives) — same
  /// dynamic list as the source picker, symmetric with the IntoZec form. The
  /// sheet heading says "…to receive" (not "…to swap from") for this direction.
  Future<void> _pickTargetAsset() async {
    final asset = await _pickAsset(
      WalletLocalizations.of(context).walletSwapPickerTitleReceive,
    );
    if (asset == null || !mounted) return;
    setState(() => _targetAsset = asset);
  }

  /// Scan a foreign-chain address QR into [target] via the shared scanner brick.
  /// The scan is ADDITIVE — the field stays fully type/paste-able; this only fills
  /// it. The decoded payload is §5.4-sensitive (NEVER logged); we unwrap its URI
  /// envelope ([normalizeScannedAddress]) so a standard BIP-21 / EIP-681 QR yields
  /// a bare address, then the user verifies it at review and the SDK validates it
  /// Rust-side. Shared by both directions (DRY).
  Future<void> _scanAddressInto(TextEditingController target) async {
    final scan = ref.read(addressScannerProvider);
    final raw = await scan(context);
    if (!mounted || raw == null) return;
    setState(() {
      target.text = normalizeScannedAddress(raw);
    });
  }

  /// IntoZec: scan the source-chain refund address (§3.3b D6, IZ-4).
  Future<void> _scanRefundAddress() => _scanAddressInto(_refundController);

  /// OutOfZec: scan the target-chain destination address (symmetric with the
  /// refund scan).
  Future<void> _scanDestinationAddress() =>
      _scanAddressInto(_destinationController);

  Widget _buildReview(
    SwapQuote quote,
    SwapFlowDirection direction,
    SwapAsset counter,
    String? payoutAddress,
    SwapFormFault? fault,
  ) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final intoZec = direction == SwapFlowDirection.intoZec;
    // Expiry is DERIVED at build, not only latched by the countdown's one-shot
    // callback (review fold): a review restored after a busy execute can reuse
    // the live countdown ELEMENT (its onExpired already fired once) while the
    // ack-reset listener cleared the flag — the derived check keeps Start
    // disabled and the busy banner suppressed on the very first frame, with no
    // dependence on callback ordering. Display-only either way — the SDK
    // re-gates execute on the real deadline.
    final quoteExpired =
        _quoteExpired ||
        quote.expiresAt <= DateTime.now().millisecondsSinceEpoch ~/ 1000;
    // Both directions also require the DISTINCT address-verification ack — the
    // IntoZec refund address (§3.3b D6), the OutOfZec payout address (N03); an
    // expired quote can never be confirmed (the SDK re-gates execute regardless).
    // A missing address is never confirmable: an ack against a dash verifies
    // nothing (the security and crypto reviews of the 2026-10-07 range).
    final verifiedAddress = intoZec ? quote.refundTo : payoutAddress;
    final hasAddress = verifiedAddress != null && verifiedAddress.isNotEmpty;
    final canConfirm =
        _acknowledged && _addressAcknowledged && hasAddress && !quoteExpired;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text(l10n.walletSwapReviewTitle, style: textTheme.headlineSmall),
        const SizedBox(height: 16),

        // The load-bearing privacy header, FIRST: OutOfZec de-shields (orange
        // warning); IntoZec ends shielded via the nudged Recv-3 step (positive,
        // honest about the brief transparent leg — §3.3b D1 pinned copy).
        if (!intoZec && quote.disclosure.deshields) ...[
          const _SwapDeshieldWarning(),
          const SizedBox(height: 16),
        ],
        if (intoZec && quote.disclosure.endsShielded) ...[
          const _IntoZecEndsShieldedCard(),
          const SizedBox(height: 16),
        ],

        // The numbers. The ZEC side is integer-exact (zatoshis → exact string);
        // the foreign side is the provider's decimal string verbatim. For IntoZec
        // the ZEC side is the guaranteed MIN out (`zecSideZat`); for OutOfZec it
        // is the exact ZEC in.
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: colors.bgCard,
            borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
            border: Border.all(color: colors.border),
          ),
          child: Column(
            children: [
              _ReviewLine(
                label: l10n.walletSwapYouSendLabel,
                value: intoZec
                    ? l10n.walletSwapForeignValue(quote.amountIn, counter.label)
                    : l10n.walletAmount(formatZec(quote.zecSideZat)),
                emphasize: true,
              ),
              const SizedBox(height: 8),
              _ReviewLine(
                label: l10n.walletSwapYouReceiveLabel,
                value: intoZec
                    ? l10n.walletAmount(formatZec(quote.zecSideZat))
                    : l10n.walletSwapReceiveValue(
                        quote.minAmountOut,
                        counter.label,
                      ),
              ),
              // Fee disclosure (#367): OutOfZec's "You send" is the deposit —
              // NOT the whole debit; the deposit send also pays a ZIP-317
              // network fee, knowable exactly only at execute-sign (there is
              // no swap propose/preview round-trip). Disclose the fee's
              // EXISTENCE honestly rather than omit it; the amounts stay
              // undoctored (no fabricated estimate rendered as a number).
              // IntoZec: the wallet sends nothing — no fee line.
              if (!intoZec) ...[
                const SizedBox(height: 8),
                _ReviewLine(
                  label: l10n.walletSwapNetworkFeeLabel,
                  value: l10n.walletSwapNetworkFeeValue,
                ),
              ],
            ],
          ),
        ),
        const SizedBox(height: 8),

        // IntoZec: the honest guaranteed-floor / max-cost line (§3.3b L8) — the
        // `zecSideZat` floor IS the worst case, no fabricated delta.
        if (intoZec) ...[
          Text(
            l10n.walletSwapIntoZecFloorNote(
              formatZec(quote.zecSideZat),
              formatBpsAsPercent(_slippageBps),
            ),
            style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
          ),
          const SizedBox(height: 8),
        ],

        // A LIVE quote countdown (maintainer UX) — replaces the static "expires
        // shortly". Display-only (the SDK gates the real monotonic+wall deadline);
        // on expiry it flips `_quoteExpired` so Start disables and the honest
        // re-quote message shows — never a silent dead quote.
        _QuoteCountdown(
          expiresAt: quote.expiresAt,
          onExpired: () {
            if (mounted && !_quoteExpired) {
              setState(() => _quoteExpired = true);
            }
          },
        ),
        const SizedBox(height: 16),

        // The address VERIFICATION step FIRST, with its DISTINCT ack — each
        // acknowledgment sits with the block it refers to (maintainer UX: the
        // address ack with the address). IntoZec: the refund address the
        // provider echoed (§3.3b D6 / L8). OutOfZec: the payout address this
        // quote was requested for (N03) — the one irreversible fact the user
        // can still catch here.
        _AddressVerification(
          address: verifiedAddress,
          title: intoZec
              ? l10n.walletSwapRefundVerifyTitle
              : l10n.walletSwapPayoutVerifyTitle,
          body: intoZec
              ? l10n.walletSwapRefundVerifyBody
              : l10n.walletSwapPayoutVerifyBody(counter.label),
          ack: intoZec
              ? l10n.walletSwapRefundVerifyAck
              : l10n.walletSwapPayoutVerifyAck,
          value: _addressAcknowledged,
          onChanged: (v) => setState(() => _addressAcknowledged = v),
        ),
        const SizedBox(height: 16),

        // The §2.6 provider disclosure + ITS acknowledgment, ADJACENT — the ack
        // ("I understand the provider will see the above") sits directly under the
        // disclosure it refers to (maintainer UX). Start stays disabled until it
        // and the address ack above are both checked.
        _SwapDisclosureCard(disclosure: quote.disclosure),
        const SizedBox(height: 12),
        // The blocking acknowledgment checkbox (§2.6): the shared
        // [WalletAckCheckbox] (a [CheckboxListTile], so the WHOLE row toggles
        // once on a tap anywhere), with Swap's sentence. Send's public-payment
        // acknowledgement is the same widget.
        WalletAckCheckbox(
          value: _acknowledged,
          onChanged: (v) => setState(() => _acknowledged = v),
          label: l10n.walletSwapAckLabel,
        ),
        const SizedBox(height: 16),

        // The RETRYABLE inline fault (#367): today only the store-busy execute
        // miss — the quote is UNCONSUMED and still valid, so the review stays
        // up and Start swap is the retry. Suppressed once the quote EXPIRED
        // (review fold): a busy that consumed the tail of the window would
        // otherwise render "try again" next to the expired message while Start
        // is disabled — the expired instruction (re-quote) is the true one.
        // liveRegion so a screen reader hears the retry guidance without
        // refocusing.
        if (!quoteExpired &&
            fault is SwapCategoricalFault &&
            fault.reason == SwapFaultReason.storeBusy) ...[
          WalletNotice(
            tone: WalletNoticeTone.warning,
            glyph: WalletGlyph.waiting,
            message: l10n.walletSwapFaultStoreBusyRetry,
            liveRegion: true,
          ),
          const SizedBox(height: 12),
        ],

        FilledButton(
          onPressed: canConfirm
              ? () => unawaited(
                  ref.read(swapControllerProvider.notifier).execute(),
                )
              : null,
          child: Text(l10n.walletSwapConfirmButton),
        ),
        const SizedBox(height: 8),
        TextButton(
          onPressed: () =>
              ref.read(swapControllerProvider.notifier).backToForm(),
          child: Text(l10n.walletSwapBackButton),
        ),
      ],
    );
  }

  /// Dispatch a quote for the current direction, reading the controllers/asset
  /// the screen owns into the matching [SwapFormInput].
  void _quote() {
    final notifier = ref.read(swapControllerProvider.notifier);
    switch (_direction) {
      case SwapFlowDirection.outOfZec:
        final target = _targetAsset;
        if (target == null) return; // defensive — Get-quote is gated on this
        unawaited(
          notifier.quote(
            OutOfZecInput(
              asset: target,
              amountText: _amountController.text,
              destination: _destinationController.text,
              slippageBps: _slippageBps,
            ),
          ),
        );
      case SwapFlowDirection.intoZec:
        final src = _sourceAsset;
        if (src == null) return; // defensive — Get-quote is gated on this
        unawaited(
          notifier.quote(
            IntoZecInput(
              token: src,
              amountText: _amountController.text,
              refundAddress: _refundController.text,
              slippageBps: _slippageBps,
            ),
          ),
        );
    }
  }
}

/// The direction flip (§3.3b L8) — IntoZec (buy ZEC, the default) ⇄ OutOfZec
/// (sell ZEC). A segmented control (ZODL parity).
class _DirectionToggle extends StatelessWidget {
  const _DirectionToggle({required this.direction, required this.onChanged});

  final SwapFlowDirection direction;
  final ValueChanged<SwapFlowDirection> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    return SegmentedButton<SwapFlowDirection>(
      segments: [
        ButtonSegment(
          value: SwapFlowDirection.intoZec,
          label: Text(l10n.walletSwapDirectionBuy),
          icon: const WalletIcon(WalletGlyph.incoming),
        ),
        ButtonSegment(
          value: SwapFlowDirection.outOfZec,
          label: Text(l10n.walletSwapDirectionSell),
          icon: const WalletIcon(WalletGlyph.outgoing),
        ),
      ],
      selected: {direction},
      onSelectionChanged: (s) => onChanged(s.first),
      showSelectedIcon: false,
    );
  }
}

/// A tappable, form-styled asset-picker tile that opens the shared D5 token
/// picker. Shows the chosen asset (icon + label) or the [hint] "select an asset"
/// cue. Direction-neutral (DRY): the IntoZec SOURCE picker and the OutOfZec
/// TARGET picker both use it, each passing its own [fieldKey] / [label] / [hint].
/// Min 48px tap target; `button: true` for screen readers.
class _AssetPickerField extends StatelessWidget {
  const _AssetPickerField({
    required this.fieldKey,
    required this.asset,
    required this.onTap,
    required this.label,
    required this.hint,
  });

  /// Stable widget key for tests (e.g. `swap-source-asset-field` /
  /// `swap-target-asset-field`).
  final Key fieldKey;
  final SwapAsset? asset;
  final Future<void> Function() onTap;

  /// The field label (e.g. "Asset to swap from" / "Receive asset").
  final String label;

  /// The empty-state cue + the accessible label when no asset is picked yet.
  final String hint;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final a = asset;
    // ONE binding, read by the InkWell and by the semantics action.
    void open() => unawaited(onTap());
    return Semantics(
      key: fieldKey,
      // `container: true` so iOS VoiceOver doesn't merge this press surface into
      // the sibling amount/address fields (flutter-patterns § iOS Semantics
      // merging) — without it a XCUITest tap retargets to a neighbouring field
      // and the picker never opens.
      container: true,
      button: true,
      label: a == null ? hint : '$label: ${a.label}',
      excludeSemantics: true,
      // …and `excludeSemantics` drops the InkWell's node, its tap action with
      // it, so the action must live on THIS node — otherwise the asset picker
      // is unreachable to a screen reader and the swap cannot be composed at
      // all (Relim `0c5ae1bd`, #661/#721).
      onTap: open,
      child: InkWell(
        onTap: open,
        // The ripple reads the field radius its InputDecorator is drawn with.
        borderRadius: BorderRadius.circular(WalletShapes.of(context).field),
        child: InputDecorator(
          decoration: InputDecoration(labelText: label),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 24),
            child: Row(
              children: [
                if (a != null) ...[
                  SwapTokenIcon(symbol: a.symbol, chain: a.chain, size: 24),
                  const SizedBox(width: 12),
                  Expanded(child: Text(a.label)),
                ] else
                  Expanded(
                    child: Text(
                      hint,
                      style: TextStyle(color: colors.textMuted),
                    ),
                  ),
                WalletIcon(WalletGlyph.dropdown, color: colors.textMuted),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// The IntoZec positive end-state line (§3.3b D1 pinned copy) — you receive ZEC to
/// your OWN address; the brief transparent leg is honest, the shield is one tap.
/// Cyan (positive), distinct from the orange OutOfZec de-shield warning.
class _IntoZecEndsShieldedCard extends StatelessWidget {
  const _IntoZecEndsShieldedCard();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Semantics(
      container: true,
      excludeSemantics: true,
      label:
          '${l10n.walletSwapIntoZecShieldTitle}. '
          '${l10n.walletSwapIntoZecEndsShielded}',
      // The line has no title, so the heading is its message and the detail
      // rides below it in `child`: every word it showed stays on screen.
      child: WalletNotice(
        tone: WalletNoticeTone.positive,
        glyph: WalletGlyph.shielded,
        message: l10n.walletSwapIntoZecShieldTitle,
        child: Text(
          l10n.walletSwapIntoZecEndsShielded,
          style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
        ),
      ),
    );
  }
}

/// The foreign-address VERIFICATION step: the full address rendered back
/// monospace/chunked for character-by-character checking, with a DISTINCT
/// acknowledgment from the privacy ack. IntoZec verifies the echoed `refundTo`
/// (§3.3b D6 / L8): a typo there loses coins on a failed swap (HARD-G — the
/// provider echo catches tampering, NOT user typos). OutOfZec verifies the payout
/// address (N03): a wrong one sends the swapped asset to someone else. The wallet
/// cannot validate a foreign address for the user, so the host owns this step.
class _AddressVerification extends StatelessWidget {
  const _AddressVerification({
    required this.address,
    required this.title,
    required this.body,
    required this.ack,
    required this.value,
    required this.onChanged,
  });

  /// The full foreign address to verify: IntoZec, the provider's echo of the
  /// refund address the SDK sent (verified equal to ours before the quote
  /// returned); OutOfZec, the payout address the quote was requested for.
  /// `null`/empty is defensive — both directions always carry one — and renders
  /// an em dash; the review then keeps Start disabled.
  final String? address;
  final String title;
  final String body;
  final String ack;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final address = this.address ?? '';

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: colors.bgCard,
        borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
        border: Border.all(color: colors.orange),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: textTheme.titleSmall?.copyWith(color: colors.text),
          ),
          const SizedBox(height: 8),
          // The full echoed address in equal-weight chunks for char-by-char
          // verification (the shared brick — see [AddressVerificationText]).
          AddressVerificationText(address),
          const SizedBox(height: 8),
          Text(
            body,
            style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
          ),
          CheckboxListTile(
            value: value,
            onChanged: (v) => onChanged(v ?? false),
            controlAffinity: ListTileControlAffinity.leading,
            contentPadding: EdgeInsets.zero,
            title: Text(ack, style: textTheme.bodyMedium),
          ),
        ],
      ),
    );
  }
}

/// Honest unavailable state (no live wallet, or swap turned off). Defensive — the
/// screen is gated; this catches a deep-link / a mid-screen wallet close.
class _SwapUnavailable extends StatelessWidget {
  const _SwapUnavailable({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              message,
              textAlign: TextAlign.center,
              style: Theme.of(
                context,
              ).textTheme.bodyMedium?.copyWith(color: colors.textMuted),
            ),
            const SizedBox(height: 16),
            OutlinedButton(
              onPressed: () => context.leaveToWalletRoot(),
              // Not "Done" (#364 F12): nothing completed on this screen — the
              // honest label is the navigation it performs.
              child: Text(l10n.walletSwapBackToWallet),
            ),
          ],
        ),
      ),
    );
  }
}

/// A neutral busy view for the transient phases (quoting / executing). One
/// coherent screen-reader node.
class _SwapBusy extends StatelessWidget {
  const _SwapBusy({required this.label, this.onStartAnother});

  final String label;

  /// The tracking LOADING arm passes this (HIGH-1): a re-attach whose
  /// first status never arrives (a GC'd order the core retries forever) would
  /// otherwise spin here with no way back to the form. Null for the momentary
  /// quote/execute busy states, which need no escape.
  final VoidCallback? onStartAnother;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final l10n = WalletLocalizations.of(context);
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Semantics(
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
                  style: textTheme.bodyMedium?.copyWith(
                    color: colors.textMuted,
                  ),
                  textAlign: TextAlign.center,
                ),
              ],
            ),
          ),
          if (onStartAnother != null) ...[
            const SizedBox(height: 20),
            TextButton(
              onPressed: onStartAnother,
              child: Text(l10n.walletSwapStartAnother),
            ),
          ],
        ],
      ),
    );
  }
}

/// The §2.6 de-shield warning — swapping out of ZEC de-shields funds and the
/// provider legs are public. Prominent (orange) and honest (design invariant 5).
class _SwapDeshieldWarning extends StatelessWidget {
  const _SwapDeshieldWarning();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    return Semantics(
      container: true,
      excludeSemantics: true,
      label: '${l10n.walletSwapDeshieldTitle}. ${l10n.walletSwapDeshieldBody}',
      child: WalletNotice.card(
        tone: WalletNoticeTone.warning,
        glyph: WalletGlyph.transparent,
        title: l10n.walletSwapDeshieldTitle,
        message: l10n.walletSwapDeshieldBody,
      ),
    );
  }
}

/// The §2.6 provider-disclosure card — renders EVERY [DisclosureItem] the SDK
/// reports (the list IS the disclosure; the UI never guesses) plus the
/// public-provider-legs note.
class _SwapDisclosureCard extends StatelessWidget {
  const _SwapDisclosureCard({required this.disclosure});

  final SwapPrivacyDisclosure disclosure;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    final lines = [
      for (final item in disclosure.providerSees) _itemMessage(l10n, item),
      // A DISTINCT fact from the de-shield warning above: `deshields` is "our
      // deposit de-shields ZEC"; `providerLegsTransparent` is "the provider's
      // own chain legs are public". Render it as its own narrow line, never the
      // conflated de-shield sentence.
      if (disclosure.providerLegsTransparent)
        l10n.walletSwapDiscloseProviderLegsPublic,
    ];

    // A group: fill, no outline (S13 §1.4).
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: colors.bgCard,
        borderRadius: BorderRadius.circular(WalletShapes.of(context).group),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l10n.walletSwapDiscloseTitle,
            style: textTheme.titleSmall?.copyWith(color: colors.text),
          ),
          const SizedBox(height: 8),
          for (final line in lines)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 3),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // What the provider sees — a privacy fact, not a secret
                  // being revealed (S13 §1.7: `reveal` keeps one meaning).
                  WalletIcon(
                    WalletGlyph.privacyNotice,
                    size: 16,
                    color: colors.textMuted,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      line,
                      style: textTheme.bodySmall?.copyWith(color: colors.text),
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }

  static String _itemMessage(WalletLocalizations l10n, DisclosureItem item) {
    return switch (item) {
      DisclosureItem.amounts => l10n.walletSwapDiscloseAmounts,
      DisclosureItem.crossAssetLink => l10n.walletSwapDiscloseCrossLink,
      DisclosureItem.destinationAddress => l10n.walletSwapDiscloseDestination,
      DisclosureItem.sourceAddress => l10n.walletSwapDiscloseSource,
      DisclosureItem.ipUnlessTor => l10n.walletSwapDiscloseIp,
      // Forward-compat arm: render a generic line, never drop it silently.
      DisclosureItem.unknown => l10n.walletSwapDiscloseGeneric,
    };
  }
}

/// One labelled review line.
class _ReviewLine extends StatelessWidget {
  const _ReviewLine({
    required this.label,
    required this.value,
    this.emphasize = false,
  });

  final String label;
  final String value;
  final bool emphasize;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final labelStyle = emphasize
        ? textTheme.titleMedium?.copyWith(color: colors.text)
        : textTheme.bodyMedium?.copyWith(color: colors.textMuted);
    final valueStyle = emphasize
        ? textTheme.titleMedium?.copyWith(
            color: colors.text,
            fontWeight: FontWeight.w600,
          )
        : textTheme.bodyMedium?.copyWith(color: colors.text);
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Flexible(child: Text(label, style: labelStyle)),
        const SizedBox(width: 12),
        Flexible(
          child: Text(value, style: valueStyle, textAlign: TextAlign.end),
        ),
      ],
    );
  }
}

/// The live swap tracking view — watches `swapStatusProvider(swapId)` once an
/// execute has minted an id. Renders "tracking unavailable" from the host's OWN
/// kill state when swap is off (§3.5 — the stream has no wire signal for a kill,
/// so the host is the authority); never reconnect-on-completion (the D-2a B1
/// contract: the notifier latches a clean end and stops).
class _SwapTrackingView extends ConsumerWidget {
  const _SwapTrackingView({
    required this.swapId,
    required this.direction,
    required this.reattached,
  });

  final String swapId;
  final SwapFlowDirection direction;

  /// True when tracking a durable-record RE-ATTACH (#367) — the IntoZec
  /// pending copy must not instruct sending funds whose deposit instructions
  /// (address/memo) were deliberately not persisted.
  final bool reattached;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l10n = WalletLocalizations.of(context);
    final swapEnabled = ref.watch(swapEnabledProvider);

    // §3.5: a host-killed swap renders tracking-unavailable from its own state —
    // and by NOT watching the status provider here, the screen drops its
    // listener. The actual poll teardown lives in `SwapStatusNotifier.build`,
    // which watches `swapEnabledProvider` and cancels its subscription when the
    // kill flips (D-2b-2) — so a killed swap stops polling even though the
    // notifier is non-autoDispose and would otherwise outlive this screen.
    if (!swapEnabled) {
      // §3.3b L8 pinned copy, direction-honest BOTH ways since #382: an IntoZec
      // kill is honest about delivery arriving on the next sync (the scoped
      // poll detects + nudges the shield), and an OutOfZec kill is honest that
      // a refund comes back to THIS wallet — the pre-#382 generic body sent
      // the user to "the provider's side", a misdirect once #368 made refunds
      // land here (the copy MED; detection resumes via the unresolved
      // re-arm once swap is back on). Neither reads as fund-loss.
      return _TrackingResult(
        icon: WalletGlyph.hidden,
        tintToken: _Tint.muted,
        title: l10n.walletSwapTrackingUnavailableTitle,
        body: direction == SwapFlowDirection.intoZec
            ? l10n.walletSwapTrackingUnavailableBodyIntoZec
            : l10n.walletSwapTrackingUnavailableBodyOutOfZec,
      );
    }

    // THE ESCAPE (money HIGH-1 ≡ UX H-UX-1, converged): every NON-terminal
    // tracking arm offers "Start another swap" back to the form. Without it a
    // re-attached swap that never terminates — a provider-GC'd order the poll
    // retries forever (SwapNotFound), or a window-passed PendingDeposit — wedged
    // the whole feature: resetToForm no-ops on a live committed state, the
    // direction toggle routes through it, and startNewSwap was reachable only
    // from a terminal/error Done. Money-safe: startNewSwap only resets the
    // CONTROLLER to the form — it never cancels the swap or dismisses the record,
    // so the swap stays listed on the wallet screen (one tap back via "View
    // swap"). Terminal arms keep their own Done (which clears + leaves).
    void startAnother() {
      ref.read(swapControllerProvider.notifier).startNewSwap();
      context.leaveToWalletRoot();
    }

    final status = ref.watch(swapStatusProvider(swapId));
    // #385 (MED-2ux): the record's PINNED outcome, if any — the card
    // polls live provider truth, but a chain-observed Refunded pin lands
    // without the card ever opening, and the provider may then GC the order.
    // Rendering "Swap not found — most likely expired" over a home row that
    // says "Swap refunded." was a cross-surface contradiction; when the
    // provider answers NotFound and the record carries a pinned outcome, the
    // pinned outcome (a confirmed terminal) renders instead. Identity-fenced
    // view (#381), so the lookup can never read another wallet's records.
    // `.select` so only THIS record's outcome (never every list refresh)
    // rebuilds the card; a loading/absent list degrades to null (the honest
    // heuristic card).
    final pinnedOutcome = ref.watch(
      walletInFlightSwapsProvider.select((list) {
        for (final record in list.value ?? const <SwapRecord>[]) {
          if (record.id == swapId) return record.outcome;
        }
        return null;
      }),
    );
    return status.when(
      data: (s) {
        // #366-e: PendingDeposit renders the deposit window its own copy
        // references — `expires_at` was already in the DTO, ignored pre-#366,
        // so a dead quote could show "Swap started" FOREVER. A static stamp
        // (re-evaluated on every status emission) + an honest window-passed
        // variant; the live-countdown/actionable-clamp alignment is #367.
        String? pendingDetail;
        if (s is SwapStatus_PendingDeposit) {
          final nowUnix = DateTime.now().millisecondsSinceEpoch ~/ 1000;
          if (nowUnix >= s.expiresAt) {
            pendingDetail = direction == SwapFlowDirection.outOfZec
                ? l10n.walletSwapPendingWindowPassedOutOfZec
                : l10n.walletSwapPendingWindowPassedIntoZec;
          } else {
            final time = walletCompactTimeFormat(l10n.localeName).format(
              DateTime.fromMillisecondsSinceEpoch(s.expiresAt * 1000).toLocal(),
            );
            pendingDetail = l10n.walletSwapPendingWindowEndsAt(time);
          }
        }
        return _statusResult(
          l10n,
          s,
          direction,
          pendingDetail: pendingDetail,
          pinnedOutcome: pinnedOutcome,
          onStartAnother: startAnother,
          // A TERMINAL Done leaves tracking FOR GOOD (W-swap-5): the swap is
          // over, so clear the live state — without this the home/self
          // re-entry would re-attach a finished swap forever. The idempotent
          // record dismiss rides the tap as the BELT to the stream latch's
          // dismiss (arch review N2: if the latch's fire-and-forget write was
          // swallowed — a busy store — Done is the natural retry; a double
          // dismiss is Ok(false) by contract). The home invalidate rides the
          // NOTIFIER's container-lifetime ref: a widget
          // ref gated on `context.mounted` was ALWAYS false here — Done
          // navigates away first — so the belt's refresh never ran; the
          // notifier ref outlives the pop.
          onTerminalDone: () {
            final session = ref.read(walletSessionProvider);
            final container = ProviderScope.containerOf(context, listen: false);
            if (session != null) {
              unawaited(
                session
                    .dismissSwapRecord(swapId: swapId)
                    .then(
                      (_) =>
                          container.invalidate(walletInFlightSwapsReadProvider),
                    )
                    .catchError((Object _) {
                      // Swallowed: the row self-lapses at its own bound.
                    }),
              );
            }
            ref.read(swapControllerProvider.notifier).startNewSwap();
            context.leaveToWalletRoot();
          },
          // #385 (MED-1ux): the not-found card's Done clears the live
          // tracking state and leaves WITHOUT dismissing the still-unresolved
          // record — the row keeps listing + watching, and removal stays with
          // the home row's disclosure dialog.
          onNotFoundDone: () {
            ref.read(swapControllerProvider.notifier).startNewSwap();
            context.leaveToWalletRoot();
          },
        );
      },
      // The genuinely-COLD attach (no carried state → bare loading; a retention
      // rebuild routes to data: instead). "Swap started" would over-claim a
      // status we have not heard back on yet — stay neutral (#347).
      loading: () => _SwapBusy(
        label: l10n.walletSwapStatusCheckingTitle,
        onStartAnother: startAnother,
      ),
      // An establish-time typed failure (SwapDisabled / bad id) — the core never
      // routes a transient fault here (it retries internally), so this is
      // persistent: surface it honestly, never a spinner-forever. Done gets the
      // SAME startNewSwap escape as a terminal (arch review M-A2): the widened
      // resetToForm guard means a bare leave would re-attach this dead-end card
      // on every re-entry for the rest of the process run — the user could
      // never start a new swap again. The durable record is NOT dismissed here
      // (the outcome was never observed; the row self-lapses or re-attaches).
      error: (_, _) => _TrackingResult(
        icon: WalletGlyph.error,
        tintToken: _Tint.orange,
        title: l10n.walletSwapTrackingError,
        // Its OWN body (review M1): the failed-status body ("The swap
        // couldn't be completed") contradicted this card's title — a tracking
        // ESTABLISH failure says nothing about the swap, which may be
        // proceeding fine; the copy must not read a failure verdict into it.
        body: l10n.walletSwapTrackingErrorBody,
        onDone: () {
          ref.read(swapControllerProvider.notifier).startNewSwap();
          context.leaveToWalletRoot();
        },
      ),
    );
  }

  /// Renders one pinned outcome as its terminal card — shared by the live
  /// status arms below and the #385 pinned-over-NotFound delegate, so the two
  /// paths can never drift apart.
  _TrackingResult _outcomeResult(
    WalletLocalizations l10n,
    SwapOutcome outcome,
    SwapFlowDirection direction,
    VoidCallback? onTerminalDone,
  ) {
    return switch (outcome) {
      SwapOutcome.success => _TrackingResult(
        icon: WalletGlyph.success,
        tintToken: _Tint.green,
        title: l10n.walletSwapStatusSuccessTitle,
        body: l10n.walletSwapStatusSuccessBody,
        onDone: onTerminalDone,
        terminal: true,
      ),
      // Refunded honesty, direction-aware (#367; the OutOfZec body upgraded at
      // #368): an OutOfZec refund returns to a wallet transparent address the
      // SDK now WATCHES (refund-index registration + the scoped poll — and
      // observing this terminal pins the outcome, which re-arms the watch), so
      // the copy promises the refund lands in the balance as unshielded funds
      // after a sync, hedged on timing (the provider's refund tx must mine
      // first). An IntoZec refund goes to the USER's refund address on the
      // source chain — this wallet never sees it, and the copy says where to
      // look.
      SwapOutcome.refunded => _TrackingResult(
        icon: WalletGlyph.refunded,
        tintToken: _Tint.orange,
        title: l10n.walletSwapStatusRefundedTitle,
        body: direction == SwapFlowDirection.outOfZec
            ? l10n.walletSwapStatusRefundedBodyOutOfZec
            : l10n.walletSwapStatusRefundedBody,
        onDone: onTerminalDone,
        terminal: true,
      ),
      SwapOutcome.failed => _TrackingResult(
        icon: WalletGlyph.error,
        tintToken: _Tint.red,
        title: l10n.walletSwapStatusFailedTitle,
        body: l10n.walletSwapStatusFailedBody,
        onDone: onTerminalDone,
        terminal: true,
      ),
    };
  }

  _TrackingResult _statusResult(
    WalletLocalizations l10n,
    SwapStatus s,
    SwapFlowDirection direction, {
    String? pendingDetail,
    SwapOutcome? pinnedOutcome,
    VoidCallback? onTerminalDone,
    VoidCallback? onNotFoundDone,
    VoidCallback? onStartAnother,
  }) {
    return switch (s) {
      // DIRECTION-AWARE (UX S4 + reliability S3b): who sends the deposit
      // differs per direction, and the copy must not lie on a money screen.
      // OutOfZec: the WALLET sends the ZEC deposit — and post-FR-23-a the
      // broadcast can trail the execute (offline/flaky network waits for the
      // outbox), so the body is honest that sending is automatic and may take
      // a moment. IntoZec: the USER sends a FOREIGN coin from their own wallet
      // — "your ZEC deposit is on its way" was factually false here (and
      // IntoZec is the DEFAULT direction).
      SwapStatus_PendingDeposit() => _TrackingResult(
        icon: WalletGlyph.awaitingDeposit,
        tintToken: _Tint.cyan,
        title: l10n.walletSwapStatusPendingTitle,
        // #367: a RE-ATTACHED IntoZec pending must not say "send them before
        // the quote expires" — the deposit instructions (address/memo) are
        // deliberately not persisted, so post-restart that action is
        // impossible to follow (and the address must NOT be re-rendered: the
        // memo is gone, and address-without-memo is a memo-chain fund-loss
        // door). The re-attach copy is honest about both arms (sent = it will
        // be detected; never sent = let it expire, start a new swap).
        body: direction == SwapFlowDirection.outOfZec
            ? l10n.walletSwapStatusPendingBodyOutOfZec
            : (reattached
                  ? l10n.walletSwapStatusPendingBodyIntoZecReattached
                  : l10n.walletSwapStatusPendingBodyIntoZec),
        detail: pendingDetail,
        onStartAnother: onStartAnother,
      ),
      // #367: the DTO's received/missing/deadline were carried and never
      // rendered — the user was told "part has arrived" with no numbers and
      // no clock. Amounts are the provider's decimal strings in the DEPOSIT
      // asset's units (verbatim — no host-side arithmetic on money strings).
      SwapStatus_UnderDeposited(
        :final received,
        :final missing,
        :final deadline,
      ) =>
        _TrackingResult(
          icon: WalletGlyph.expiring,
          tintToken: _Tint.orange,
          title: l10n.walletSwapStatusUnderTitle,
          body: direction == SwapFlowDirection.outOfZec
              ? l10n.walletSwapStatusUnderBody
              : l10n.walletSwapStatusUnderBodyIntoZec,
          detail: l10n.walletSwapStatusUnderDetail(
            received,
            missing,
            walletCompactTimeFormat(l10n.localeName).format(
              DateTime.fromMillisecondsSinceEpoch(deadline * 1000).toLocal(),
            ),
          ),
          onStartAnother: onStartAnother,
        ),
      SwapStatus_DepositDetected() => _TrackingResult(
        icon: WalletGlyph.depositDetected,
        tintToken: _Tint.cyan,
        title: l10n.walletSwapStatusDetectedTitle,
        body: l10n.walletSwapStatusDetectedBody,
        onStartAnother: onStartAnother,
      ),
      SwapStatus_Processing() => _TrackingResult(
        icon: WalletGlyph.syncing,
        tintToken: _Tint.accent,
        title: l10n.walletSwapStatusProcessingTitle,
        body: l10n.walletSwapStatusProcessingBody,
        onStartAnother: onStartAnother,
      ),
      SwapStatus_Success() => _outcomeResult(
        l10n,
        SwapOutcome.success,
        direction,
        onTerminalDone,
      ),
      SwapStatus_Refunded() => _outcomeResult(
        l10n,
        SwapOutcome.refunded,
        direction,
        onTerminalDone,
      ),
      // #367: the SDK's poll policy synthesizes Failed(notFound) after
      // several consecutive provider not-founds (a GC'd order). Its own copy:
      // "failed" would over-claim (the provider can no longer tell us
      // anything), and a landed deposit still refunds provider-side — the
      // body must not assert loss.
      // #385 (MED-2ux): a record whose outcome is already PINNED — the
      // chain-observed Refunded pin is the canonical shape (it lands without
      // the card ever opening, and the provider then GC's the order) — is a
      // CONFIRMED terminal that outranks the synthesized heuristic: render it
      // instead of contradicting the home row's outcome line. Its Done
      // dismisses normally (the outcome rendered full-screen = seen).
      SwapStatus_Failed(:final code)
          when code == SwapFailureCode.notFound && pinnedOutcome != null =>
        _outcomeResult(l10n, pinnedOutcome, direction, onTerminalDone),
      // #385 (MED-1ux): the synthesized not-found is a HEURISTIC and
      // is deliberately never pinned (see `_outcomeOf`) — the record under
      // this card is still UNRESOLVED, so Done must NOT dismiss it: the
      // silent dismiss voided the watch the overdue row had just promised,
      // with none of the disclosure the home row's Remove gives the SAME
      // record. Done here only clears the live tracking state and leaves;
      // the row stays listed + watched (the body copy says so), and removal
      // stays with the home row's disclosure dialog.
      SwapStatus_Failed(:final code) when code == SwapFailureCode.notFound =>
        _TrackingResult(
          icon: WalletGlyph.notFound,
          tintToken: _Tint.orange,
          title: l10n.walletSwapStatusNotFoundTitle,
          body: l10n.walletSwapStatusNotFoundBody,
          onDone: onNotFoundDone,
        ),
      SwapStatus_Failed() => _outcomeResult(
        l10n,
        SwapOutcome.failed,
        direction,
        onTerminalDone,
      ),
      // Forward-compat arm: neutral, never alarming (spec §3.3 unknown handling).
      SwapStatus_Unknown() => _TrackingResult(
        icon: WalletGlyph.unknown,
        tintToken: _Tint.muted,
        title: l10n.walletSwapStatusUnknownTitle,
        body: l10n.walletSwapStatusUnknownBody,
        onStartAnother: onStartAnother,
      ),
    };
  }
}

/// Which semantic color a tracking result paints in (resolved against the theme
/// inside the widget — a pure enum keeps `_statusResult` context-free + testable).
enum _Tint { muted, cyan, accent, green, orange, red }

/// A terminal-ish tracking result card (icon + title + body + Done).
/// [detail] (W-swap-5 #366-e) is an optional muted line under the body — the
/// deposit-window stamp / window-passed honesty on the pending arm. [onDone]
/// overrides the Done tap for TERMINAL arms (clear the live state, then
/// leave); null keeps the plain leave (a non-terminal Done is just "go back —
/// the swap keeps tracking"). [onStartAnother] (HIGH-1) adds a quiet
/// secondary "Start another swap" escape under Done on the NON-terminal arms —
/// the only way back to the form while a committed swap is live, so a swap
/// stuck non-terminal (a GC'd order polled forever, a window-passed pending)
/// can never brick the feature. It never cancels the swap (the record stays
/// listed on the wallet screen).
class _TrackingResult extends StatelessWidget {
  const _TrackingResult({
    required this.icon,
    required this.tintToken,
    required this.title,
    required this.body,
    this.detail,
    this.onDone,
    this.onStartAnother,
    this.terminal = false,
  });

  final WalletGlyph icon;
  final _Tint tintToken;
  final String title;
  final String body;
  final String? detail;
  final VoidCallback? onDone;
  final VoidCallback? onStartAnother;

  /// True ONLY for a genuinely finished swap (success / refunded / failed) —
  /// its primary button says "Done". Everything still in motion or unresolved
  /// (pending, detected, processing, unknown, the not-found heuristic, the
  /// establish-failure card) says "Back to wallet" instead (#364 F12): "Done"
  /// on a live swap reads as "the swap is done", and users left the screen
  /// believing a mid-flight swap had completed.
  final bool terminal;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final tint = switch (tintToken) {
      _Tint.muted => colors.textMuted,
      _Tint.cyan => colors.cyan,
      _Tint.accent => colors.accent,
      _Tint.green => colors.green,
      _Tint.orange => colors.orange,
      _Tint.red => colors.red,
    };
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // A11y (#364 F8): one merged live node over the status content, so
            // a screen reader hears each transition (pending → detected →
            // processing → terminal) as it lands — the visual card swap was
            // silent. The buttons stay OUTSIDE the region (their own nodes);
            // the icon is decorative.
            Semantics(
              container: true,
              liveRegion: true,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  ExcludeSemantics(
                    child: WalletIcon(icon, size: 48, color: tint),
                  ),
                  const SizedBox(height: 16),
                  Text(
                    title,
                    style: Theme.of(context).textTheme.headlineSmall,
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 12),
                  Text(
                    body,
                    style: Theme.of(
                      context,
                    ).textTheme.bodyMedium?.copyWith(color: colors.textMuted),
                    textAlign: TextAlign.center,
                  ),
                  if (detail != null) ...[
                    const SizedBox(height: 8),
                    Text(
                      detail!,
                      style: Theme.of(
                        context,
                      ).textTheme.bodySmall?.copyWith(color: colors.textMuted),
                      textAlign: TextAlign.center,
                    ),
                  ],
                ],
              ),
            ),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: onDone ?? () => context.leaveToWalletRoot(),
              child: Text(
                terminal ? l10n.walletSwapDone : l10n.walletSwapBackToWallet,
              ),
            ),
            if (onStartAnother != null) ...[
              const SizedBox(height: 4),
              TextButton(
                onPressed: onStartAnother,
                child: Text(l10n.walletSwapStartAnother),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

/// Render a [SwapFormFault] as an honest inline message (never a raw code; design
/// invariant 6). Pure mapping to a plain-language line. [onViewSwap] (W-swap-5,
/// #366) adds a "View swap" action under the message — supplied only for the
/// already-in-progress guard fault, where the honest remedy is to LOOK at the
/// tracked swap, never to re-quote.
class _FormFault extends StatelessWidget {
  const _FormFault({super.key, required this.fault, this.onViewSwap});

  final SwapFormFault fault;

  /// Re-attach to the guard's in-flight swap ([SwapFaultReason.swapInFlight]
  /// only); null renders the plain copy-only fault.
  final VoidCallback? onViewSwap;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);

    // A sealed switch (compile-time exhaustiveness) — a future SwapFormFault
    // subtype is a compile error here, never a runtime cast throw.
    final message = switch (fault) {
      SwapAmountFault(:final fault) => _amountFaultMessage(l10n, fault),
      SwapCategoricalFault(:final reason) => _categoricalFaultMessage(
        l10n,
        reason,
      ),
      // FR-23: the alpha ceiling bounds the deposit — stated honestly as an app
      // restriction (the amount itself is valid). Its OWN key since #364 S6:
      // the send form's copy says "limits sends", which misreads on a swap
      // form (the user isn't on a send).
      SwapOverCeiling(:final ceilingZat) => l10n.walletSwapFaultOverCeiling(
        formatZec(ceilingZat),
      ),
      // #367 spendable pre-check: the deposit (plus the fee allowance) exceeds
      // what is spendable right now — honest amounts (render-only, §5.4), with
      // a catching-up hedge instead of a verdict over a partial figure.
      SwapInsufficientSpendable(
        :final neededZat,
        :final spendableZat,
        :final catchingUp,
      ) =>
        catchingUp
            ? l10n.walletSwapFaultInsufficientCatchingUp(
                formatZec(neededZat),
                formatZec(spendableZat),
              )
            : l10n.walletSwapFaultInsufficient(
                formatZec(neededZat),
                formatZec(spendableZat),
              ),
    };

    // A11y (#364 S5, send-form parity): announce the fault the moment it
    // appears — it renders at the foot of a scrollable form, so a screen-reader
    // user otherwise gets no "why is Get quote doing nothing". The icon is
    // decorative; the words carry the meaning. The sighted counterpart is the
    // screen's scroll-to-fault listener.
    final onViewSwap = this.onViewSwap;
    return WalletNotice(
      tone: WalletNoticeTone.warning,
      glyph: WalletGlyph.error,
      message: message,
      liveRegion: true,
      actions: [
        if (onViewSwap != null)
          TextButton(
            onPressed: onViewSwap,
            child: Text(l10n.walletSwapViewSwap),
          ),
      ],
    );
  }

  static String _amountFaultMessage(
    WalletLocalizations l10n,
    ZecAmountFault f,
  ) {
    // Reuse the send screen's amount-parse copy (the parser is shared).
    return switch (f) {
      ZecAmountFault.empty => l10n.walletSendFaultAmountEmpty,
      ZecAmountFault.notANumber => l10n.walletSendFaultAmountNotANumber,
      ZecAmountFault.tooManyDecimals => l10n.walletSendFaultAmountDecimals,
      ZecAmountFault.notPositive => l10n.walletSendFaultAmountNotPositive,
      ZecAmountFault.outOfRange => l10n.walletSendFaultAmountOutOfRange,
    };
  }

  static String _categoricalFaultMessage(
    WalletLocalizations l10n,
    SwapFaultReason reason,
  ) {
    return switch (reason) {
      SwapFaultReason.destinationRequired =>
        l10n.walletSwapFaultDestinationRequired,
      SwapFaultReason.foreignAmountRequired =>
        l10n.walletSwapFaultForeignAmountRequired,
      SwapFaultReason.refundAddressRequired =>
        l10n.walletSwapFaultRefundAddressRequired,
      SwapFaultReason.destinationInvalid =>
        l10n.walletSwapFaultDestinationInvalid,
      SwapFaultReason.quoteExpired => l10n.walletSwapFaultExpired,
      SwapFaultReason.quoteOutOfBounds => l10n.walletSwapFaultOutOfBounds,
      SwapFaultReason.slippageTooHigh => l10n.walletSwapFaultSlippageTooHigh,
      SwapFaultReason.providerUnavailable =>
        l10n.walletSwapFaultProviderUnavailable,
      SwapFaultReason.connectionFailed => l10n.walletSwapFaultConnection,
      SwapFaultReason.executeTimedOut => l10n.walletSwapFaultExecuteTimeout,
      SwapFaultReason.providerMisbehaved =>
        l10n.walletSwapFaultProviderMisbehaved,
      SwapFaultReason.swapOff => l10n.walletSwapFaultSwapOff,
      SwapFaultReason.depositFailed => l10n.walletSwapFaultDepositFailed,
      // NOT a re-quote invitation (W-swap-4-a-2): the copy points at the
      // in-progress swap; re-quoting is the double-deposit door.
      SwapFaultReason.swapInFlight => l10n.walletSwapFaultAlreadyInFlight,
      SwapFaultReason.refundAddressUnavailable =>
        l10n.walletSwapFaultRefundUnavailable,
      SwapFaultReason.destinationAddressUnavailable =>
        l10n.walletSwapFaultDestinationUnavailable,
      SwapFaultReason.swapStateUnavailable =>
        l10n.walletSwapFaultStateUnavailable,
      // #367: the retryable busy — "try again", the same action re-run works
      // (nothing was consumed); distinct from the state-unavailable dead end.
      SwapFaultReason.storeBusy => l10n.walletSwapFaultStoreBusyRetry,
      SwapFaultReason.requestInvalid => l10n.walletSwapFaultRequestInvalid,
      SwapFaultReason.quoteTermsDiffer => l10n.walletSwapFaultTermsDiffer,
      SwapFaultReason.couldNotQuote => l10n.walletSwapFaultCouldNotQuote,
      SwapFaultReason.walletUnavailable =>
        l10n.walletSwapFaultWalletUnavailable,
    };
  }
}

/// A live countdown to the quote's display deadline on the review screen. Display
/// ONLY — the SDK gates the REAL deadline (monotonic + wall clocks) at execute, so
/// this never decides money; it is the honest UI that tells the user how long they
/// have and, on expiry, that they must re-quote (sending to an expired quote's
/// address risks a provider refund). Self-ticking (1s); cancels at zero. Mirrors
/// the deposit screen's countdown and reuses its `formatCountdown`.
class _QuoteCountdown extends StatefulWidget {
  const _QuoteCountdown({required this.expiresAt, required this.onExpired});

  /// Unix seconds (display-only). A fresh quote (new value) restarts the clock.
  final int expiresAt;

  /// Called ONCE when the countdown crosses zero so the parent disables Start.
  final VoidCallback onExpired;

  @override
  State<_QuoteCountdown> createState() => _QuoteCountdownState();
}

class _QuoteCountdownState extends State<_QuoteCountdown> {
  Timer? _ticker;
  late int _remaining;
  bool _firedExpired = false;

  @override
  void initState() {
    super.initState();
    _start();
  }

  @override
  void didUpdateWidget(_QuoteCountdown old) {
    super.didUpdateWidget(old);
    if (old.expiresAt != widget.expiresAt) {
      _firedExpired = false;
      _start();
    }
  }

  void _start() {
    _ticker?.cancel();
    _ticker = null;
    _remaining = _secondsLeft();
    if (_remaining <= 0) {
      _notifyExpired();
      return;
    }
    _ticker = Timer.periodic(const Duration(seconds: 1), (_) {
      final left = _secondsLeft();
      if (!mounted) return;
      setState(() => _remaining = left);
      if (left <= 0) {
        _ticker?.cancel();
        _ticker = null;
        _notifyExpired();
      }
    });
  }

  void _notifyExpired() {
    if (_firedExpired) return;
    _firedExpired = true;
    // Out of the build/timer frame — the parent does setState on this.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) widget.onExpired();
    });
  }

  int _secondsLeft() {
    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    return widget.expiresAt - now;
  }

  @override
  void dispose() {
    _ticker?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final expired = _remaining <= 0;
    return Semantics(
      // liveRegion so a screen reader announces the expiry flip (an
      // honest-degradation state the user must not miss). The label is
      // COARSE-granular (#364 F9): with the per-second text inside the live
      // region, TalkBack re-announced the countdown EVERY second for the whole
      // review life — a 1 Hz announcement storm. The explicit label changes at
      // most once a minute (and once at the sub-minute flip + once at expiry),
      // while the visual text below keeps its own cadence.
      liveRegion: true,
      container: true,
      // Under a minute the label switches to a DEDICATED sentence (
      // review M4): composing "less than a minute" into the {time} slot of
      // "Quote valid for about {time}" double-hedged ("about less than a
      // minute") in every locale, at the most time-critical spoken moment.
      label: expired
          ? l10n.walletSwapQuoteExpired
          : (_remaining >= 60
                ? l10n.walletSwapQuoteExpiresIn(
                    localizedCountdown(l10n, _remaining),
                  )
                : l10n.walletSwapQuoteExpiresUnderMinute),
      excludeSemantics: true,
      child: WalletNotice(
        tone: expired ? WalletNoticeTone.warning : WalletNoticeTone.info,
        glyph: expired ? WalletGlyph.timerExpired : WalletGlyph.timer,
        message: expired
            ? l10n.walletSwapQuoteExpired
            : l10n.walletSwapQuoteExpiresIn(
                localizedCountdown(l10n, _remaining),
              ),
      ),
    );
  }
}
