import 'package:zec_wallet/zec_wallet.dart';

/// The swap display model + the reference-app slippage host-policy (spec §3.5;
/// ADR-0014/0525). The foreign-asset MENU is no longer a static curated list — it
/// is the live `/v0/tokens` registry surfaced by the shared token picker
/// (`swap_token_picker.dart` / `swapTokensProvider`) for BOTH directions (IntoZec
/// source + OutOfZec target). Like [buildWalletConfig], the slippage is a HOST
/// decision — the SDK is policy-free (it validates a [QuoteRequest] and enforces
/// the slippage ceiling; it does not choose the menu). A production host would
/// derive its menu from its own signed config; this file governs ONLY this
/// example app.

/// One swap asset — a provider-namespaced pair plus its display label, used for
/// the SOURCE (IntoZec) or TARGET (OutOfZec) of a swap. `chain`/`symbol` are the
/// exact `AssetId` strings the provider expects; a wrong pair is the SDK's typed
/// `RequestInvalid`/provider error, never a silent mis-swap
/// (validate-at-the-boundary, design invariant 7).
class SwapAsset {
  const SwapAsset({
    required this.chain,
    required this.symbol,
    required this.label,
  });

  /// Provider chain id (e.g. `"eth"`).
  final String chain;

  /// Provider asset symbol (e.g. `"usdc"`).
  final String symbol;

  /// Human display label (e.g. `"USDC on Ethereum"`).
  final String label;

  AssetId toAssetId() => AssetId(chain: chain, symbol: symbol);

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is SwapAsset &&
          chain == other.chain &&
          symbol == other.symbol &&
          label == other.label;

  @override
  int get hashCode => Object.hash(chain, symbol, label);
}

/// Slippage tolerance the reference app requests, in basis points. The SDK's own
/// default is 2% (200 bps); a request above the SDK's hard ceiling is rejected
/// typed (`SlippageToleranceTooHigh`), so this stays well under it. Exposing a
/// user-tunable slider is a documented follow-on — a wider window is a drain
/// surface, not a casual preference (spec §2.6 M1), so the safe default ships
/// first.
const int referenceSwapSlippageBps = 200;
