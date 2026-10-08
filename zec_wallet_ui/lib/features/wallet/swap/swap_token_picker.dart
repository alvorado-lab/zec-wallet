import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:zec_wallet/zec_wallet.dart';

import '../../../core/theme/colors.dart';
import '../../../core/theme/icons.dart';
import '../../../core/theme/shapes.dart';
import 'package:zec_wallet_ui/l10n/wallet_localizations.dart';
import '../../../shared/wallet_loading.dart';
import '../../../shared/wallet_notice.dart';
import '../../../shared/wallet_sheet.dart';
import 'swap_chains.dart';
import 'swap_token_icon.dart';
import 'swap_tokens_provider.dart';

/// Present the shared D5 token picker (spec §3.3b D5) and resolve to the chosen
/// [SwapToken], or `null` if dismissed. Used by BOTH directions — the IntoZec
/// SOURCE picker and the OutOfZec TARGET picker (symmetric since an earlier revision). A modal
/// bottom sheet over the shared [swapTokensProvider] — the fetch fires when this
/// opens (lazy; §5.2).
Future<SwapToken?> showSwapTokenPicker(
  BuildContext context, {
  required String title,
}) {
  // The shared sheet frame — which gives the picker the 560 desktop width cap
  // it lacked (stage S11 C4).
  return showWalletSheet<SwapToken>(
    context,
    builder: (_) => _SwapTokenPickerSheet(title: title),
  );
}

/// A display-ready label for a token: uppercased symbol + the HUMAN chain name,
/// e.g. "USDC on Ethereum" (matching the curated OutOfZec convention, not the raw
/// "USDC on ETH" code). Language-aware via [WalletLocalizations]; the symbol is a
/// provider identifier (not translatable), the chain is mapped to a friendly name.
String swapTokenLabel(WalletLocalizations l10n, String symbol, String chain) =>
    l10n.walletSwapTokenLabel(symbol.toUpperCase(), chainDisplayName(chain));

class _SwapTokenPickerSheet extends ConsumerStatefulWidget {
  const _SwapTokenPickerSheet({required this.title});

  /// Direction-specific sheet heading ("…to swap from" for IntoZec source,
  /// "…to receive" for OutOfZec target) — the shared sheet carries no direction
  /// knowledge, so the caller supplies the right framing.
  final String title;

  @override
  ConsumerState<_SwapTokenPickerSheet> createState() =>
      _SwapTokenPickerSheetState();
}

class _SwapTokenPickerSheetState extends ConsumerState<_SwapTokenPickerSheet> {
  final _searchController = TextEditingController();

  /// Lower-cased search query; filters by symbol OR chain. Empty ⇒ full list.
  String _query = '';

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final tokens = ref.watch(swapTokensProvider);

    return SafeArea(
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxHeight: MediaQuery.sizeOf(context).height * 0.7,
        ),
        // Resize for the keyboard so the search field stays visible while typing.
        child: Padding(
          padding: EdgeInsets.fromLTRB(
            16,
            0,
            16,
            16 + MediaQuery.viewInsetsOf(context).bottom,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              WalletSheetHeader(
                title: widget.title,
                closeKey: const Key('swap-token-picker-close'),
              ),
              const SizedBox(height: 12),
              // Search the 100s-long live list (maintainer UX: scrolling it is painful).
              TextField(
                key: const Key('swap-token-search'),
                controller: _searchController,
                autocorrect: false,
                enableSuggestions: false,
                textInputAction: TextInputAction.search,
                onChanged: (v) =>
                    setState(() => _query = v.trim().toLowerCase()),
                decoration: InputDecoration(
                  hintText: l10n.walletSwapPickerSearchHint,
                  prefixIcon: const WalletIcon(WalletGlyph.search),
                  isDense: true,
                ),
              ),
              const SizedBox(height: 12),
              Flexible(
                child: tokens.when(
                  data: (list) => _Body(list: list, query: _query),
                  loading: () => const _PickerCentered(
                    child: WalletLoadingIndicator(
                      key: Key('swap-token-picker-loading'),
                    ),
                  ),
                  // A Dart error is the no-cache first-fault (typed) — the SDK
                  // serves stale within its own timeout otherwise. Honest + a retry.
                  error: (_, _) => _PickerError(
                    onRetry: () => ref.invalidate(swapTokensProvider),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// The data body: the optional "couldn't refresh" banner (`fresh == false`, L6),
/// then the (search-filtered) list — or an honest empty / no-match state.
class _Body extends StatelessWidget {
  const _Body({required this.list, required this.query});

  final SwapTokenList list;
  final String query;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);

    if (list.tokens.isEmpty) {
      // Empty AND fresh ⇒ the provider genuinely has nothing right now ($0/null
      // price dropped everything); empty AND stale is the same honest dead-end —
      // never a blank picker (L6).
      return _PickerCentered(child: Text(l10n.walletSwapPickerEmpty));
    }

    final filtered = query.isEmpty
        ? list.tokens
        : list.tokens
              .where(
                (t) =>
                    t.symbol.toLowerCase().contains(query) ||
                    t.chain.toLowerCase().contains(query),
              )
              .toList();

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (!list.fresh) ...[const _StaleBanner(), const SizedBox(height: 8)],
        Flexible(
          child: filtered.isEmpty
              // The list is non-empty but nothing matches the query — an honest
              // "no match", distinct from the genuinely-empty list above.
              ? _PickerCentered(
                  child: Text(l10n.walletSwapPickerNoMatch(query)),
                )
              : ListView.builder(
                  shrinkWrap: true,
                  itemCount: filtered.length,
                  itemBuilder: (context, i) => _TokenRow(token: filtered[i]),
                ),
        ),
      ],
    );
  }
}

class _TokenRow extends StatelessWidget {
  const _TokenRow({required this.token});

  final SwapToken token;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final textTheme = Theme.of(context).textTheme;
    // ONE binding, read by the ListTile and by the semantics action below.
    void pick() => Navigator.of(context).pop(token);
    return Semantics(
      button: true,
      // Full "SYMBOL on CHAIN" for a screen reader (the visual drops the inline
      // "on CHAIN" — maintainer UX — but a11y still needs the chain).
      label: swapTokenLabel(l10n, token.symbol, token.chain),
      excludeSemantics: true,
      // …and `excludeSemantics` drops the ListTile's node, its tap action with
      // it, so the action must live on THIS node — every row of the picker is
      // otherwise announced and unpickable (Relim `0c5ae1bd`, #661/#721).
      onTap: pick,
      child: ListTile(
        contentPadding: EdgeInsets.zero,
        // Decorative monogram — excluded from semantics so a screen reader reads
        // the row label, not the placeholder initials (a11y fold).
        leading: ExcludeSemantics(
          child: SwapTokenIcon(symbol: token.symbol, chain: token.chain),
        ),
        // Symbol prominent; the chain is a small subtle chip, not "on NEAR" inline.
        title: Text(
          token.symbol.toUpperCase(),
          style: textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w600),
        ),
        trailing: _ChainChip(chain: token.chain),
        // Tapping returns the token to `showSwapTokenPicker`'s caller.
        onTap: pick,
      ),
    );
  }
}

/// A small, muted chain badge (e.g. `NEAR`) on a token row — keeps the chain
/// visible (it matters for cross-chain correctness) without the repeated
/// "on CHAIN" the maintainer asked to drop from each name.
class _ChainChip extends StatelessWidget {
  const _ChainChip({required this.chain});

  final String chain;

  @override
  Widget build(BuildContext context) {
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: colors.bgHover,
        borderRadius: BorderRadius.circular(WalletShapes.of(context).chip),
        border: Border.all(color: colors.border),
      ),
      child: Text(
        chainDisplayName(chain),
        style: textTheme.labelSmall?.copyWith(color: colors.textMuted),
      ),
    );
  }
}

/// The L6 "couldn't refresh, showing cached" banner — honest degradation, not an
/// error: the list below is the last good data, usable.
class _StaleBanner extends StatelessWidget {
  const _StaleBanner();

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    return WalletNotice(
      tone: WalletNoticeTone.warning,
      glyph: WalletGlyph.serviceUnreachable,
      message: l10n.walletSwapPickerStale,
    );
  }
}

class _PickerError extends StatelessWidget {
  const _PickerError({required this.onRetry});

  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final l10n = WalletLocalizations.of(context);
    final colors = WalletColors.of(context);
    final textTheme = Theme.of(context).textTheme;
    return _PickerCentered(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          WalletIcon(WalletGlyph.error, size: 32, color: colors.orange),
          const SizedBox(height: 12),
          Text(
            l10n.walletSwapPickerError,
            textAlign: TextAlign.center,
            style: textTheme.bodyMedium?.copyWith(color: colors.textMuted),
          ),
          const SizedBox(height: 16),
          OutlinedButton(
            onPressed: onRetry,
            child: Text(l10n.walletSwapPickerRetry),
          ),
        ],
      ),
    );
  }
}

class _PickerCentered extends StatelessWidget {
  const _PickerCentered({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 32),
      child: Center(child: child),
    );
  }
}
