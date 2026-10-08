import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import '../../../core/theme/shapes.dart';
import '../../../core/theme/typography.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/address_text.dart';
import '../../../shared/qr_tile.dart';
import '../../../shared/wallet_dialog.dart';
import '../../../shared/wallet_notice.dart';
import 'swap_assets.dart';
import 'swap_token_picker.dart' show swapTokenLabel;

/// The IntoZec "send your payment" deposit screen (spec §3.3b D7 / L7).
///
/// IntoZec's money flow is the OPPOSITE of OutOfZec: the WALLET sends nothing —
/// the USER sends the source coin to the provider's [SwapQuote.depositAddress].
/// So after `execute()` this dedicated screen shows the deposit address (QR +
/// copy), the EXACT "send {amountIn} {symbol} on {chain}" instruction, the
/// deadline countdown, an "I've sent the funds" affordance, and a back-press
/// guard ("leaving won't cancel your in-flight swap"). Then live tracking starts.
///
/// §5.4: the deposit address IS the §5.4 never-log SwapId — display/QR/copy only,
/// NEVER logged. Share is an additive follow-up (like the QR scanner, IZ-4):
/// copy is the essential affordance and ships first.
class SwapDepositView extends StatefulWidget {
  const SwapDepositView({
    super.key,
    required this.quote,
    required this.counter,
    required this.onConfirmSent,
    required this.onLeave,
    required this.onStartNew,
  });

  /// The executed quote — carries the deposit address, the exact amount-in, and
  /// the `expiresAt` deadline.
  final SwapQuote quote;

  /// The foreign SOURCE asset (chain/symbol/label) — the coin the user sends.
  final SwapAsset counter;

  /// "I've sent the funds" → advance to live tracking (sends nothing; the swap
  /// was already registered at execute).
  final VoidCallback onConfirmSent;

  /// Confirmed system-back → leave the swap screen (the swap continues; the
  /// scoped poll detects delivery and nudges the shield).
  final VoidCallback onLeave;

  /// EXPIRED-window escape (review HIGH): once the deadline passes, the
  /// only primary ("I've sent the funds") disables while the live
  /// [SwapAwaitingDeposit] stays latched for the wallet-identity lifetime
  /// (entry `resetToForm` no-ops on it BY DESIGN — money-in-motion re-attach,
  /// W-swap-5) — so without this the expired notice's own instruction ("start
  /// a new swap") had no button anywhere and the swap feature was WEDGED until
  /// process death (never, on desktop). Clears the live state back to the form
  /// the durable record stays home-listed.
  final VoidCallback onStartNew;

  @override
  State<SwapDepositView> createState() => _SwapDepositViewState();
}

class _SwapDepositViewState extends State<SwapDepositView> {
  Timer? _ticker;

  /// Seconds remaining until the deposit deadline; recomputed each tick.
  late int _remaining;

  @override
  void initState() {
    super.initState();
    _remaining = _secondsLeft();
    // A 1s live countdown. Cancelled in dispose; self-cancels at expiry so it
    // never ticks past zero (and a near-deadline test can settle).
    _ticker = Timer.periodic(const Duration(seconds: 1), (_) {
      final left = _secondsLeft();
      if (!mounted) return;
      setState(() => _remaining = left);
      if (left <= 0) {
        _ticker?.cancel();
        _ticker = null;
      }
    });
  }

  @override
  void dispose() {
    _ticker?.cancel();
    super.dispose();
  }

  int _secondsLeft() {
    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    return widget.quote.expiresAt - now;
  }

  Future<void> _confirmLeave() async {
    final l10n = WalletLocalizations.of(context);
    // Expiry-aware body (#364 F11): the live-window copy says "you'll need the
    // deposit address to pay, so copy it first" — an INVITATION to pay. Once
    // the window has closed, paying is exactly what the user must NOT do (a
    // late deposit only refunds provider-side), so the expired variant warns
    // off sending instead. Captured at dialog OPEN: a window that expires
    // while the dialog sits open still showed the honest-at-open copy, and the
    // deposit screen behind it flips to its own expired notice.
    final expired = _remaining <= 0;
    // Neutral (stage S11 C3): two equal choices — the swap continues either way.
    final leave = await showWalletConfirm(
      context,
      title: l10n.walletSwapDepositBackTitle,
      body: expired
          ? l10n.walletSwapDepositBackBodyExpired
          : l10n.walletSwapDepositBackBody,
      cancelLabel: l10n.walletSwapDepositBackStay,
      confirmLabel: l10n.walletSwapDepositBackLeave,
      kind: WalletConfirmKind.neutral,
    );
    // The dialog can outlive this State (a session flip swaps the body while
    // it sits on the navigator) — the project's after-await pattern.
    if (!mounted) return;
    if (leave) widget.onLeave();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    final asset = swapTokenLabel(
      l10n,
      widget.counter.symbol,
      widget.counter.chain,
    );
    final expired = _remaining <= 0;

    // ONE binding for the address copy: the button's `onPressed` and the
    // semantics action below both read it, so a screen reader's double-tap and
    // a finger run the same code.
    Future<void> copyAddress() async {
      await Clipboard.setData(ClipboardData(text: widget.quote.depositAddress));
      unawaited(HapticFeedback.lightImpact());
      if (!context.mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.walletSwapDepositCopied)));
    }

    // S13 §1.4 — Copy amount writes EXACTLY the quote's amount-in, the
    // provider's own decimal string: the deposit must match it to the digit,
    // and a re-formatted figure is how it would stop matching.
    Future<void> copyAmount() async {
      await Clipboard.setData(ClipboardData(text: widget.quote.amountIn));
      unawaited(HapticFeedback.lightImpact());
      if (!context.mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.walletSwapDepositAmountCopied)),
      );
    }

    return PopScope(
      // The deposit instructions are shown once — guard an accidental back so the
      // user doesn't lose them mid-payment. Confirmed leaving does NOT cancel the
      // swap (it continues server-side; the scoped poll detects delivery).
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (didPop) return;
        unawaited(_confirmLeave());
      },
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(l10n.walletSwapDepositTitle, style: textTheme.headlineSmall),
          const SizedBox(height: 8),
          Text(
            l10n.walletSwapDepositInstruction(
              widget.quote.amountIn,
              asset,
              widget.counter.chain.toUpperCase(),
            ),
            style: textTheme.bodyMedium?.copyWith(color: colors.text),
          ),
          const SizedBox(height: 8),
          // A cost, so it stays on screen (S13 §1.4).
          Text(
            l10n.walletSwapDepositExactNote,
            style: textTheme.bodySmall?.copyWith(color: colors.textMuted),
          ),
          const SizedBox(height: 8),
          Align(
            alignment: AlignmentDirectional.centerStart,
            child: FilledButton.tonalIcon(
              key: const Key('swap-deposit-copy-amount'),
              icon: const WalletIcon(WalletGlyph.copy),
              label: Text(l10n.walletSwapDepositCopyAmount),
              onPressed: () => unawaited(copyAmount()),
            ),
          ),
          const SizedBox(height: 20),

          // The deadline countdown — honest about the window. Expired flips to a
          // clear "don't send / already-sent-is-refunded" message (§4.4).
          _DepositCountdown(remaining: _remaining),
          const SizedBox(height: 20),

          Center(
            child: QrTile(
              payload: widget.quote.depositAddress,
              tileKey: const Key('swap-deposit-qr-tile'),
              label: l10n.walletSwapDepositQrLabel,
              // The size is QrTile's own token (S13 §1.4), not a literal here.
            ),
          ),
          const SizedBox(height: 20),

          Text(
            l10n.walletSwapDepositAddressLabel,
            style: textTheme.titleSmall?.copyWith(color: colors.text),
          ),
          const SizedBox(height: 8),
          // A group (fill, the group radius), not a raw Material Card (S13 §1.4).
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: colors.bgCard,
              borderRadius: BorderRadius.circular(
                WalletShapes.of(context).group,
              ),
            ),
            // Bold first/last chars, muted middle (shared AddressText) — same
            // verification treatment as the receive address.
            child: AddressText(widget.quote.depositAddress),
          ),
          const SizedBox(height: 12),
          // `container: true` + excludeSemantics so iOS VoiceOver doesn't merge the
          // copy button into the address text above (flutter-patterns § iOS merge).
          Semantics(
            container: true,
            button: true,
            excludeSemantics: true,
            label: l10n.walletSwapDepositCopy,
            // `excludeSemantics` drops the child subtree and the button's own
            // tap action with it, so the action must live on THIS node. The
            // deposit address is the one string that has to reach the payer;
            // a button a screen reader cannot activate loses the swap
            // (Relim `0c5ae1bd`, #661/#721).
            onTap: () => unawaited(copyAddress()),
            // Tonal (S13 §1.4): the copy is the essential action, below the
            // one primary ("I've sent the funds").
            child: FilledButton.tonalIcon(
              icon: const WalletIcon(WalletGlyph.copy),
              label: Text(l10n.walletSwapDepositCopy),
              onPressed: () => unawaited(copyAddress()),
            ),
          ),

          // Some source chains (an XRP destination tag, a Cosmos/Stellar/EOS memo)
          // REQUIRE a memo on the deposit — omit it and the funds are lost. Shown
          // ONLY when the provider quote carries one (most chains, and every ZEC
          // deposit, carry none). The user attaches it to their EXTERNAL deposit
          // (§4.4), so the wallet's job is to surface it un-missably.
          if ((widget.quote.depositMemo ?? '').isNotEmpty) ...[
            const SizedBox(height: 20),
            _DepositMemo(memo: widget.quote.depositMemo!),
          ],
          const SizedBox(height: 24),

          // The hand-off to live tracking. Disabled once the window has expired
          // (sending past the deadline would be refunded — don't encourage it).
          FilledButton(
            onPressed: expired ? null : widget.onConfirmSent,
            child: Text(l10n.walletSwapDepositSent),
          ),
          // The expired-window escape (see [SwapDepositView.onStartNew]): the
          // notice above says "start a new swap" — this IS that button, and it
          // genuinely resets to the form (no navigation), unlike the tracking
          // cards' exit-to-wallet escape. Rendered only once expired: while
          // the window is live, leaving is the deliberate dialog-guarded path.
          if (expired) ...[
            const SizedBox(height: 8),
            TextButton(
              onPressed: widget.onStartNew,
              child: Text(l10n.walletSwapStartAnother),
            ),
          ],
        ],
      ),
    );
  }
}

/// The countdown line — neutral while live, an honest closed-window message once
/// expired. A `liveRegion` so a screen reader announces the expiry flip.
class _DepositCountdown extends StatelessWidget {
  const _DepositCountdown({required this.remaining});

  final int remaining;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final expired = remaining <= 0;

    return Semantics(
      liveRegion: true,
      container: true,
      // COARSE-granular label (#364 F9, quote-countdown parity): the
      // per-second text inside a live region was a 1 Hz screen-reader
      // announcement storm. This label changes at most once a minute, once at
      // the sub-minute flip, and once at expiry.
      label: expired
          ? l10n.walletSwapDepositExpired
          : l10n.walletSwapDepositExpiresIn(
              remaining >= 60
                  ? localizedCountdown(l10n, remaining)
                  : l10n.walletCountdownUnderMinute,
            ),
      excludeSemantics: true,
      child: WalletNotice(
        tone: expired ? WalletNoticeTone.warning : WalletNoticeTone.info,
        glyph: expired ? WalletGlyph.timerExpired : WalletGlyph.timer,
        message: expired
            ? l10n.walletSwapDepositExpired
            : l10n.walletSwapDepositExpiresIn(
                localizedCountdown(l10n, remaining),
              ),
      ),
    );
  }
}

/// Format a remaining-seconds count as a compact LOCALIZED countdown: the
/// hour form past an hour, minute words at minute magnitudes, second words
/// under a minute (zero at/under zero). Pure given an l10n instance.
///
/// Unit words over `mm:ss` at minute magnitudes (#364 N3): "Quote valid for
/// about 15:00" misreads as a CLOCK TIME (three o'clock) in 24h-clock locales
/// — a duration on a money deadline must never be mistakable for a
/// time-of-day. Minutes are floored (never overstate the remaining window;
/// the SDK gates the real deadline regardless). The unit forms live in the
/// ARB (the first cut hardcoded Latin "min"/"s" into all 16
/// locales' SPOKEN countdown labels — ar/he/ja/zh users heard English
/// mid-sentence at every minute boundary of a money deadline; translators
/// now own the unit rendering per locale).
String localizedCountdown(WalletLocalizations l10n, int seconds) {
  if (seconds <= 0) return l10n.walletCountdownSeconds(0);
  final h = seconds ~/ 3600;
  final m = (seconds % 3600) ~/ 60;
  if (h > 0) {
    return l10n.walletCountdownHoursMinutes(h, m.toString().padLeft(2, '0'));
  }
  if (m > 0) return l10n.walletCountdownMinutes(m);
  return l10n.walletCountdownSeconds(seconds);
}

/// The REQUIRED deposit-memo section — rendered only when the provider quote
/// carries a `depositMemo` (a destination tag / memo the source chain needs, e.g.
/// XRP/Cosmos). Framed in the warning colour (the same orange as an expired
/// window) because omitting it can lose funds; the memo itself is a monospace
/// `SelectableText` (what-you-see-is-what-you-copy) with a copy button. §5.4: the
/// memo is a NEVER-log value — display + copy only, never logged.
class _DepositMemo extends StatelessWidget {
  const _DepositMemo({required this.memo});

  final String memo;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;

    // ONE binding for the memo copy — read by the button's `onPressed` and by
    // the semantics action below.
    Future<void> copyMemo() async {
      await Clipboard.setData(ClipboardData(text: memo));
      unawaited(HapticFeedback.lightImpact());
      if (!context.mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.walletSwapDepositMemoCopied)));
    }

    // A card-notice in the DANGER tone (stage S11 C2): a missing memo loses
    // the funds. The memo VALUE is never the notice's `message` (localized copy
    // only): it rides in `child`, monospace and selectable, with its label and
    // its own Copy button.
    return WalletNotice.card(
      tone: WalletNoticeTone.danger,
      glyph: WalletGlyph.warning,
      title: l10n.walletSwapDepositMemoRequired,
      message: l10n.walletSwapDepositMemoWarning,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            l10n.walletSwapDepositMemoLabel,
            style: textTheme.labelMedium?.copyWith(color: colors.textMuted),
          ),
          const SizedBox(height: 4),
          SelectableText(
            memo,
            key: const Key('swap-deposit-memo'),
            style: WalletTypography.of(context).monoOn(
              (textTheme.bodyLarge ?? const TextStyle()).copyWith(
                letterSpacing: 0.2,
              ),
            ),
          ),
          const SizedBox(height: 12),
          // `container: true` + excludeSemantics so iOS VoiceOver doesn't merge
          // the copy button into the memo text above (flutter-patterns § iOS merge).
          Semantics(
            container: true,
            button: true,
            excludeSemantics: true,
            label: l10n.walletSwapDepositMemoCopy,
            // `excludeSemantics` drops the child subtree and the button's own
            // tap action with it, so the action must live on THIS node. On
            // the chains that require it, a deposit without this memo is
            // lost funds (Relim `0c5ae1bd`, #661/#721).
            onTap: () => unawaited(copyMemo()),
            child: OutlinedButton.icon(
              icon: const WalletIcon(WalletGlyph.copy),
              label: Text(l10n.walletSwapDepositMemoCopy),
              onPressed: () => unawaited(copyMemo()),
            ),
          ),
        ],
      ),
    );
  }
}
